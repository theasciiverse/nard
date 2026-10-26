//! The terminal: set up, an event loop, and put back as it was, even on a panic.

use std::io::{self, stdout};
use std::time::{Duration, Instant};

use ratatui::crossterm::event::{
    self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind, KeyModifiers, MouseButton,
    MouseEventKind,
};
use ratatui::crossterm::execute;
use ratatui::Frame;

pub trait Program {
    fn tick(&mut self, now: Instant);
    fn key(&mut self, key: KeyCode, now: Instant);
    fn click(&mut self, x: u16, y: u16, now: Instant);
    fn draw(&mut self, frame: &mut Frame);
    fn done(&self) -> bool;
}

fn restore() {
    let _ = execute!(stdout(), DisableMouseCapture);
    ratatui::restore();
}

pub fn run(program: &mut impl Program) -> io::Result<()> {
    let hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        restore();
        hook(info);
    }));
    let mut terminal = ratatui::init();
    execute!(stdout(), EnableMouseCapture)?;
    let result = (|| {
        while !program.done() {
            program.tick(Instant::now());
            terminal.draw(|f| program.draw(f))?;
            if !event::poll(Duration::from_millis(30))? {
                continue;
            }
            let now = Instant::now();
            match event::read()? {
                Event::Key(k) if k.kind == KeyEventKind::Press => {
                    if k.code == KeyCode::Char('c') && k.modifiers.contains(KeyModifiers::CONTROL) {
                        break;
                    }
                    log::debug!("key {:?} {:?}", k.code, k.modifiers);
                    // An Esc can reach us joined to the next key as Alt+key;
                    // Alt means nothing here, so it is the two keys.
                    if k.modifiers.contains(KeyModifiers::ALT) {
                        program.key(KeyCode::Esc, now);
                    }
                    program.key(k.code, now);
                }
                Event::Mouse(m) if m.kind == MouseEventKind::Down(MouseButton::Left) => {
                    program.click(m.column, m.row, now)
                }
                _ => {}
            }
        }
        Ok(())
    })();
    restore();
    result
}
