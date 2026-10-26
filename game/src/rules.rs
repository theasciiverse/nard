//! The rules of backgammon: the position, legal plays, the cube and scoring.
//!
//! Each side counts the board from its own home: points 1-24, 25 its bar and
//! 0 its men borne off. A side's point p is the other side's point 25 - p.

use std::collections::HashMap;
use std::fmt::Write;

pub const OFF: usize = 0;
pub const BAR: usize = 25;
pub const MEN: u8 = 15;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Side {
    White,
    Black,
}

impl Side {
    pub fn other(self) -> Side {
        match self {
            Side::White => Side::Black,
            Side::Black => Side::White,
        }
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn name(self) -> &'static str {
        match self {
            Side::White => "White",
            Side::Black => "Black",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct Position {
    /// Men per point, each side in its own numbering.
    pub men: [[u8; 26]; 2],
    pub to_move: Side,
}

/// One man moved by one die.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Hop {
    pub from: usize,
    pub to: usize,
    pub die: u8,
    pub hit: bool,
}

/// A whole turn: every hop, in the order played.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Play {
    pub hops: Vec<Hop>,
}

impl Position {
    pub fn start() -> Position {
        let mut side = [0; 26];
        side[24] = 2;
        side[13] = 5;
        side[8] = 3;
        side[6] = 5;
        Position { men: [side, side], to_move: Side::White }
    }

    pub fn side(&self, side: Side) -> &[u8; 26] {
        &self.men[side.index()]
    }

    pub fn mover(&self) -> &[u8; 26] {
        self.side(self.to_move)
    }

    /// The opponent's men on the mover's point `p` (1-24).
    fn against(&self, p: usize) -> u8 {
        self.side(self.to_move.other())[25 - p]
    }

    pub fn pips(&self, side: Side) -> u32 {
        (1..=25).map(|p| p as u32 * self.side(side)[p] as u32).sum()
    }

    pub fn off(&self, side: Side) -> u8 {
        self.side(side)[OFF]
    }

    pub fn winner(&self) -> Option<Side> {
        [Side::White, Side::Black].into_iter().find(|&s| self.off(s) == MEN)
    }

    /// Where a man of the mover on `from` lands with `die`, if it may.
    pub fn hop(&self, from: usize, die: u8) -> Option<Hop> {
        let me = self.mover();
        if me[from] == 0 || from == OFF || (me[BAR] > 0 && from != BAR) {
            return None;
        }
        let to = from as i32 - die as i32;
        if to <= 0 {
            if me[7..=BAR].iter().any(|&n| n > 0) {
                return None;
            }
            if to < 0 && me[from + 1..=6].iter().any(|&n| n > 0) {
                return None;
            }
            return Some(Hop { from, to: OFF, die, hit: false });
        }
        let to = to as usize;
        match self.against(to) {
            0 => Some(Hop { from, to, die, hit: false }),
            1 => Some(Hop { from, to, die, hit: true }),
            _ => None,
        }
    }

    pub fn apply(&mut self, hop: Hop) {
        let (me, them) = (self.to_move.index(), self.to_move.other().index());
        self.men[me][hop.from] -= 1;
        self.men[me][hop.to] += 1;
        if hop.hit {
            self.men[them][25 - hop.to] -= 1;
            self.men[them][BAR] += 1;
        }
    }

    /// The position after `hops`, with the same side still to move.
    pub fn after(&self, hops: &[Hop]) -> Position {
        let mut pos = self.clone();
        for &hop in hops {
            pos.apply(hop);
        }
        pos
    }

    /// Plays the turn and passes the move to the other side.
    pub fn play(&mut self, play: &Play) {
        *self = self.after(&play.hops);
        self.to_move = self.to_move.other();
    }

