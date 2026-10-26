//! The game in play: whose turn, the dice, the cube, and a person's play
//! built hop by hop. Driven by keys, clicks and `tick(now)`.

use std::time::{Duration, Instant};

use rand::Rng;
use rand_chacha::ChaCha8Rng;

use crate::ai::{Computer, Strength};
use crate::rules::{self, BAR, Cube, Hop, Kind, OFF, Outcome, Play, Position, Side};

const OPENING: Duration = Duration::from_millis(1300);
const THROW: Duration = Duration::from_millis(600);
const THINK: Duration = Duration::from_millis(500);
const ANSWER: Duration = Duration::from_millis(900);
const HOP: Duration = Duration::from_millis(420);
const NO_MOVE: Duration = Duration::from_millis(1500);
const DEMO_AGAIN: Duration = Duration::from_secs(4);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Player {
    Human,
    Computer(Strength),
}

/// A place on the board as drawn: a point in the bottom side's numbering,
/// the bar or the bear-off tray.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Spot {
    Point(usize),
    Bar,
    Off,
}

#[derive(Clone, Debug)]
pub enum Phase {
    /// Each side throws one die; the higher plays both.
    Opening { dice: [u8; 2], until: Instant },
    /// The side to move may double, then throws.
    ToRoll { since: Instant },
    /// The side to move doubled; the other takes or passes.
    Offered { since: Instant },
    Throwing { until: Instant },
    /// A person builds the play.
    Choosing,
    /// A person's play is complete, waiting to be confirmed.
    Done,
    /// The computer's play, shown hop by hop.
    Showing { play: Play, shown: usize, next: Instant },
    NoMove { until: Instant },
    Over { since: Instant },
}

pub struct Turn {
    pub side: Side,
    pub dice: (u8, u8),
    pub play: String,
}

pub struct App {
    pub seats: [Player; 2],
    /// The side drawn at the bottom of the board.
    pub view: Side,
    pub pos: Position,
    pub cube: Cube,
    pub dice: Option<(u8, u8)>,
    /// Every legal play of the dice, in every hop order.
    pub plays: Vec<Play>,
    pub partial: Vec<Hop>,
    /// A picked man, by the mover's point.
    pub picked: Option<usize>,
    pub phase: Phase,
    pub turn: u32,
    pub history: Vec<Turn>,
    pub score: [u32; 2],
    pub message: String,
    pub demo: bool,
    rng: ChaCha8Rng,
    computer: Option<Computer>,
}

impl App {
    pub fn new(seats: [Player; 2], rng: ChaCha8Rng, computer: Option<Computer>, demo: bool, now: Instant) -> App {
        let view = if seats[0] == Player::Human || seats[1] != Player::Human { Side::White } else { Side::Black };
        let mut app = App {
            seats,
            view,
            pos: Position::start(),
            cube: Cube::default(),
            dice: None,
            plays: Vec::new(),
            partial: Vec::new(),
            picked: None,
            phase: Phase::Choosing,
            turn: 0,
            history: Vec::new(),
            score: [0, 0],
            message: String::new(),
            demo,
            rng,
            computer,
        };
        app.new_game(now);
        app
    }

    pub fn into_computer(self) -> Option<Computer> {
        self.computer
    }

    pub fn player(&self, side: Side) -> Player {
        self.seats[side.index()]
    }

    pub fn name(&self, side: Side) -> String {
        let humans = self.seats.iter().filter(|&&p| p == Player::Human).count();
        match self.player(side) {
            Player::Human if humans == 1 => "You".into(),
            Player::Human => side.name().into(),
            Player::Computer(s) if humans == 0 => format!("{} ({})", side.name(), s.name()),
            Player::Computer(s) => format!("Sage ({})", s.name()),
        }
    }

    /// Whether a person is to act now (and the keys for play apply).
    pub fn human_to_act(&self) -> bool {
        let side = match self.phase {
            Phase::Offered { .. } => self.pos.to_move.other(),
            _ => self.pos.to_move,
        };
        self.player(side) == Player::Human
    }

    fn die(&mut self) -> u8 {
        self.rng.random_range(1..=6)
    }

    pub fn new_game(&mut self, now: Instant) {
        self.pos = Position::start();
        self.cube = Cube::default();
        self.dice = None;
        self.plays.clear();
        self.partial.clear();
        self.picked = None;
        self.turn = 0;
        self.history.clear();
        let dice = [self.die(), self.die()];
        self.message = "Each side throws one die; the higher plays first".into();
        self.phase = Phase::Opening { dice, until: now + OPENING };
    }

