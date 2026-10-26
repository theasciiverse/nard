//! Nard: backgammon in the terminal, against bgsage or a friend.

mod ai;
mod app;
mod assets;
mod ornament;
mod rules;
mod term;
mod ui;

use std::time::{Instant, SystemTime, UNIX_EPOCH};

use rand::{Rng, SeedableRng};
use rand_chacha::ChaCha8Rng;
use ratatui::Frame;
use ratatui::crossterm::event::KeyCode;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::widgets::{Block, BorderType, Widget};

use ai::{Computer, Strength};
use app::{App, Phase, Player, Spot};
use rules::Side;
use ui::{Cursor, Layout, SURFACE};

const HELP: &str = include_str!("../HELP.txt");

const USAGE: &str = "\
nard: backgammon in the terminal, against bgsage or a friend

  --seed N   throw the same dice again (the seed of a game is in the log)
  --demo     the computer plays itself, game after game
  --plain    no braille ornament (O switches it during play)
  --help     this

RUST_LOG=debug nard 2>game.log  keeps a log of the session.";

const OPPONENTS: [(&str, Option<Strength>); 4] = [
    ("Sage · easy", Some(Strength::Easy)),
    ("Sage · normal", Some(Strength::Normal)),
    ("Sage · hard", Some(Strength::Hard)),
    ("a friend", None),
];

enum Mode {
    Start,
    Play(Box<App>),
}

struct Screen {
    mode: Mode,
    opponent: usize,
    colour: Side,
    row: usize,
    help: Option<u16>,
    cursor: Cursor,
    layout: Layout,
    computer: Option<Computer>,
    engine_error: Option<String>,
    rng: ChaCha8Rng,
    demo: bool,
    ornate: bool,
    quit: bool,
    /// The turn the cursor was last placed for.
    placed: Option<u32>,
}

impl Screen {
    fn new(seed: u64, demo: bool, computer: Option<Computer>, engine_error: Option<String>) -> Screen {
        Screen {
            mode: Mode::Start,
            opponent: 2,
            colour: Side::White,
            row: 0,
            help: None,
            cursor: Cursor { col: 7, row: 1 },
            layout: Layout::default(),
            computer,
            engine_error,
            rng: ChaCha8Rng::seed_from_u64(seed),
            demo,
            ornate: true,
            quit: false,
            placed: None,
        }
    }

    fn start_game(&mut self, now: Instant) {
        let seats = if self.demo {
            [Player::Computer(Strength::Hard), Player::Computer(Strength::Normal)]
        } else {
            let other = match OPPONENTS[self.opponent].1 {
                Some(s) if self.computer.is_some() => Player::Computer(s),
                _ => Player::Human,
            };
            match self.colour {
                Side::White => [Player::Human, other],
                Side::Black => [other, Player::Human],
            }
        };
        let rng = ChaCha8Rng::seed_from_u64(self.rng.random());
        let app = App::new(seats, rng, self.computer.take(), self.demo, now);
        self.cursor = Cursor { col: 7, row: 1 };
        self.placed = None;
        self.mode = Mode::Play(Box::new(app));
    }

    fn back_to_menu(&mut self) {
        if let Mode::Play(app) = std::mem::replace(&mut self.mode, Mode::Start) {
            self.computer = app.into_computer();
        }
    }

    fn start_key(&mut self, key: KeyCode, now: Instant) {
        match key {
            KeyCode::Up | KeyCode::Char('k') => self.row = 0,
            KeyCode::Down | KeyCode::Char('j') | KeyCode::Tab => self.row = 1 - self.row,
            KeyCode::Left | KeyCode::Right | KeyCode::Char('h') | KeyCode::Char('l') => {
                let back = matches!(key, KeyCode::Left | KeyCode::Char('h'));
                if self.row == 0 {
                    let n = OPPONENTS.len();
                    self.opponent = (self.opponent + if back { n - 1 } else { 1 }) % n;
                } else {
                    self.colour = self.colour.other();
                }
            }
            KeyCode::Enter | KeyCode::Char(' ') => self.start_game(now),
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            _ => {}
        }
    }

