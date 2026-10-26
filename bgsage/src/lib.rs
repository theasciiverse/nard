//! bgsage's production model ("stage11") behind a safe handle.
//!
//! Boards are bgsage's: seen from the side on roll, index 1-24 the points
//! numbered from that side's home board, positive for its men and negative
//! for the opponent's, index 25 its bar and index 0 the opponent's bar.

use std::ffi::{CStr, CString, c_char, c_int};
use std::fmt;
use std::marker::PhantomData;
use std::path::Path;
use std::ptr::NonNull;

use bgsage_sys as sys;
use serde::Deserialize;

pub type Board = [i32; 26];

pub const STARTING_BOARD: Board = [
    0, -2, 0, 0, 0, 0, 5, 0, 3, 0, 0, 0, -5, 5, 0, 0, 0, -3, 0, -5, 0, 0, 0, 0, 2, 0,
];

/// Stage 11's networks in the order bgsage's `python/bgsage/weights.py`
/// passes them: the 17 Stage 9 pair networks (prim_prim, anch_prim and
/// anch_anch share prim_anch's), then the 7 Stage 11 back-game networks.
const WEIGHTS: [&str; 24] = [
    "sl_s9_purerace", "sl_s9_race_race", "sl_s9_race_att", "sl_s9_race_prim",
    "sl_s9_race_anch", "sl_s9_att_race", "sl_s9_att_att", "sl_s9_att_prim",
    "sl_s9_att_anch", "sl_s9_prim_race", "sl_s9_prim_att", "sl_s9_prim_anch",
    "sl_s9_prim_anch", "sl_s9_anch_race", "sl_s9_anch_att", "sl_s9_prim_anch",
    "sl_s9_prim_anch", "sl_s11_bg_deep", "sl_s11_bg_middle", "sl_s11_bg_double",
    "sl_s11_bg_p3", "sl_s11_bg_containment", "sl_s11_bg_snake", "sl_s11_bg_massive",
];
const STRATEGY: &str = "backgame_pair_phased";
const BEAROFF: &str = "data/bearoff_1sided.db";

#[derive(Debug)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "bgsage: {}", self.0)
    }
}

impl std::error::Error for Error {}

fn last_error() -> Error {
    // SAFETY: bgsage returns a thread-local, NUL-terminated string.
    let text = unsafe { CStr::from_ptr(sys::bgsage_mobile_last_error()) };
    Error(text.to_string_lossy().into_owned())
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CubeOwner {
    Centred,
    Player,
    Opponent,
}

/// The cube in money play, from the side on roll.
#[derive(Clone, Copy, Debug)]
pub struct Cube {
    pub value: i32,
    pub owner: CubeOwner,
    pub jacoby: bool,
}

impl Default for Cube {
    fn default() -> Self {
        Cube { value: 1, owner: CubeOwner::Centred, jacoby: false }
    }
}

/// One legal play, as the board it leaves (still from the mover's side).
#[derive(Clone, Debug, Deserialize)]
pub struct Play {
    pub board: Board,
    /// Win, win gammon, win backgammon, lose gammon, lose backgammon.
    pub probs: [f64; 5],
    pub equity: f64,
    pub cubeless_equity: f64,
    pub equity_diff: f64,
}

#[derive(Deserialize)]
struct Plays {
    moves: Vec<Play>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CubeDecision {
    pub probs: [f64; 5],
    pub cubeless_equity: f64,
    pub equity_nd: f64,
    pub equity_dt: f64,
    pub equity_dp: f64,
    pub should_double: bool,
    pub should_take: bool,
    pub optimal_action: String,
}

/// The engine. Loading reads about 14 MB of networks and bearoff data.
/// Calls take `&mut self` because bgsage requires them to be serialised.
pub struct Engine {
    raw: NonNull<sys::BgsageMobileEngine>,
    _not_sync: PhantomData<*mut ()>,
}

// SAFETY: the handle has no thread affinity; `&mut self` serialises calls.
unsafe impl Send for Engine {}

impl Engine {
    /// Loads Stage 11 from a directory holding `models/` and `data/`.
    pub fn load(assets: &Path) -> Result<Engine, Error> {
        let cstring = |p: &Path| {
            CString::new(p.to_string_lossy().into_owned()).map_err(|e| Error(e.to_string()))
        };
        let paths = WEIGHTS
            .iter()
            .map(|w| cstring(&assets.join("models").join(format!("{w}.weights.best"))))
            .collect::<Result<Vec<_>, _>>()?;
        let pointers: Vec<*const c_char> = paths.iter().map(|p| p.as_ptr()).collect();
        let mut hidden = [400 as c_int; WEIGHTS.len()];
        hidden[0] = 100;
        let strategy = CString::new(STRATEGY).unwrap();
        let bearoff = cstring(&assets.join(BEAROFF))?;
        // SAFETY: every pointer is valid for the call; bgsage copies the strings.
        let raw = unsafe {
            sys::bgsage_mobile_create(
                strategy.as_ptr(),
                pointers.as_ptr(),
                hidden.as_ptr(),
                WEIGHTS.len() as c_int,
                bearoff.as_ptr(),
            )
        };
        NonNull::new(raw).map(|raw| Engine { raw, _not_sync: PhantomData }).ok_or_else(last_error)
    }

    /// Every legal play of `dice`, best first, searched `ply` deep (1 or 2).
    pub fn plays(&mut self, board: &Board, dice: (u8, u8), ply: u8, cube: Cube) -> Result<Vec<Play>, Error> {
        let mut request = request(board, ply, cube);
        request.die1 = dice.0.into();
        request.die2 = dice.1.into();
        let plays: Plays = self.analyze(sys::OP_CHECKER, &request)?;
        Ok(plays.moves)
    }

    /// Whether the side on roll should double, and whether the other should take.
    pub fn cube_decision(&mut self, board: &Board, ply: u8, cube: Cube) -> Result<CubeDecision, Error> {
        self.analyze(sys::OP_CUBE, &request(board, ply, cube))
    }

    fn analyze<T: for<'a> Deserialize<'a>>(&mut self, op: c_int, request: &sys::BgsageMobileRequest) -> Result<T, Error> {
        // SAFETY: the handle is live and `request` outlives the call.
        let text = unsafe { sys::bgsage_mobile_analyze(self.raw.as_ptr(), op, request) };
        if text.is_null() {
            return Err(last_error());
        }
        // SAFETY: a non-null result is a NUL-terminated string we now own.
        let parsed = serde_json::from_slice(unsafe { CStr::from_ptr(text) }.to_bytes());
        unsafe { sys::bgsage_mobile_free(text) };
        parsed.map_err(|e| Error(e.to_string()))
    }
}

impl Drop for Engine {
    fn drop(&mut self) {
        // SAFETY: created by bgsage_mobile_create and dropped once.
        unsafe { sys::bgsage_mobile_destroy(self.raw.as_ptr()) }
    }
}

fn request(board: &Board, ply: u8, cube: Cube) -> sys::BgsageMobileRequest {
    sys::BgsageMobileRequest {
        board: *board,
        ply: ply.into(),
        cube_value: cube.value,
        cube_owner: match cube.owner {
            CubeOwner::Centred => 0,
            CubeOwner::Player => 1,
            CubeOwner::Opponent => 2,
        },
        jacoby: cube.jacoby.into(),
        budget_ms: 1000,
        ..Default::default()
    }
}
