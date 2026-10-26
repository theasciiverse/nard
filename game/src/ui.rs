//! Drawing. The layout follows Pouya Hadidi's Backgammon TUI (MIT): each
//! point a slim box with its tip and number, the bar and bear-off tray as
//! columns, a side panel of players, dice and recent plays, and an action
//! panel under the board. It scales with the terminal and stays centred.

use ratatui::buffer::Buffer;
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::widgets::{Block, BorderType, Widget};

use crate::app::{App, Phase, Player, Spot};
use crate::rules::{BAR, MEN, OFF, Side};

// Hadidi's "Obsidian" theme.
pub const SURFACE: Color = Color::Rgb(0x18, 0x18, 0x1b);
const BORDER: Color = Color::Rgb(0x3f, 0x3f, 0x46);
const PRIMARY: Color = Color::Rgb(0x06, 0xb6, 0xd4);
const ACCENT: Color = Color::Rgb(0xa8, 0x55, 0xf7);
const TEXT: Color = Color::Rgb(0xf4, 0xf4, 0xf5);
const MUTED: Color = Color::Rgb(0x71, 0x71, 0x7a);
const WHITE_MAN: Color = Color::Rgb(0xff, 0xff, 0xff);
const BLACK_MAN: Color = Color::Rgb(0xe1, 0x1d, 0x48);
const LEGAL: Color = Color::Rgb(0x22, 0xc5, 0x5e);
const SELECTED: Color = Color::Rgb(0xea, 0xb3, 0x08);
const CURSOR: Color = Color::Rgb(0x38, 0xbd, 0xf8);
const WARNING: Color = Color::Rgb(0xef, 0x44, 0x44);

// The board as a piece of furniture: walnut, brass, painted points.
const WALNUT: Color = Color::Rgb(0x1f, 0x19, 0x14);
const RIM: Color = Color::Rgb(0x2a, 0x20, 0x17);
const WOOD: Color = Color::Rgb(0x2c, 0x22, 0x19);
const BRASS: Color = Color::Rgb(0xb8, 0x93, 0x5a);
const BRASS_DIM: Color = Color::Rgb(0x6e, 0x58, 0x36);
/// The two paints of the points: border, then tip.
const IVORY: (Color, Color) = (Color::Rgb(0x7d, 0x73, 0x5e), Color::Rgb(0xd8, 0xc9, 0xa3));
const CRIMSON: (Color, Color) = (Color::Rgb(0x73, 0x2b, 0x37), Color::Rgb(0xc8, 0x45, 0x5a));
/// The tallest a point is drawn, in men; taller looks empty.
const MOST_ROWS: u16 = 10;

pub const MIN_WIDTH: u16 = 100;
const RIM_W: u16 = 3;
const RIM_H: u16 = 2;
pub const MIN_HEIGHT: u16 = 26;

/// Keys for the men that may move, in the action panel (none that mean
/// something else).
pub const SOURCE_KEYS: &[char] = &['a', 's', 'f', 'g', 'z', 'x', 'c', 'v', 'b', 'e', 'r', 't', 'w', 'i', 'p'];

fn fg(c: Color) -> Style {
    Style::new().fg(c).bg(SURFACE)
}

fn bold(c: Color) -> Style {
    fg(c).add_modifier(Modifier::BOLD)
}

fn man_glyph(side: Side) -> &'static str {
    match side {
        Side::White => "●",
        Side::Black => "○",
    }
}

fn man_colour(side: Side) -> Color {
    match side {
        Side::White => WHITE_MAN,
        Side::Black => BLACK_MAN,
    }
}

/// Sizes for one step of the scaling layout.
#[derive(Clone, Copy, Debug)]
struct Step {
    point: u16,
    bar: u16,
    tray: u16,
    pad: u16,
    gap: u16,
    panel: u16,
}

const STEPS: [Step; 4] = [
    Step { point: 9, bar: 11, tray: 13, pad: 1, gap: 2, panel: 40 },
    Step { point: 7, bar: 9, tray: 11, pad: 1, gap: 2, panel: 36 },
    Step { point: 5, bar: 9, tray: 11, pad: 1, gap: 2, panel: 36 },
    Step { point: 5, bar: 5, tray: 7, pad: 0, gap: 1, panel: 22 },
];

impl Step {
    fn board_width(&self) -> u16 {
        2 + 2 * self.pad + 12 * self.point + (self.bar + 2) + (self.tray + 1)
    }

    fn width(&self) -> u16 {
        self.board_width() + self.gap + self.panel
    }
}

/// Where everything went on screen, for clicks.
#[derive(Clone, Debug, Default)]
pub struct Layout {
    pub spots: Vec<(Rect, Spot)>,
    pub dice: Rect,
}