    /// The position as it should be drawn, with the hops made so far.
    pub fn shown(&self) -> Position {
        match &self.phase {
            Phase::Showing { play, shown, .. } => self.pos.after(&play.hops[..*shown]),
            _ => self.pos.after(&self.partial),
        }
    }

    pub fn tick(&mut self, now: Instant) {
        match self.phase.clone() {
            Phase::Opening { dice, until } if now >= until => {
                if dice[0] == dice[1] {
                    self.message = format!("Both threw {}: again", dice[0]);
                    let dice = [self.die(), self.die()];
                    self.phase = Phase::Opening { dice, until: now + OPENING };
                } else {
                    let first = if dice[0] > dice[1] { Side::White } else { Side::Black };
                    self.pos.to_move = first;
                    self.turn = 1;
                    self.message = format!("{} threw higher and plays first", self.name(first));
                    self.dice = Some((dice[0].max(dice[1]), dice[0].min(dice[1])));
                    self.begin_play(now);
                }
            }
            Phase::ToRoll { since } if now >= since + THINK => {
                if let Player::Computer(s) = self.player(self.pos.to_move) {
                    let doubles = match self.computer.as_mut() {
                        Some(c) => c.doubles(&self.pos, self.cube, s),
                        None => false,
                    };
                    if doubles { self.double(now) } else { self.throw(now) }
                }
            }
            Phase::Offered { since } if now >= since + ANSWER => {
                let taker = self.pos.to_move.other();
                if let Player::Computer(s) = self.player(taker) {
                    let takes = match self.computer.as_mut() {
                        Some(c) => c.takes(&self.pos, self.cube, s),
                        None => true,
                    };
                    self.answer(takes, now);
                }
            }
            Phase::Throwing { until } if now >= until => self.begin_play(now),
            Phase::Showing { play, shown, next } if now >= next => {
                if shown < play.hops.len() {
                    self.phase = Phase::Showing { play, shown: shown + 1, next: now + HOP };
                } else {
                    self.commit(play, now);
                }
            }
            Phase::NoMove { until } if now >= until => self.next_turn(now),
            Phase::Over { since, .. } if self.demo && now >= since + DEMO_AGAIN => self.new_game(now),
            _ => {}
        }
    }

    fn begin_play(&mut self, now: Instant) {
        let dice = self.dice.expect("dice thrown");
        let side = self.pos.to_move;
        self.plays = self.pos.plays(dice);
        self.partial.clear();
        self.picked = None;
        if self.plays[0].hops.is_empty() {
            self.message = format!("{} cannot move", self.name(side));
            self.history.push(Turn { side, dice, play: "no move".into() });
            self.phase = Phase::NoMove { until: now + NO_MOVE };
            return;
        }
        match self.player(side) {
            Player::Human => self.phase = Phase::Choosing,
            Player::Computer(strength) => {
                let distinct = rules::distinct(&self.pos, &self.plays);
                let play = match self.computer.as_mut() {
                    Some(c) => c.choose(&self.pos, dice, self.cube, strength, &distinct, &mut self.rng),
                    None => distinct[0].clone(),
                };
                self.phase = Phase::Showing { play, shown: 0, next: now + HOP };
            }
        }
    }

    fn commit(&mut self, play: Play, now: Instant) {
        let side = self.pos.to_move;
        let dice = self.dice.expect("dice thrown");
        self.history.push(Turn { side, dice, play: rules::notation(&play) });
        self.pos.play(&play);
        self.partial.clear();
        self.picked = None;
        match rules::outcome(&self.pos, self.cube) {
            Some(outcome) => self.finish(outcome, now),
            None => self.next_turn(now),
        }
    }

    fn next_turn(&mut self, now: Instant) {
        if matches!(self.phase, Phase::NoMove { .. }) {
            self.pos.to_move = self.pos.to_move.other();
        }
        self.turn += 1;
        self.dice = None;
        self.plays.clear();
        self.phase = Phase::ToRoll { since: now };
        if self.player(self.pos.to_move) == Player::Human {
            self.message = self.roll_prompt();
        }
    }

    fn roll_prompt(&self) -> String {
        let side = self.pos.to_move;
        let who = self.name(side);
        if self.cube.may_double(side) {
            format!("{who}: throw the dice, or double to {}", self.cube.value * 2)
        } else {
            format!("{who}: throw the dice")
        }
    }

    fn finish(&mut self, outcome: Outcome, now: Instant) {
        self.score[outcome.winner.index()] += outcome.points;
        let won = if outcome.passed {
            "the double was passed".to_string()
        } else {
            outcome.kind.name().to_string()
        };
        let points = if outcome.points == 1 { "1 point".into() } else { format!("{} points", outcome.points) };
        self.message = format!("{} won: {won}, {points}", self.name(outcome.winner));
        self.dice = None;
        self.phase = Phase::Over { since: now };
    }