    fn play_key(&mut self, key: KeyCode, now: Instant) {
        let Mode::Play(app) = &mut self.mode else { return };
        let row_of = |spot: Spot, c: Cursor| match spot {
            Spot::Point(p) if p >= 13 => 0,
            Spot::Point(_) => 1,
            _ => c.row,
        };
        match key {
            KeyCode::Char('q') => self.quit = true,
            KeyCode::Char('m') => self.back_to_menu(),
            KeyCode::Char('o') => self.ornate = !self.ornate,
            KeyCode::Left | KeyCode::Char('h') => self.cursor = self.cursor.moved(-1, 0),
            KeyCode::Right | KeyCode::Char('l') => self.cursor = self.cursor.moved(1, 0),
            KeyCode::Up | KeyCode::Char('k') => self.cursor = self.cursor.moved(0, -1),
            KeyCode::Down | KeyCode::Char('j') => self.cursor = self.cursor.moved(0, 1),
            KeyCode::Esc | KeyCode::Backspace | KeyCode::Char('u') => app.take_back(),
            KeyCode::Char('d') if app.human_to_act() => app.double(now),
            KeyCode::Char('y') if app.human_to_act() => app.answer(true, now),
            KeyCode::Char('n') if app.human_to_act() => app.answer(false, now),
            KeyCode::Enter | KeyCode::Char(' ') => match app.phase {
                Phase::ToRoll { .. } if app.human_to_act() => app.throw(now),
                Phase::Choosing => app.select(self.cursor.spot()),
                Phase::Done => app.confirm(now),
                Phase::Over { .. } if !app.demo => app.new_game(now),
                _ => {}
            },
            KeyCode::Char(c @ '1'..='9') => {
                if let (Phase::Choosing, Some(from)) = (&app.phase, app.picked) {
                    let i = c as usize - '1' as usize;
                    if let Some((to, chain)) = app.targets(from).into_iter().nth(i) {
                        let spot = app.spot(app.pos.to_move, to);
                        app.place(chain);
                        self.cursor = Cursor::at(spot, row_of(spot, self.cursor));
                    }
                }
            }
            KeyCode::Char(c) => {
                if let Some(i) = ui::SOURCE_KEYS.iter().position(|&k| k == c)
                    && matches!(app.phase, Phase::Choosing) && app.human_to_act()
                        && let Some(&from) = app.sources().get(i) {
                            let spot = app.spot(app.pos.to_move, from);
                            app.select(spot);
                            self.cursor = Cursor::at(spot, row_of(spot, self.cursor));
                        }
            }
            _ => {}
        }
    }

    fn help_key(&mut self, key: KeyCode) {
        let Some(top) = self.help.as_mut() else { return };
        let page = 10;
        match key {
            KeyCode::Esc | KeyCode::F(1) | KeyCode::Char('?') | KeyCode::Char('q') => self.help = None,
            KeyCode::Up | KeyCode::Char('k') => *top = top.saturating_sub(1),
            KeyCode::Down | KeyCode::Char('j') => *top += 1,
            KeyCode::PageUp => *top = top.saturating_sub(page),
            KeyCode::PageDown | KeyCode::Char(' ') => *top += page,
            KeyCode::Home => *top = 0,
            KeyCode::End => *top = u16::MAX,
            _ => {}
        }
    }

    /// Puts the cursor on a man that may move when a person's turn begins.
    fn place_cursor(&mut self) {
        let Mode::Play(app) = &self.mode else { return };
        if !matches!(app.phase, Phase::Choosing) || !app.human_to_act() || self.placed == Some(app.turn) {
            return;
        }
        self.placed = Some(app.turn);
        if let Some(&from) = app.sources().first() {
            let spot = app.spot(app.pos.to_move, from);
            if app.sources().iter().all(|&p| app.spot(app.pos.to_move, p) != self.cursor.spot()) {
                let row = if matches!(spot, Spot::Point(p) if p >= 13) { 0 } else { 1 };
                self.cursor = Cursor::at(spot, row);
            }
        }
    }
}

impl term::Program for Screen {
    fn tick(&mut self, now: Instant) {
        if let Mode::Play(app) = &mut self.mode {
            app.tick(now);
        }
        self.place_cursor();
    }

    fn key(&mut self, key: KeyCode, now: Instant) {
        if self.help.is_some() {
            return self.help_key(key);
        }
        if matches!(key, KeyCode::F(1) | KeyCode::Char('?')) {
            self.help = Some(0);
            return;
        }
        match self.mode {
            Mode::Start => self.start_key(key, now),
            Mode::Play(_) => self.play_key(key, now),
        }
    }