impl Layout {
    pub fn spot_at(&self, x: u16, y: u16) -> Option<Spot> {
        self.spots.iter().find(|(r, _)| r.contains((x, y).into())).map(|&(_, s)| s)
    }
}

/// The cursor as a column (0-5 points, 6 bar, 7-12 points, 13 tray) and
/// a row (0 top, 1 bottom) of the drawn board.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Cursor {
    pub col: u8,
    pub row: u8,
}

impl Cursor {
    pub fn spot(self) -> Spot {
        match (self.row, self.col) {
            (_, 6) => Spot::Bar,
            (_, 13) => Spot::Off,
            (0, c @ 0..=5) => Spot::Point(13 + c as usize),
            (0, c) => Spot::Point(12 + c as usize),
            (_, c @ 0..=5) => Spot::Point(12 - c as usize),
            (_, c) => Spot::Point(13 - c as usize),
        }
    }

    pub fn at(spot: Spot, row: u8) -> Cursor {
        match spot {
            Spot::Bar => Cursor { col: 6, row },
            Spot::Off => Cursor { col: 13, row },
            Spot::Point(p @ 13..=18) => Cursor { col: (p - 13) as u8, row: 0 },
            Spot::Point(p @ 19..=24) => Cursor { col: (p - 12) as u8, row: 0 },
            Spot::Point(p @ 7..=12) => Cursor { col: (12 - p) as u8, row: 1 },
            Spot::Point(p) => Cursor { col: (13 - p) as u8, row: 1 },
        }
    }

    pub fn moved(self, dx: i8, dy: i8) -> Cursor {
        Cursor {
            col: (self.col as i8 + dx).clamp(0, 13) as u8,
            row: (self.row as i8 + dy).clamp(0, 1) as u8,
        }
    }
}

fn put(buf: &mut Buffer, x: u16, y: u16, text: &str, style: Style, right: u16) {
    if y >= buf.area.bottom() || x >= right {
        return;
    }
    let width = (right - x) as usize;
    buf.set_stringn(x, y, text, width, style);
}

fn centred(buf: &mut Buffer, r: Rect, y: u16, text: &str, style: Style) {
    let w = text.chars().count() as u16;
    let x = r.x + r.width.saturating_sub(w) / 2;
    put(buf, x, y, text, style, r.right());
}

fn boxed(buf: &mut Buffer, r: Rect, kind: BorderType, colour: Color) -> Rect {
    let block = Block::bordered().border_type(kind).border_style(fg(colour)).style(fg(TEXT));
    let inner = block.inner(r);
    block.render(r, buf);
    inner
}

/// How one spot on the board is lit.
#[derive(Clone, Copy, Default)]
struct Light {
    cursor: bool,
    picked: bool,
    source: bool,
    target: bool,
    hit: bool,
}

impl Light {
    fn border(self, idle: Color) -> Color {
        if self.cursor {
            CURSOR
        } else if self.picked {
            SELECTED
        } else if self.target {
            if self.hit { WARNING } else { LEGAL }
        } else if self.source {
            PRIMARY
        } else {
            idle
        }
    }

    fn label(self) -> Style {
        let c = if self.cursor {
            CURSOR
        } else if self.picked {
            SELECTED
        } else if self.target {
            if self.hit { WARNING } else { LEGAL }
        } else if self.source {
            PRIMARY
        } else {
            return fg(MUTED);
        };
        bold(c)
    }
}

struct Lights {
    cursor: Option<Spot>,
    picked: Option<Spot>,
    sources: Vec<Spot>,
    targets: Vec<(Spot, bool)>,
}

impl Lights {
    fn of(app: &App, cursor: Option<Cursor>) -> Lights {
        let side = app.pos.to_move;
        let human = app.human_to_act();
        let sources = if human { app.sources() } else { Vec::new() };
        let shown = app.shown();
        let targets = match app.picked {
            Some(from) if human => app
                .targets(from)
                .into_iter()
                .map(|(to, chain)| (app.spot(side, to), chain.last().is_some_and(|h| h.hit) && to != OFF && shown.side(side.other())[25 - to] == 1))
                .collect(),
            _ => Vec::new(),
        };
        Lights {
            cursor: if human { cursor.map(Cursor::spot) } else { None },
            picked: app.picked.map(|p| app.spot(side, p)),
            sources: sources.into_iter().map(|p| app.spot(side, p)).collect(),
            targets,
        }
    }

    fn at(&self, spot: Spot) -> Light {
        let target = self.targets.iter().find(|(s, _)| *s == spot);
        Light {
            cursor: self.cursor == Some(spot),
            picked: self.picked == Some(spot),
            source: self.sources.contains(&spot),
            target: target.is_some(),
            hit: target.is_some_and(|t| t.1),
        }
    }
}