    // ----- actions, for a person or the computer -----

    pub fn throw(&mut self, now: Instant) {
        if let Phase::ToRoll { .. } = self.phase {
            self.dice = Some((self.die(), self.die()));
            self.message.clear();
            self.phase = Phase::Throwing { until: now + THROW };
        }
    }

    pub fn may_double(&self) -> bool {
        matches!(self.phase, Phase::ToRoll { .. }) && self.cube.may_double(self.pos.to_move)
    }

    pub fn double(&mut self, now: Instant) {
        if self.may_double() {
            let (side, to) = (self.pos.to_move, self.cube.value * 2);
            self.message = format!("{} doubles to {to}", self.name(side));
            if self.player(side.other()) == Player::Human {
                self.message += &format!(". {}: take (y) or pass (n)?", self.name(side.other()));
            }
            self.phase = Phase::Offered { since: now };
        }
    }

    pub fn answer(&mut self, take: bool, now: Instant) {
        if let Phase::Offered { .. } = self.phase {
            let (doubler, taker) = (self.pos.to_move, self.pos.to_move.other());
            if take {
                self.cube.take(taker);
                self.message = format!("{} takes; the cube is at {}", self.name(taker), self.cube.value);
                self.phase = Phase::ToRoll { since: now };
                if self.player(doubler) == Player::Human {
                    self.message += &format!(". {}: throw the dice", self.name(doubler));
                }
            } else {
                let outcome = Outcome { winner: doubler, kind: Kind::Single, points: self.cube.value, passed: true };
                self.finish(outcome, now);
            }
        }
    }

    /// The mover's point for a spot on the drawn board.
    pub fn point(&self, spot: Spot) -> usize {
        match spot {
            Spot::Bar => BAR,
            Spot::Off => OFF,
            Spot::Point(p) if self.pos.to_move == self.view => p,
            Spot::Point(p) => 25 - p,
        }
    }

    /// The spot drawn for a point of `side`.
    pub fn spot(&self, side: Side, point: usize) -> Spot {
        match point {
            BAR => Spot::Bar,
            OFF => Spot::Off,
            p if side == self.view => Spot::Point(p),
            p => Spot::Point(25 - p),
        }
    }

    fn continuing(&self) -> impl Iterator<Item = &Play> {
        self.plays.iter().filter(|p| p.hops.len() > self.partial.len() && p.hops.starts_with(&self.partial))
    }

    /// The mover's points a man may be moved from next.
    pub fn sources(&self) -> Vec<usize> {
        if self.phase_is_choosing() {
            let mut v: Vec<usize> = self.continuing().map(|p| p.hops[self.partial.len()].from).collect();
            v.sort_unstable_by(|a, b| b.cmp(a));
            v.dedup();
            v
        } else {
            Vec::new()
        }
    }

    /// Where the man on `from` may go, each with the hops that take it there
    /// (one die, or several dice played by the same man).
    pub fn targets(&self, from: usize) -> Vec<(usize, Vec<Hop>)> {
        let mut found: Vec<(usize, Vec<Hop>)> = Vec::new();
        for play in self.continuing() {
            let mut at = from;
            let mut chain = Vec::new();
            for &hop in &play.hops[self.partial.len()..] {
                if hop.from != at || at == OFF {
                    break;
                }
                chain.push(hop);
                at = hop.to;
                match found.iter_mut().find(|(to, _)| *to == at) {
                    Some((_, c)) if c.len() <= chain.len() => {}
                    Some((_, c)) => *c = chain.clone(),
                    None => found.push((at, chain.clone())),
                }
            }
        }
        found.sort_unstable_by(|a, b| b.0.cmp(&a.0));
        found
    }

    fn phase_is_choosing(&self) -> bool {
        matches!(self.phase, Phase::Choosing)
    }

    /// A person points at a spot: pick a man up, or put it down.
    pub fn select(&mut self, spot: Spot) {
        if !self.phase_is_choosing() || !self.human_to_act() {
            return;
        }
        let point = self.point(spot);
        if self.picked == Some(point) {
            self.picked = None;
            return;
        }
        if let Some(from) = self.picked
            && let Some((_, chain)) = self.targets(from).into_iter().find(|(to, _)| *to == point) {
                self.place(chain);
                return;
            }
        if self.sources().contains(&point) {
            self.picked = Some(point);
        } else if self.picked.is_none() {
            // A man that can reach this spot by itself and only one way: move it.
            let reach: Vec<Vec<Hop>> = self
                .sources()
                .into_iter()
                .filter_map(|f| self.targets(f).into_iter().find(|(to, _)| *to == point).map(|(_, c)| c))
                .collect();
            if reach.len() == 1 {
                self.place(reach.into_iter().next().unwrap());
            }
        }
    }