    /// Every legal play of a throw, in every order its hops can be made.
    /// A play must use as many dice as it can; if only one die of two can
    /// be used, the higher whenever possible. No legal hop gives one empty play.
    pub fn plays(&self, dice: (u8, u8)) -> Vec<Play> {
        let dice: Vec<u8> = if dice.0 == dice.1 { vec![dice.0; 4] } else { vec![dice.0, dice.1] };
        let mut found = Vec::new();
        self.search(&dice, &mut Vec::new(), &mut found);
        let most = found.iter().map(|p: &Play| p.hops.len()).max().unwrap_or(0);
        found.retain(|p| p.hops.len() == most);
        if most == 1 && dice[0] != dice[1] {
            let high = dice[0].max(dice[1]);
            if found.iter().any(|p| p.hops[0].die == high) {
                found.retain(|p| p.hops[0].die == high);
            }
        }
        if found.is_empty() {
            found.push(Play::default());
        }
        found
    }

    fn search(&self, dice: &[u8], hops: &mut Vec<Hop>, found: &mut Vec<Play>) {
        let mut moved = false;
        for (i, &die) in dice.iter().enumerate() {
            if dice[..i].contains(&die) {
                continue;
            }
            let rest: Vec<u8> = dice.iter().enumerate().filter(|&(j, _)| j != i).map(|(_, &d)| d).collect();
            for from in 1..=BAR {
                if let Some(hop) = self.hop(from, die) {
                    moved = true;
                    let mut next = self.clone();
                    next.apply(hop);
                    hops.push(hop);
                    next.search(&rest, hops, found);
                    hops.pop();
                }
            }
        }
        if !moved && !hops.is_empty() {
            found.push(Play { hops: hops.clone() });
        }
    }
}

/// The plays of a throw that end in different positions, one spelling each.
pub fn distinct(pos: &Position, plays: &[Play]) -> Vec<Play> {
    let mut best: HashMap<Position, Play> = HashMap::new();
    let mut order = Vec::new();
    for play in plays {
        let end = pos.after(&play.hops);
        match best.get(&end) {
            Some(kept) if notation(kept).len() <= notation(play).len() => {}
            Some(_) => {
                best.insert(end, play.clone());
            }
            None => {
                order.push(end.clone());
                best.insert(end, play.clone());
            }
        }
    }
    order.into_iter().map(|end| best.remove(&end).unwrap()).collect()
}

fn point_name(p: usize) -> String {
    match p {
        BAR => "bar".into(),
        OFF => "off".into(),
        _ => p.to_string(),
    }
}

/// Standard notation, in the mover's numbering: `bar/22 13/7* 6/5(2)`.
/// A man moved twice is written once (`13/8`), its hits shown on the way
/// (`13/9*/8`).
pub fn notation(play: &Play) -> String {
    if play.hops.is_empty() {
        return "no move".into();
    }
    // Each run: the points a man passes through and whether it hit there.
    let mut runs: Vec<Vec<(usize, bool)>> = Vec::new();
    for hop in &play.hops {
        let joined = runs
            .iter_mut()
            .rev()
            .find(|r| r.last().is_some_and(|&(p, _)| p == hop.from && p != OFF));
        match joined {
            Some(run) => run.push((hop.to, hop.hit)),
            None => runs.push(vec![(hop.from, false), (hop.to, hop.hit)]),
        }
    }
    // A hit belongs to its point: `6/4*(2)`, not `6/4* 6/4`.
    let hit_on: Vec<usize> = play.hops.iter().filter(|h| h.hit).map(|h| h.to).collect();
    let mut parts: Vec<String> = runs
        .iter()
        .map(|run| {
            let mut s = point_name(run[0].0);
            let last = run.len() - 1;
            for (i, &(p, _)) in run.iter().enumerate().skip(1) {
                let hit = hit_on.contains(&p);
                // A stop on the way is only worth naming where it hit.
                if i == last || hit {
                    write!(s, "/{}", point_name(p)).unwrap();
                    if hit {
                        s.push('*');
                    }
                }
            }
            s
        })
        .collect();
    parts.sort_by_key(|s| {
        let from = s.split('/').next().unwrap_or("");
        std::cmp::Reverse(if from == "bar" { 25 } else { from.parse().unwrap_or(0) })
    });
    let mut out: Vec<String> = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let n = parts[i..].iter().take_while(|p| **p == parts[i]).count();
        out.push(if n > 1 { format!("{}({n})", parts[i]) } else { parts[i].clone() });
        i += n;
    }
    out.join(" ")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cube {
    pub value: u32,
    pub owner: Option<Side>,
}

impl Default for Cube {
    fn default() -> Self {
        Cube { value: 1, owner: None }
    }
}

impl Cube {
    pub fn may_double(&self, side: Side) -> bool {
        self.owner.is_none_or(|o| o == side) && self.value < 64
    }