/// Draws the whole game screen and says where things went.
/// `ornate`: the braille rim and ground, which some fonts draw badly.
pub fn draw(app: &App, cursor: Cursor, ornate: bool, area: Rect, buf: &mut Buffer) -> Layout {
    buf.set_style(area, fg(TEXT));
    let mut layout = Layout::default();
    if area.width < MIN_WIDTH || area.height < MIN_HEIGHT {
        too_small(area, buf);
        return layout;
    }
    let step = *STEPS.iter().find(|s| s.width() <= area.width).unwrap_or(&STEPS[3]);
    let action_h = 5;
    // An inlaid rim round the board when there is room: 3 columns a side
    // (wall and band), 2 rows top and bottom.
    let rim = ornate && area.width >= step.width() + 2 * RIM_W && area.height >= 2 * 5 + 16 + 2 * RIM_H;
    let (rim_w, rim_h) = if rim { (RIM_W, RIM_H) } else { (0, 0) };
    let rows = ((area.height - 16 - 2 * rim_h) / 2).clamp(5, MOST_ROWS.min(MEN as u16));
    let board_h = 2 + 2 * (rows + 4) + 2 * rim_h;
    let width = step.width() + 2 * rim_w;
    let total_h = board_h + 1 + action_h;
    let x0 = area.x + (area.width - width) / 2;
    let y0 = area.y + area.height.saturating_sub(total_h) / 2;
    let board = Rect::new(x0, y0, step.board_width() + 2 * rim_w, board_h);
    let panel = Rect::new(board.right() + step.gap, y0, step.panel, board_h);
    let action = Rect::new(x0, board.bottom() + 1, width, action_h);

    if ornate {
        crate::ornament::ground(buf, area, Rect::new(x0, y0, width, total_h));
    }
    let lights = Lights::of(app, Some(cursor));
    draw_board(app, &lights, step, rows, rim, board, buf, &mut layout);
    draw_panel(app, step, panel, buf, &mut layout);
    draw_action(app, action, buf);
    layout
}

fn too_small(area: Rect, buf: &mut Buffer) {
    let y = area.y + area.height / 2;
    centred(buf, area, y.saturating_sub(1), "Backgammon needs a larger terminal", bold(TEXT));
    let need = format!("at least {MIN_WIDTH}×{MIN_HEIGHT}, now {}×{}", area.width, area.height);
    centred(buf, area, y + 1, &need, fg(MUTED));
}