    fn click(&mut self, x: u16, y: u16, now: Instant) {
        if self.help.is_some() {
            return;
        }
        let Mode::Play(app) = &mut self.mode else { return self.start_game(now) };
        let spot = self.layout.spot_at(x, y);
        let on_dice = self.layout.dice.contains((x, y).into());
        match app.phase {
            Phase::ToRoll { .. } if app.human_to_act() => app.throw(now),
            Phase::Choosing => {
                if let Some(spot) = spot {
                    let row = if matches!(spot, Spot::Point(p) if p >= 13) { 0 } else { self.cursor.row };
                    self.cursor = Cursor::at(spot, row);
                    app.select(spot);
                }
            }
            Phase::Done if on_dice => app.confirm(now),
            Phase::Over { .. } if !app.demo => app.new_game(now),
            _ => {}
        }
    }

    fn draw(&mut self, frame: &mut Frame) {
        let area = frame.area();
        let buf = frame.buffer_mut();
        match &self.mode {
            Mode::Start => draw_start(self, area, buf),
            Mode::Play(app) => self.layout = ui::draw(app, self.cursor, self.ornate, area, buf),
        }
        if let Some(top) = self.help.as_mut() {
            draw_help(top, area, buf);
        }
    }

    fn done(&self) -> bool {
        self.quit
    }
}

fn draw_start(s: &Screen, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    use ratatui::style::Color;
    let primary = Color::Rgb(0x06, 0xb6, 0xd4);
    let muted = Color::Rgb(0x71, 0x71, 0x7a);
    let text = Color::Rgb(0xf4, 0xf4, 0xf5);
    let cursor = Color::Rgb(0x38, 0xbd, 0xf8);
    let base = Style::new().bg(SURFACE);
    buf.set_style(area, base.fg(text));
    let (w, h) = (66.min(area.width), 15.min(area.height));
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    let block = Block::bordered().border_type(BorderType::Rounded).border_style(base.fg(Color::Rgb(0x3f, 0x3f, 0x46)));
    let inner = block.inner(r);
    block.render(r, buf);
    let line = |buf: &mut ratatui::buffer::Buffer, y: u16, t: &str, st: Style| {
        let tw = t.chars().count() as u16;
        buf.set_stringn(inner.x + inner.width.saturating_sub(tw) / 2, inner.y + y, t, inner.width as usize, st);
    };
    line(buf, 1, "● ○  N A R D  ○ ●", base.fg(primary).add_modifier(Modifier::BOLD));
    line(buf, 2, "backgammon · money play with the doubling cube", base.fg(muted));
    let choice = |i: usize| if s.row == i { base.fg(cursor).add_modifier(Modifier::BOLD) } else { base.fg(text) };
    let opponent = if s.computer.is_none() && OPPONENTS[s.opponent].1.is_some() { "a friend (no engine)" } else { OPPONENTS[s.opponent].0 };
    line(buf, 5, &format!("Opponent   ◀  {opponent:<20} ▶"), choice(0));
    let colour = format!("{} {}", if s.colour == Side::White { "●" } else { "○" }, s.colour.name());
    line(buf, 7, &format!("You play   ◀  {colour:<20} ▶"), choice(1));
    line(buf, 10, "[Enter] play   [↑↓←→] choose   [F1] rules   [Q] quit", base.fg(muted));
    let credit = match &s.engine_error {
        Some(e) => format!("engine unavailable: {e}"),
        None => "engine: bgsage (Mark Higgins) · look: Backgammon TUI (Pouya Hadidi)".into(),
    };
    line(buf, 12, &credit, base.fg(muted));
}