    pub fn take(&mut self, taker: Side) {
        self.value *= 2;
        self.owner = Some(taker);
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Single = 1,
    Gammon = 2,
    Backgammon = 3,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Single => "a single game",
            Kind::Gammon => "a gammon",
            Kind::Backgammon => "a backgammon",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub winner: Side,
    pub kind: Kind,
    pub points: u32,
    /// The loser passed a double rather than play on.
    pub passed: bool,
}

/// How a finished game scores, or `None` while it goes on.
pub fn outcome(pos: &Position, cube: Cube) -> Option<Outcome> {
    let winner = pos.winner()?;
    let loser = pos.side(winner.other());
    let kind = if loser[OFF] > 0 {
        Kind::Single
    } else if loser[BAR] > 0 || loser[19..=24].iter().any(|&n| n > 0) {
        Kind::Backgammon
    } else {
        Kind::Gammon
    };
    Some(Outcome { winner, kind, points: kind as u32 * cube.value, passed: false })
}

/// The board as bgsage sees it, from the side on roll.
pub fn to_engine(pos: &Position) -> bgsage::Board {
    let (me, them) = (pos.mover(), pos.side(pos.to_move.other()));
    let mut board = [0; 26];
    for p in 1..=24 {
        board[p] = me[p] as i32 - them[25 - p] as i32;
    }
    board[25] = me[BAR] as i32;
    board[0] = them[BAR] as i32;
    board
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A position from the mover's view: (point, men), negative for theirs.
    fn position(points: &[(usize, i32)]) -> Position {
        let mut men = [[0u8; 26]; 2];
        for &(p, n) in points {
            if n > 0 {
                men[0][p] = n as u8;
            } else {
                men[1][if p == 0 { BAR } else { 25 - p }] = (-n) as u8;
            }
        }
        for side in &mut men {
            side[OFF] = MEN - side[1..].iter().sum::<u8>();
        }
        Position { men, to_move: Side::White }
    }

    fn spellings(pos: &Position, dice: (u8, u8)) -> Vec<String> {
        let mut v: Vec<String> = distinct(pos, &pos.plays(dice)).iter().map(notation).collect();
        v.sort();
        v
    }

    #[test]
    fn opening_three_one() {
        let pos = Position::start();
        let s = spellings(&pos, (3, 1));
        assert!(s.contains(&"8/5 6/5".to_string()), "{s:?}");
        assert!(s.contains(&"24/20".to_string()), "{s:?}");
        assert!(s.contains(&"13/9".to_string()), "{s:?}");
    }

    #[test]
    fn doubles_play_four() {
        let pos = Position::start();
        let plays = pos.plays((6, 6));
        assert!(plays.iter().all(|p| p.hops.len() == 4));
        assert!(spellings(&pos, (6, 6)).contains(&"24/18(2) 13/7(2)".to_string()));
    }

    #[test]
    fn bar_first_and_closed_board() {
        // On the bar against a five-point board (our 19-23 closed): only a 1 enters.
        let pos = position(&[(BAR, 1), (6, 14), (19, -2), (20, -2), (21, -2), (22, -2), (23, -2), (1, -5)]);
        assert_eq!(spellings(&pos, (2, 3)), vec!["no move"]);
        let s = spellings(&pos, (6, 1));
        assert_eq!(s, vec!["bar/18"], "enters with the 1, then plays the 6 on");
    }

    #[test]
    fn higher_die_when_only_one_can_be_played() {
        // One man on 13; a 6 to 7 is open, a 5 to 8 is open, but after either the other die is blocked.
        let pos = position(&[(13, 1), (6, 14), (2, -2), (1, -2), (3, -2), (4, -2), (5, -2), (12, -2), (11, -3)]);
        let plays = pos.plays((6, 5));
        assert!(plays.iter().all(|p| p.hops.len() == 1 && p.hops[0].die == 6), "{plays:?}");
    }

    #[test]
    fn bearing_off_with_higher_dice() {
        let pos = position(&[(4, 1), (2, 1), (24, -15)]);
        assert_eq!(spellings(&pos, (6, 5)), vec!["4/off 2/off"]);
        // A 6 with a man still on the 5 point must move inside, not bear off from 4.
        let pos = position(&[(5, 1), (4, 1), (24, -15)]);
        let s = spellings(&pos, (6, 1));
        assert!(s.contains(&"5/off 4/3".to_string()) || s.contains(&"5/4 4/off".to_string()), "{s:?}");
        assert!(pos.hop(4, 6).is_none());
    }

    #[test]
    fn notation_joins_and_hits() {
        let play = Play {
            hops: vec![
                Hop { from: 13, to: 9, die: 4, hit: true },
                Hop { from: 9, to: 8, die: 1, hit: false },
            ],
        };
        assert_eq!(notation(&play), "13/9*/8");
        let play = Play {
            hops: vec![
                Hop { from: 6, to: 4, die: 2, hit: true },
                Hop { from: 6, to: 4, die: 2, hit: false },
            ],
        };
        assert_eq!(notation(&play), "6/4*(2)");
    }

    #[test]
    fn gammons_and_backgammons() {
        // Their men on our 3 point are in our home board.
        let mut pos = position(&[(1, 1), (3, -15)]);
        pos.men[0][OFF] = 15;
        pos.men[0][1] = 0;
        let o = outcome(&pos, Cube { value: 2, owner: None }).unwrap();
        assert_eq!((o.kind, o.points), (Kind::Backgammon, 6));
        let mut pos = position(&[(1, 1), (13, -15)]);
        pos.men[0][OFF] = 15;
        pos.men[0][1] = 0;
        assert_eq!(outcome(&pos, Cube::default()).unwrap().kind, Kind::Gammon);
    }

    /// Our distinct plays must end on exactly the boards bgsage generates,
    /// over thousands of positions from random games.
    #[test]
    fn same_plays_as_bgsage() {
        use rand::{Rng, SeedableRng};
        use std::collections::HashSet;
        let assets = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../vendor/bgsage");
        let mut engine = bgsage::Engine::load(&assets).unwrap();
        let mut rng = rand_chacha::ChaCha8Rng::seed_from_u64(7);
        let mut checked = 0;
        for _ in 0..60 {
            let mut pos = Position::start();
            while pos.winner().is_none() {
                let dice = (rng.random_range(1..=6), rng.random_range(1..=6));
                let ours = distinct(&pos, &pos.plays(dice));
                let mine: HashSet<bgsage::Board> = ours.iter().map(|p| to_engine(&pos.after(&p.hops))).collect();
                assert_eq!(mine.len(), ours.len());
                let theirs: HashSet<bgsage::Board> = engine
                    .plays(&to_engine(&pos), dice, 1, bgsage::Cube::default())
                    .unwrap()
                    .into_iter()
                    .map(|p| p.board)
                    .collect();
                assert_eq!(mine, theirs, "{pos:?} {dice:?}");
                checked += 1;
                let play = ours[rng.random_range(0..ours.len())].clone();
                pos.play(&play);
            }
        }
        assert!(checked > 3000, "{checked}");
    }
}
