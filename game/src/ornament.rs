//! Decoration around the board, after the inlaid tavla and nardy boards:
//! a ground of eight-pointed stars drawn in braille. Each terminal cell is
//! a 2 x 4 grid of dots, and the dots are close to square, so the figures
//! are worked out in dot space.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};

use crate::ui::SURFACE;

/// Brass for the stars, mother-of-pearl turquoise for their hearts: both
/// kept dim so the ornament stays behind the game.
const BRASS: Color = Color::Rgb(0x6e, 0x5a, 0x35);
const PEARL: Color = Color::Rgb(0x38, 0x62, 0x5d);

/// The bit of each dot in a braille cell, by (column, row).
const DOTS: [[u16; 4]; 2] = [[0x01, 0x02, 0x04, 0x40], [0x08, 0x10, 0x20, 0x80]];

/// The repeat of the pattern, in dots.
const PERIOD: f32 = 28.0;

#[derive(Clone, Copy, PartialEq)]
enum Ink {
    None,
    Brass,
    Pearl,
}

/// What lies under one dot of the ground.
fn ink(x: f32, y: f32) -> Ink {
    // Large stars on a square lattice, small ones between them, offset by
    // half a period, and lines joining them along the diagonals.
    let (cx, cy) = ((x / PERIOD).round() * PERIOD, (y / PERIOD).round() * PERIOD);
    let (dx, dy) = ((x - cx).abs(), (y - cy).abs());
    let line = 0.55;
    // An eight-pointed star of radius `s`: a square and a diamond laid
    // over each other, only the outline of the two together.
    let star = |dx: f32, dy: f32, s: f32| {
        let square = dx.max(dy);
        let diamond = (dx + dy) / std::f32::consts::SQRT_2;
        ((square - s).abs() < line && diamond >= s - line) || ((diamond - s).abs() < line && square >= s - line)
    };
    let big = 0.30 * PERIOD;
    if star(dx, dy, big) {
        return Ink::Brass;
    }
    let r = (dx * dx + dy * dy).sqrt();
    if (r - 0.08 * PERIOD).abs() < 0.6 || r < 0.8 {
        return Ink::Pearl;
    }
    let (hx, hy) = ((PERIOD / 2.0 - dx).abs(), (PERIOD / 2.0 - dy).abs());
    let small = 0.11 * PERIOD;
    if star(hx, hy, small) {
        return Ink::Pearl;
    }
    // The diagonal from a large star's corner to the small star beyond.
    let on_diagonal = (dx - dy).abs() < line * std::f32::consts::SQRT_2;
    if on_diagonal && dx > big && hx.max(hy) > small {
        return Ink::Brass;
    }
    Ink::None
}

/// Fills `area` with the ground, leaving `clear` (and a border round it)
/// untouched. The pattern is centred on `clear`, so it sits the same way
/// round the board whatever the terminal's size.
pub fn ground(buf: &mut Buffer, area: Rect, clear: Rect) {
    let keep = Rect::new(
        clear.x.saturating_sub(2),
        clear.y.saturating_sub(1),
        clear.width + 4,
        clear.height + 2,
    );
    let (ox, oy) = (
        (clear.x as f32 + clear.width as f32 / 2.0) * 2.0,
        (clear.y as f32 + clear.height as f32 / 2.0) * 4.0,
    );
    for cy in area.top()..area.bottom() {
        for cx in area.left()..area.right() {
            if keep.contains((cx, cy).into()) {
                continue;
            }
            let mut bits = 0u16;
            let (mut brass, mut pearl) = (0, 0);
            for (col, rows) in DOTS.iter().enumerate() {
                for (row, &bit) in rows.iter().enumerate() {
                    let x = cx as f32 * 2.0 + col as f32 - ox;
                    let y = cy as f32 * 4.0 + row as f32 - oy;
                    match ink(x, y) {
                        Ink::None => {}
                        Ink::Brass => {
                            bits |= bit;
                            brass += 1;
                        }
                        Ink::Pearl => {
                            bits |= bit;
                            pearl += 1;
                        }
                    }
                }
            }
            if bits == 0 {
                continue;
            }
            let colour = if pearl > brass { PEARL } else { BRASS };
            let symbol = char::from_u32(0x2800 + bits as u32).unwrap();
            buf[(cx, cy)].set_char(symbol).set_style(Style::new().fg(colour).bg(SURFACE));
        }
    }
}

/// One repeat of the rim's chain of linked lozenges, 8 dots along and 4
/// across.
const CHAIN: [&str; 4] = ["...##...", "##....##", "##....##", "...##..."];

/// The rosette in each corner of the rim, 4 dots square.
const CORNER: [&str; 4] = ["#..#", ".##.", ".##.", "#..#"];

fn chain(along: i32, across: i32) -> bool {
    CHAIN[across as usize].as_bytes()[along.rem_euclid(8) as usize] == b'#'
}

/// Fills the rim between `outside` and `field` with the inlaid band:
/// two columns at the sides, one row at the top and bottom.
pub fn band(buf: &mut Buffer, outside: Rect, field: Rect, colour: Color, wood: Color) {
    for cy in outside.top()..outside.bottom() {
        for cx in outside.left()..outside.right() {
            let (side, end) = (cx < field.left() || cx >= field.right(), cy < field.top() || cy >= field.bottom());
            if !side && !end {
                continue;
            }
            let mut bits = 0u16;
            for (col, rows) in DOTS.iter().enumerate() {
                for (row, &bit) in rows.iter().enumerate() {
                    let x = cx as i32 * 2 + col as i32;
                    let y = cy as i32 * 4 + row as i32;
                    let on = if side && end {
                        let (u, v) = ((x - outside.left() as i32 * 2).rem_euclid(4), row as i32);
                        CORNER[v as usize].as_bytes()[u as usize] == b'#'
                    } else if end {
                        chain(x - field.left() as i32 * 2, row as i32)
                    } else {
                        let across = (x - if cx < field.left() { outside.left() } else { field.right() } as i32 * 2).rem_euclid(4);
                        chain(y - field.top() as i32 * 4, across)
                    };
                    if on {
                        bits |= bit;
                    }
                }
            }
            let symbol = char::from_u32(0x2800 + bits as u32).unwrap();
            buf[(cx, cy)].set_char(symbol).set_style(Style::new().fg(colour).bg(wood));
        }
    }
}

#[cfg(test)]
mod tests {
    /// Paints the ground alone, for a look by eye:
    /// `CELLS=path cargo test -p backgammon ground_alone -- --ignored`.
    #[test]
    #[ignore]
    fn ground_alone() {
        use ratatui::buffer::Buffer;
        use ratatui::layout::Rect;
        let area = Rect::new(0, 0, 90, 30);
        let mut buf = Buffer::empty(area);
        super::ground(&mut buf, area, Rect::new(45, 15, 0, 0));
        crate::ui::tests::dump(&buf, &std::env::var("CELLS").unwrap());
    }
}