/// Changes one background colour to another within `r`.
fn recolour(buf: &mut Buffer, r: Rect, from: Color, to: Color) {
    for y in r.top()..r.bottom() {
        for x in r.left()..r.right() {
            let cell = &mut buf[(x, y)];
            if cell.bg == from {
                cell.bg = to;
            }
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_board(app: &App, lights: &Lights, step: Step, rows: u16, rim: bool, r: Rect, buf: &mut Buffer, layout: &mut Layout) {
    // The outer wall: a double brass line, then the inlaid rim.
    let wall = boxed(buf, r, BorderType::Double, BRASS);
    let r = if rim {
        let field = Rect::new(r.x + RIM_W, r.y + RIM_H, r.width - 2 * RIM_W, r.height - 2 * RIM_H);
        crate::ornament::band(buf, wall, field, BRASS_DIM, RIM);
        field
    } else {
        wall.outer(ratatui::layout::Margin::new(1, 1))
    };
    let inner = if rim { boxed(buf, r, BorderType::Plain, BRASS_DIM) } else { wall };
    let inner_field = inner;
    let pos = app.shown();
    let point_h = rows + 4;
    let left = inner.x + step.pad;
    let bar_x = left + 6 * step.point + 1;
    let right = bar_x + step.bar + 1;
    let tray_x = right + 6 * step.point + 1;
    for col in 0..12u16 {
        let x = if col < 6 { left + col * step.point } else { right + (col - 6) * step.point };
        let c = if col < 6 { col as u8 } else { col as u8 + 1 };
        for row in 0..2u8 {
            let spot = Cursor { col: c, row }.spot();
            let rect = Rect::new(x, inner.y + row as u16 * point_h, step.point, point_h);
            // The points alternate in colour along each side, and face a
            // point of the other colour.
            let paint = if (col + row as u16).is_multiple_of(2) { IVORY } else { CRIMSON };
            draw_point(app, &pos, spot, row == 0, rows, lights.at(spot), paint, rect, buf);
            layout.spots.push((rect, spot));
        }
    }
    let bar = Rect::new(bar_x, inner.y, step.bar, 2 * point_h);
    draw_bar(app, &pos, lights.at(Spot::Bar), rows, bar, buf);
    layout.spots.push((bar, Spot::Bar));
    let tray = Rect::new(tray_x, inner.y, step.tray, 2 * point_h);
    draw_tray(app, &pos, lights.at(Spot::Off), rows, tray, buf);
    layout.spots.push((tray, Spot::Off));
    recolour(buf, inner_field, SURFACE, WALNUT);
    recolour(buf, bar.inner(ratatui::layout::Margin::new(1, 1)), WALNUT, WOOD);
}

/// Who holds a drawn point, and how many men.
fn holder(app: &App, pos: &crate::rules::Position, spot: Spot) -> Option<(Side, u8)> {
    let Spot::Point(p) = spot else { return None };
    let (view, other) = (app.view, app.view.other());
    let mine = pos.side(view)[p];
    let theirs = pos.side(other)[25 - p];
    if mine > 0 {
        Some((view, mine))
    } else if theirs > 0 {
        Some((other, theirs))
    } else {
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn draw_point(app: &App, pos: &crate::rules::Position, spot: Spot, top: bool, rows: u16, light: Light,
              paint: (Color, Color), r: Rect, buf: &mut Buffer) {
    let inner = boxed(buf, r, BorderType::Plain, light.border(paint.0));
    let Spot::Point(number) = spot else { return };
    let (tip, men_y, label_y) = if top {
        (inner.y, inner.y + 1, inner.y + rows + 1)
    } else {
        (inner.y + rows, inner.y, inner.y + rows + 1)
    };
    let tip_style = if light.cursor || light.picked || light.target { light.label() } else { bold(paint.1) };
    centred(buf, inner, tip, if top { "▼" } else { "▲" }, tip_style);
    centred(buf, inner, label_y, &number.to_string(), light.label());
    // Slot 0 is at the board's edge.
    let slot_y = |slot: u16| if top { men_y + slot } else { men_y + rows - 1 - slot };
    let held = holder(app, pos, spot);
    if let Some((side, n)) = held {
        let style = if light.picked {
            bold(SELECTED)
        } else if light.hit {
            bold(WARNING)
        } else {
            bold(man_colour(side))
        };
        let n = n as u16;
        for slot in 0..n.min(rows) {
            let text = if n > rows && slot == rows - 1 { format!("{n}") } else { man_glyph(side).to_string() };
            centred(buf, inner, slot_y(slot), &text, style);
        }
    }
    if light.target && !light.hit {
        let slot = held.map_or(0, |(_, n)| n as u16);
        if slot < rows {
            centred(buf, inner, slot_y(slot), "◌", bold(LEGAL));
        }
    }
}

/// Men in a column of the bar or tray: a glyph per man, the count if more.
fn stack(buf: &mut Buffer, r: Rect, ys: impl Iterator<Item = u16>, side: Side, n: u8, style: Style) {
    let ys: Vec<u16> = ys.collect();
    let room = ys.len() as u8;
    for (i, &y) in ys.iter().enumerate().take(n.min(room) as usize) {
        let text = if n > room && i + 1 == room as usize { format!("{n}") } else { man_glyph(side).to_string() };
        centred(buf, r, y, &text, style);
    }
}

fn cube_text(value: u32) -> String {
    format!(" {value} ")
}

fn cube_style() -> Style {
    Style::new().fg(SURFACE).bg(TEXT).add_modifier(Modifier::BOLD)
}

fn draw_bar(app: &App, pos: &crate::rules::Position, light: Light, rows: u16, r: Rect, buf: &mut Buffer) {
    let inner = boxed(buf, r, BorderType::Plain, light.border(BRASS_DIM));
    let half = rows + 2;
    let wide = inner.width >= 7;
    let (bottom, top) = (app.view, app.view.other());
    // The bottom side's men on the bar wait in the top half, where they enter.
    let style = |side: Side| if light.picked && side == app.pos.to_move { bold(SELECTED) } else { bold(man_colour(side)) };
    let label = |side: Side| if wide { format!("{}-BAR", &side.name()[..1]) } else { man_glyph(side).to_string() };
    centred(buf, inner, inner.y, &label(bottom), fg(MUTED));
    stack(buf, inner, inner.y + 1..inner.y + half, bottom, pos.side(bottom)[BAR], style(bottom));
    let mid = inner.y + half;
    let name = if light.cursor { "[BAR]" } else { "BAR" };
    centred(buf, inner, mid, name, light.label().patch(if light.cursor || light.source { Style::new() } else { fg(BORDER) }));
    if app.cube.owner.is_none() {
        centred(buf, inner, mid + 1, &cube_text(app.cube.value), cube_style());
    }
    let bottom_y = mid + 2;
    centred(buf, inner, bottom_y + half - 1, &label(top), fg(MUTED));
    stack(buf, inner, (bottom_y..bottom_y + half - 1).rev(), top, pos.side(top)[BAR], style(top));
}

fn draw_tray(app: &App, pos: &crate::rules::Position, light: Light, _rows: u16, r: Rect, buf: &mut Buffer) {
    let inner = boxed(buf, r, BorderType::Plain, light.border(BRASS_DIM));
    let half = inner.height.saturating_sub(2) / 2;
    let wide = inner.width >= 9;
    let (bottom, top) = (app.view, app.view.other());
    let count = |side: Side| {
        if wide { format!("{} {}/15", man_glyph(side), pos.off(side)) } else { format!("{}{}", man_glyph(side), pos.off(side)) }
    };
    // The top side's men off in the top half, as on a real board.
    if wide {
        centred(buf, inner, inner.y, &format!("{}-OFF", &top.name()[..1]), fg(MUTED));
    }
    centred(buf, inner, inner.y + wide as u16, &count(top), bold(man_colour(top)));
    let mid = inner.y + half;
    let name = if light.cursor { "[OFF]" } else if light.target { "►OFF◄" } else { "OFF" };
    let style = if light.cursor || light.target { light.label() } else { bold(BORDER) };
    centred(buf, inner, mid, name, style);
    if light.target && wide {
        centred(buf, inner, mid - 2, "► BEAR ◄", bold(LEGAL));
    }
    let end = inner.bottom() - 1;
    centred(buf, inner, end - wide as u16, &count(bottom), bold(man_colour(bottom)));
    if wide {
        centred(buf, inner, end, &format!("{}-OFF", &bottom.name()[..1]), fg(MUTED));
    }
    // An owned cube sits on its owner's side.
    if let Some(owner) = app.cube.owner {
        let y = if owner == top { mid - 1 } else { mid + 1 };
        centred(buf, inner, y, &cube_text(app.cube.value), cube_style());
    }
}

fn phase_tag(app: &App) -> &'static str {
    match app.phase {
        Phase::Opening { .. } => "OPENING",
        Phase::ToRoll { .. } | Phase::Throwing { .. } => "ROLLING",
        Phase::Offered { .. } => "CUBE",
        Phase::Choosing | Phase::Done | Phase::Showing { .. } | Phase::NoMove { .. } => "MOVING",
        Phase::Over { .. } => "GAME OVER",
    }
}

const DIE_FACES: [&str; 7] = ["", "⚀", "⚁", "⚂", "⚃", "⚄", "⚅"];

fn draw_panel(app: &App, step: Step, r: Rect, buf: &mut Buffer, layout: &mut Layout) {
    let outer = boxed(buf, r, BorderType::Rounded, BRASS_DIM);
    let padx = if step.panel >= 30 { 2 } else { 1 };
    let inner = Rect::new(outer.x + padx, outer.y + 1, outer.width - 2 * padx, outer.height.saturating_sub(2));
    let rx = inner.right();
    let mut y = inner.y;
    put(buf, inner.x, y, &format!("ROUND {}", app.turn.max(1)), bold(PRIMARY), rx);
    let tag = format!("[{}]", phase_tag(app));
    put(buf, rx.saturating_sub(tag.len() as u16), y, &tag, bold(MUTED), rx);
    y += 2;

    let to_move = app.pos.to_move;
    let thinking = matches!(app.player(to_move), Player::Computer(_))
        && matches!(app.phase, Phase::ToRoll { .. } | Phase::Throwing { .. } | Phase::Showing { .. });
    if thinking && inner.height > 22 {
        let badge = boxed(buf, Rect::new(inner.x, y, inner.width, 3), BorderType::Plain, ACCENT);
        let what = if matches!(app.phase, Phase::Showing { .. }) { "moving" } else { "rolling" };
        centred(buf, badge, badge.y, &format!("{} {what}…", app.name(to_move)), bold(ACCENT));
        y += 4;
    }

    for side in [app.view.other(), app.view] {
        let on_turn = side == to_move && !matches!(app.phase, Phase::Opening { .. } | Phase::Over { .. });
        let card = boxed(buf, Rect::new(inner.x, y, inner.width, 4), BorderType::Plain, if on_turn { CURSOR } else { BORDER });
        let card = Rect::new(card.x + 1, card.y, card.width.saturating_sub(2), card.height);
        put(buf, card.x, card.y, &format!("{} {}", man_glyph(side), app.name(side)), bold(man_colour(side)), card.right());
        if on_turn {
            put(buf, card.right() - 6, card.y, "◀ TURN", bold(PRIMARY), card.right());
        }
        let pos = app.shown();
        let (pips, off) = if card.width >= 22 {
            (format!("Pips: {}", pos.pips(side)), format!("Off: {}/15", pos.off(side)))
        } else {
            (format!("Pips {}", pos.pips(side)), format!("Off {}", pos.off(side)))
        };
        put(buf, card.x, card.y + 1, &pips, fg(MUTED), card.right());
        put(buf, card.right().saturating_sub(off.len() as u16), card.y + 1, &off, fg(MUTED), card.right());
        y += 5;
    }

    // The dice.
    let left = app.dice_left();
    let shown: Vec<u8> = match (app.dice, &app.phase) {
        (_, Phase::Opening { dice, .. }) => dice.to_vec(),
        (Some((a, b)), _) if a == b => vec![a; 4],
        (Some((a, b)), _) => vec![a, b],
        (None, _) => Vec::new(),
    };
    // Each die boxed when they fit, else in a row of text.
    let boxed_dice = inner.width > 2 + 2 * 7;
    let die_w = if 2 + 4 * 7 + 3 <= inner.width || shown.len() < 4 { 7 } else { 5 };
    let dice_box = Rect::new(inner.x, y, inner.width, if boxed_dice { 7 } else { 5 });
    let lit = !left.is_empty() && !matches!(app.phase, Phase::Opening { .. });
    let d = boxed(buf, dice_box, BorderType::Plain, if lit { PRIMARY } else { BORDER });
    let d = Rect::new(d.x + 1, d.y, d.width.saturating_sub(2), d.height);
    layout.dice = dice_box;
    put(buf, d.x, d.y, "DICE & MOVES", bold(MUTED), d.right());
    if shown.len() == 4 {
        put(buf, d.right().saturating_sub(8), d.y, "DOUBLES!", bold(WARNING), d.right());
    }
    let mut unused = left.clone();
    let faces: Vec<(String, bool)> = shown
        .iter()
        .map(|&v| {
            let fresh = matches!(app.phase, Phase::Opening { .. } | Phase::Throwing { .. });
            let avail = fresh || unused.iter().position(|&x| x == v).map(|i| unused.remove(i)).is_some();
            let face = if die_w == 7 { format!("{} {v}", DIE_FACES[v as usize]) } else { v.to_string() };
            (face, avail)
        })
        .collect();
    if !faces.is_empty() && boxed_dice {
        let boxes_w = faces.len() as u16 * (die_w + 1) - 1;
        let mut x = d.x + d.width.saturating_sub(boxes_w) / 2;
        for (text, avail) in &faces {
            let b = boxed(buf, Rect::new(x, d.y + 1, die_w, 3), BorderType::Plain, if *avail { PRIMARY } else { BORDER });
            centred(buf, b, b.y, text, if *avail { bold(TEXT) } else { fg(MUTED) });
            x += die_w + 1;
        }
    } else if !faces.is_empty() {
        let mut x = d.x;
        for (text, avail) in &faces {
            put(buf, x, d.y + 1, text, if *avail { bold(TEXT) } else { fg(MUTED) }, d.right());
            x += text.chars().count() as u16 + 2;
        }
    }
    let note = match app.phase {
        Phase::Opening { dice, .. } => (format!("{} {} · {} {}", man_glyph(Side::White), dice[0], man_glyph(Side::Black), dice[1]), MUTED),
        Phase::ToRoll { .. } if app.human_to_act() => ("► Space to throw".into(), PRIMARY),
        Phase::ToRoll { .. } | Phase::Offered { .. } => ("waiting".into(), MUTED),
        Phase::Throwing { .. } => ("throwing…".into(), MUTED),
        Phase::Over { .. } => ("game over".into(), MUTED),
        _ if left.is_empty() => ("✓ all dice used".into(), MUTED),
        _ => (format!("{} move{} left", left.len(), if left.len() == 1 { "" } else { "s" }), LEGAL),
    };
    centred(buf, d, d.bottom() - 1, &note.0, bold(note.1));
    y += dice_box.height + 1;

    // The cube and the score.
    let cube = match app.cube.owner {
        None => "centred".to_string(),
        Some(s) => format!("{}'s", app.name(s)).replace("You's", "yours"),
    };
    put(buf, inner.x, y, "CUBE", bold(MUTED), rx);
    put(buf, inner.x + 7, y, &cube_text(app.cube.value), cube_style(), rx);
    put(buf, inner.x + 8 + cube_text(app.cube.value).len() as u16, y, &cube, fg(TEXT), rx);
    y += 1;
    put(buf, inner.x, y, "SCORE", bold(MUTED), rx);
    let mut x = inner.x + 7;
    for side in [app.view, app.view.other()] {
        let t = format!("{} {}", man_glyph(side), app.score[side.index()]);
        put(buf, x, y, &t, bold(man_colour(side)), rx);
        x += t.chars().count() as u16 + 3;
    }
    y += 2;

    // Recent plays, newest first.
    if y < inner.bottom() {
        put(buf, inner.x, y, "RECENT MOVES:", bold(MUTED), rx);
        y += 1;
    }
    for (i, turn) in app.history.iter().rev().enumerate() {
        if y >= inner.bottom() {
            break;
        }
        let line = format!("{} {}{} {}", man_glyph(turn.side), turn.dice.0, turn.dice.1, turn.play);
        let style = if i == 0 { fg(TEXT) } else { fg(MUTED) };
        put(buf, inner.x, y, &line, style.patch(Style::new().fg(if i == 0 { man_colour(turn.side) } else { MUTED })), rx);
        y += 1;
    }
}

/// A name without its strength, for tight places.
fn short(name: &str) -> &str {
    name.split(" (").next().unwrap_or(name)
}

fn chip(buf: &mut Buffer, x: &mut u16, y: u16, key: &str, text: &str, colour: Color, right: u16) {
    let k = format!("[{key}]");
    put(buf, *x, y, &k, bold(PRIMARY), right);
    *x += k.chars().count() as u16 + 1;
    put(buf, *x, y, text, fg(colour), right);
    *x += text.chars().count() as u16 + 3;
}

fn draw_action(app: &App, r: Rect, buf: &mut Buffer) {
    let human = app.human_to_act();
    let border = if human { PRIMARY } else { BRASS_DIM };
    let outer = boxed(buf, r, BorderType::Rounded, border);
    let inner = Rect::new(outer.x + 1, outer.y, outer.width.saturating_sub(2), outer.height);
    let rx = inner.right();
    let side = app.pos.to_move;
    let title = match (&app.phase, human) {
        (Phase::Opening { .. }, _) => "OPENING THROW".to_string(),
        (Phase::ToRoll { .. }, true) => "YOUR THROW".to_string(),
        (Phase::Offered { .. }, true) => "DOUBLE OFFERED".to_string(),
        (Phase::Choosing, true) => match app.picked {
            Some(p) => format!("CHOOSE DESTINATION  moving from {}", point_name(p)),
            None => format!("MOVABLE CHECKERS  dice left: {}", dice_list(&app.dice_left())),
        },
        (Phase::Done, true) => "TURN COMPLETE".to_string(),
        (Phase::Over { .. }, _) => "GAME OVER".to_string(),
        (Phase::NoMove { .. }, _) => "NO MOVE".to_string(),
        _ => format!("{}'S TURN", short(&app.name(side)).to_uppercase()),
    };
    put(buf, inner.x, inner.y, &title, bold(PRIMARY), rx);
    if !app.message.is_empty() {
        let w = app.message.chars().count() as u16;
        let x = rx.saturating_sub(w).max(inner.x + title.chars().count() as u16 + 3);
        put(buf, x, inner.y, &app.message, fg(SELECTED), rx);
    }

    let y = inner.y + 1;
    let mut x = inner.x;
    match (&app.phase, human) {
        (Phase::ToRoll { .. }, true) => {
            chip(buf, &mut x, y, "Space", "throw the dice", TEXT, rx);
            if app.may_double() {
                chip(buf, &mut x, y, "D", &format!("double to {}", app.cube.value * 2), TEXT, rx);
            }
        }
        (Phase::Offered { .. }, true) => {
            chip(buf, &mut x, y, "Y", &format!("take, play on for {}", app.cube.value * 2), TEXT, rx);
            chip(buf, &mut x, y, "N", &format!("pass, lose {}", app.cube.value), TEXT, rx);
        }
        (Phase::Choosing, true) => match app.picked {
            None => {
                for (i, p) in app.sources().into_iter().enumerate() {
                    let key = SOURCE_KEYS.get(i).map_or(String::new(), |c| c.to_ascii_uppercase().to_string());
                    let n = app.shown().mover()[p];
                    chip(buf, &mut x, y, &key, &format!("{} ({n})", point_name(p)), TEXT, rx);
                }
            }
            Some(from) => {
                for (i, (to, chain)) in app.targets(from).into_iter().enumerate().take(9) {
                    let hit = chain.iter().any(|h| h.hit);
                    let text = format!("→ {}{}", point_name(to), if hit { " hit!" } else { "" });
                    chip(buf, &mut x, y, &(i + 1).to_string(), &text, if hit { WARNING } else { TEXT }, rx);
                }
            }
        },
        (Phase::Done, true) => {
            chip(buf, &mut x, y, "Enter", "finish the turn", TEXT, rx);
            chip(buf, &mut x, y, "Esc", "take back", TEXT, rx);
        }
        (Phase::Over { .. }, _) if !app.demo => {
            chip(buf, &mut x, y, "Enter", "next game", TEXT, rx);
            chip(buf, &mut x, y, "M", "menu", TEXT, rx);
        }
        _ => {
            let last = app.history.last().map(|t| format!("{} played {}", short(&app.name(t.side)), t.play));
            put(buf, x, y, &last.unwrap_or_default(), fg(MUTED), rx);
        }
    }

    let keys = "←→↑↓ move · Enter pick/place · Esc take back · D double · O ornament · F1 help · M menu · Q quit";
    put(buf, inner.x, inner.y + 2, keys, fg(MUTED), rx);
}

fn point_name(p: usize) -> String {
    match p {
        BAR => "bar".into(),
        OFF => "off".into(),
        _ => p.to_string(),
    }
}

fn dice_list(dice: &[u8]) -> String {
    dice.iter().map(|d| format!("[{} {d}]", DIE_FACES[*d as usize])).collect::<Vec<_>>().join(" ")
}

#[cfg(test)]
pub mod tests {
    use super::*;
    use crate::app::Player;
    use rand::SeedableRng;
    use std::time::Instant;

    fn app() -> App {
        let rng = rand_chacha::ChaCha8Rng::seed_from_u64(1);
        App::new([Player::Human, Player::Human], rng, None, false, Instant::now())
    }

    fn render(app: &App, w: u16, h: u16) -> (Buffer, Layout) {
        let area = Rect::new(0, 0, w, h);
        let mut buf = Buffer::empty(area);
        let layout = draw(app, Cursor { col: 0, row: 1 }, true, area, &mut buf);
        (buf, layout)
    }

    pub fn text(buf: &Buffer) -> String {
        let mut s = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                s.push_str(buf[(x, y)].symbol());
            }
            s.push('\n');
        }
        s
    }

    #[test]
    fn fits_and_centres_at_every_size() {
        let app = app();
        for (w, h) in [(100, 30), (125, 38), (140, 40), (200, 55), (100, 26)] {
            let (buf, layout) = render(&app, w, h);
            assert_eq!(layout.spots.len(), 26, "{w}x{h}");
            for (i, (a, _)) in layout.spots.iter().enumerate() {
                assert!(a.right() <= w && a.bottom() <= h, "{w}x{h}: {a:?}");
                for (b, _) in &layout.spots[i + 1..] {
                    assert!(!a.intersects(*b), "{w}x{h}: {a:?} overlaps {b:?}");
                }
            }
            let used: Vec<u16> = (0..w).filter(|&x| (0..h).any(|y| buf[(x, y)].symbol() != " ")).collect();
            let (left, right) = (used[0], w - 1 - used[used.len() - 1]);
            assert!(left.abs_diff(right) <= 1, "{w}x{h}: margins {left} and {right}");
        }
    }

    /// Prints frames for a look by eye: `cargo test -p backgammon frames -- --ignored --nocapture`.
    #[test]
    #[ignore]
    fn frames() {
        let mut app = app();
        let mut now = Instant::now();
        while !matches!(app.phase, Phase::Choosing) {
            now += std::time::Duration::from_secs(2);
            app.tick(now);
        }
        let from = app.sources()[0];
        app.select(app.spot(app.pos.to_move, from));
        let size: Vec<u16> = std::env::var("SIZE").unwrap_or("100x30".into()).split('x').map(|n| n.parse().unwrap()).collect();
        let (buf, _) = render(&app, size[0], size[1]);
        println!("{}", text(&buf));
        if let Ok(path) = std::env::var("CELLS") {
            dump(&buf, &path);
        }
    }

    /// Writes every cell with its colours, for painting as an image.
    pub fn dump(buf: &Buffer, path: &str) {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                let c = &buf[(x, y)];
                let rgb = |c: Color| match c { Color::Rgb(r, g, b) => format!("{r},{g},{b}"), _ => "-".into() };
                out += &format!("{x}\t{y}\t{}\t{}\t{}\t{}\n", c.symbol(), rgb(c.fg), rgb(c.bg), c.modifier.contains(Modifier::BOLD) as u8);
            }
        }
        std::fs::write(path, out).unwrap();
    }

    #[test]
    fn too_small_says_so() {
        let (buf, layout) = render(&app(), 90, 24);
        assert!(layout.spots.is_empty());
        assert!(text(&buf).contains("larger terminal"));
    }

    #[test]
    fn cursor_walks_the_board() {
        let mut c = Cursor { col: 0, row: 0 };
        assert_eq!(c.spot(), Spot::Point(13));
        c = c.moved(1, 0);
        assert_eq!(c.spot(), Spot::Point(14));
        c = Cursor { col: 12, row: 1 };
        assert_eq!(c.spot(), Spot::Point(1));
        for p in 1..=24 {
            assert_eq!(Cursor::at(Spot::Point(p), 0).spot(), Spot::Point(p));
        }
    }
}