    /// Plays the hops of the man picked up.
    pub fn place(&mut self, chain: Vec<Hop>) {
        self.partial.extend(chain);
        self.picked = None;
        if self.partial.len() == self.plays[0].hops.len() {
            self.phase = Phase::Done;
            self.message = "Enter to finish the turn, Esc to take back".into();
        }
    }

    pub fn take_back(&mut self) {
        match self.phase {
            Phase::Choosing | Phase::Done if self.human_to_act() => {
                if self.picked.take().is_none() && self.partial.pop().is_some() {
                    self.phase = Phase::Choosing;
                    self.message.clear();
                }
            }
            _ => {}
        }
    }

    pub fn confirm(&mut self, now: Instant) {
        if let Phase::Done = self.phase {
            let play = Play { hops: self.partial.clone() };
            self.commit(play, now);
        }
    }

    /// The dice not yet used in this turn.
    pub fn dice_left(&self) -> Vec<u8> {
        let Some((a, b)) = self.dice else { return Vec::new() };
        let mut left = if a == b { vec![a; 4] } else { vec![a, b] };
        let used: Vec<u8> = match &self.phase {
            Phase::Showing { play, shown, .. } => play.hops[..*shown].iter().map(|h| h.die).collect(),
            Phase::Choosing | Phase::Done => self.partial.iter().map(|h| h.die).collect(),
            Phase::Opening { .. } | Phase::Throwing { .. } => Vec::new(),
            _ => left.clone(),
        };
        for d in used {
            if let Some(i) = left.iter().position(|&x| x == d) {
                left.remove(i);
            }
        }
        left
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rand::SeedableRng;

    fn friends() -> App {
        let rng = ChaCha8Rng::seed_from_u64(3);
        App::new([Player::Human, Player::Human], rng, None, false, Instant::now())
    }

    fn until_choosing(app: &mut App, mut now: Instant) -> Instant {
        for _ in 0..50 {
            now += Duration::from_secs(2);
            app.tick(now);
            if matches!(app.phase, Phase::Choosing) {
                return now;
            }
            if matches!(app.phase, Phase::ToRoll { .. }) {
                app.throw(now);
            }
        }
        panic!("never came to choosing: {:?}", app.phase);
    }

    #[test]
    fn a_person_plays_a_turn() {
        let mut app = friends();
        let now = until_choosing(&mut app, Instant::now());
        let side = app.pos.to_move;
        while matches!(app.phase, Phase::Choosing) {
            let from = app.sources()[0];
            let view = app.spot(side, from);
            app.select(view);
            assert_eq!(app.picked, Some(from));
            let (to, _) = app.targets(from)[0];
            app.select(app.spot(side, to));
        }
        assert!(matches!(app.phase, Phase::Done));
        app.take_back();
        assert!(matches!(app.phase, Phase::Choosing));
        let from = app.sources()[0];
        let (_, chain) = app.targets(from)[0].clone();
        app.place(chain);
        app.confirm(now);
        assert_eq!(app.pos.to_move, side.other());
        assert!(matches!(app.phase, Phase::ToRoll { .. }));
        assert_eq!(app.history.len(), 1);
    }

    #[test]
    fn double_and_pass_ends_the_game() {
        let mut app = friends();
        let now = until_choosing(&mut app, Instant::now());
        let first = app.pos.to_move;
        let play = rules::distinct(&app.pos, &app.plays)[0].clone();
        app.place(play.hops);
        app.confirm(now);
        app.double(now);
        assert!(matches!(app.phase, Phase::Offered { .. }));
        app.answer(false, now);
        assert!(matches!(app.phase, Phase::Over { .. }));
        assert_eq!(app.score[first.other().index()], 1);
    }

    /// Whole games between two computers through the engine: every play
    /// legal, the cube used, and each game ends.
    #[test]
    fn computers_finish_games() {
        let mut computer = Some(Computer::load().expect("engine"));
        let mut games = 0;
        for seed in 0..4 {
            let rng = ChaCha8Rng::seed_from_u64(seed);
            let seats = [Player::Computer(Strength::Hard), Player::Computer(Strength::Normal)];
            let mut now = Instant::now();
            let mut app = App::new(seats, rng, computer.take(), true, now);
            let mut ticks = 0;
            while !matches!(app.phase, Phase::Over { .. }) {
                now += Duration::from_secs(2);
                app.tick(now);
                ticks += 1;
                assert!(ticks < 5000, "game did not end");
            }
            assert!(app.score.iter().sum::<u32>() >= 1);
            games += 1;
            computer = app.into_computer();
        }
        assert_eq!(games, 4);
    }
}