fn draw_help(top: &mut u16, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    use ratatui::style::Color;
    let base = Style::new().bg(SURFACE);
    let (w, h) = (84.min(area.width), area.height.saturating_sub(2).min(40));
    let r = Rect::new(area.x + (area.width - w) / 2, area.y + (area.height - h) / 2, w, h);
    ratatui::widgets::Clear.render(r, buf);
    buf.set_style(r, base.fg(Color::Rgb(0xf4, 0xf4, 0xf5)));
    let block = Block::bordered()
        .border_type(BorderType::Rounded)
        .border_style(base.fg(Color::Rgb(0x06, 0xb6, 0xd4)))
        .title(" Rules and keys ")
        .title_bottom(" Esc or F1: back · ↑↓ PgUp PgDn: scroll ");
    let inner = block.inner(r);
    block.render(r, buf);
    let lines: Vec<&str> = HELP.lines().collect();
    let room = inner.height.saturating_sub(1);
    *top = (*top).min((lines.len() as u16).saturating_sub(room));
    for (i, l) in lines.iter().skip(*top as usize).take(room as usize).enumerate() {
        let heading = !l.starts_with(' ') && !l.is_empty() && l.chars().all(|c| !c.is_lowercase());
        let st = if heading { base.fg(Color::Rgb(0x06, 0xb6, 0xd4)).add_modifier(Modifier::BOLD) } else { base };
        buf.set_stringn(inner.x + 2, inner.y + 1 + i as u16, l, inner.width as usize - 3, st);
    }
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.iter().any(|a| a == "--help" || a == "-h") {
        println!("{USAGE}");
        return;
    }
    if std::env::var_os("RUST_LOG").is_some() {
        env_logger::init();
    }
    let seed = match args.iter().position(|a| a == "--seed").and_then(|i| args.get(i + 1)) {
        Some(n) => n.parse().unwrap_or_else(|_| {
            eprintln!("--seed takes a number");
            std::process::exit(2)
        }),
        None => SystemTime::now().duration_since(UNIX_EPOCH).map_or(1, |d| d.as_nanos() as u64),
    };
    log::info!("seed {seed}");
    let demo = args.iter().any(|a| a == "--demo");
    let (computer, engine_error) = match Computer::load() {
        Ok(c) => (Some(c), None),
        Err(e) => {
            log::error!("{e}");
            (None, Some(e))
        }
    };
    let mut screen = Screen::new(seed, demo, computer, engine_error);
    screen.ornate = !args.iter().any(|a| a == "--plain");
    if demo {
        screen.start_game(Instant::now());
    }
    if let Err(e) = term::run(&mut screen) {
        eprintln!("nard: {e}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use term::Program;

    fn frame(screen: &mut Screen, w: u16, h: u16) -> ratatui::buffer::Buffer {
        let mut terminal = ratatui::Terminal::new(ratatui::backend::TestBackend::new(w, h)).unwrap();
        terminal.draw(|f| screen.draw(f)).unwrap();
        terminal.backend().buffer().clone()
    }

    /// A person's whole game through the keys, against hard, to the end:
    /// the start screen, throws, picking and placing, taking back,
    /// finishing turns, the cube; the screen drawn at every step.
    #[test]
    fn a_session_through_the_keys() {
        let mut screen = Screen::new(5, false, Computer::load().ok(), None);
        let mut now = Instant::now();
        let start = frame(&mut screen, 100, 30);
        assert!(ui::tests::text(&start).contains("N A R D"));
        screen.key(KeyCode::Enter, now);
        let mut finished = 0;
        let mut over = false;
        for step in 0..5000 {
            now += Duration::from_millis(700);
            screen.tick(now);
            let Mode::Play(app) = &screen.mode else { panic!("left the game") };
            match app.phase {
                Phase::ToRoll { .. } if app.human_to_act() => screen.key(KeyCode::Char(' '), now),
                Phase::Offered { .. } if app.human_to_act() => screen.key(KeyCode::Char('y'), now),
                Phase::Choosing if app.human_to_act() => {
                    screen.key(KeyCode::Char('a'), now);
                    screen.key(KeyCode::Char('1'), now);
                }
                Phase::Done => {
                    screen.key(KeyCode::Esc, now);
                    screen.key(KeyCode::Char('a'), now);
                    screen.key(KeyCode::Char('1'), now);
                    screen.key(KeyCode::Enter, now);
                    finished += 1;
                }
                Phase::Over { .. } => {
                    over = true;
                    break;
                }
                _ => {}
            }
            let (w, h) = if step % 2 == 0 { (100, 30) } else { (190, 55) };
            frame(&mut screen, w, h);
            if finished == 6 {
                if let Ok(path) = std::env::var("CELLS") {
                    ui::tests::dump(&frame(&mut screen, 125, 38), &path);
                }
            }
        }
        assert!(over, "the game did not end ({finished} turns finished)");
        let Mode::Play(app) = &screen.mode else { unreachable!() };
        assert!(app.score.iter().sum::<u32>() >= 1, "nobody scored");
        let text = ui::tests::text(&frame(&mut screen, 125, 38));
        assert!(text.contains("GAME OVER") && text.contains("next game"), "{text}");
        screen.key(KeyCode::Char('?'), now);
        assert!(ui::tests::text(&frame(&mut screen, 125, 38)).contains("Rules and keys"));
        screen.key(KeyCode::Esc, now);
        screen.key(KeyCode::Char('q'), now);
        assert!(screen.done());
    }

    #[test]
    fn help_fits_its_box() {
        for line in super::HELP.lines() {
            assert!(line.chars().count() <= 78, "too wide: {line}");
        }
    }
}
