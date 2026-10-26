//! The computer player: bgsage chooses, our rules spell the play.

use rand::Rng;

use crate::rules::{self, Cube, Play, Position, Side};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Strength {
    Easy,
    Normal,
    Hard,
}

impl Strength {
    pub fn name(self) -> &'static str {
        match self {
            Strength::Easy => "easy",
            Strength::Normal => "normal",
            Strength::Hard => "hard",
        }
    }

    fn ply(self) -> u8 {
        if self == Strength::Hard { 2 } else { 1 }
    }
}

/// How far below the best play an easy computer may stray, in equity.
const EASY_SLACK: f64 = 0.12;

pub struct Computer {
    engine: bgsage::Engine,
}

/// The cube as bgsage sees it, from the side on roll.
fn engine_cube(cube: Cube, on_roll: Side) -> bgsage::Cube {
    let owner = match cube.owner {
        None => bgsage::CubeOwner::Centred,
        Some(s) if s == on_roll => bgsage::CubeOwner::Player,
        Some(_) => bgsage::CubeOwner::Opponent,
    };
    bgsage::Cube { value: cube.value as i32, owner, jacoby: false }
}

impl Computer {
    pub fn load() -> Result<Computer, String> {
        let dir = crate::assets::dir().map_err(|e| format!("engine files: {e}"))?;
        let engine = bgsage::Engine::load(&dir).map_err(|e| e.to_string())?;
        Ok(Computer { engine })
    }

    /// Picks one of `plays` (the distinct plays of `dice` in `pos`).
    pub fn choose(&mut self, pos: &Position, dice: (u8, u8), cube: Cube, strength: Strength,
                  plays: &[Play], rng: &mut impl Rng) -> Play {
        if plays.len() == 1 {
            return plays[0].clone();
        }
        let ranked = match self.engine.plays(&rules::to_engine(pos), dice, strength.ply(),
                                             engine_cube(cube, pos.to_move)) {
            Ok(r) => r,
            Err(e) => {
                log::error!("{e}; playing the first legal play");
                return plays[0].clone();
            }
        };
        let pick = if strength == Strength::Easy {
            let near = ranked.iter().take_while(|p| p.equity >= ranked[0].equity - EASY_SLACK).count();
            &ranked[rng.random_range(0..near)]
        } else {
            &ranked[0]
        };
        match plays.iter().find(|p| rules::to_engine(&pos.after(&p.hops)) == pick.board) {
            Some(play) => play.clone(),
            None => {
                log::error!("bgsage chose a board our rules cannot reach: {:?}", pick.board);
                debug_assert!(false, "unreachable engine board");
                plays[0].clone()
            }
        }
    }

    /// Whether the side on roll should double now.
    pub fn doubles(&mut self, pos: &Position, cube: Cube, strength: Strength) -> bool {
        if strength == Strength::Easy || !cube.may_double(pos.to_move) {
            return false;
        }
        self.decide(pos, cube, strength).is_some_and(|d| d.should_double)
    }

    /// Whether to take when the side on roll in `pos` doubles.
    pub fn takes(&mut self, pos: &Position, cube: Cube, strength: Strength) -> bool {
        self.decide(pos, cube, strength).is_none_or(|d| d.should_take)
    }

    fn decide(&mut self, pos: &Position, cube: Cube, strength: Strength) -> Option<bgsage::CubeDecision> {
        let d = self.engine.cube_decision(&rules::to_engine(pos), strength.ply(), engine_cube(cube, pos.to_move));
        match d {
            Ok(d) => {
                log::debug!("cube: {} (ND {:+.3} D/T {:+.3})", d.optimal_action, d.equity_nd, d.equity_dt);
                Some(d)
            }
            Err(e) => {
                log::error!("{e}");
                None
            }
        }
    }
}
