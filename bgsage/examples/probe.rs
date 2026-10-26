//! Milestone 0: does bgsage load and play sensibly from Rust, and how fast?
//! Run with `cargo run --release --example probe`.

use std::path::Path;
use std::time::Instant;

use bgsage::{Board, Cube, CubeOwner, Engine, STARTING_BOARD};

/// The book opening plays (2024 rollouts), as from/to pairs.
const OPENINGS: [((u8, u8), &str); 15] = [
    ((2, 1), "13/11 6/5"), ((3, 1), "8/5 6/5"), ((4, 1), "24/23 13/9"),
    ((5, 1), "24/23 13/8"), ((6, 1), "13/7 8/7"), ((3, 2), "24/21 13/11"),
    ((4, 2), "8/4 6/4"), ((5, 2), "13/11 13/8"), ((6, 2), "24/18 13/11"),
    ((4, 3), "24/20 13/10"), ((5, 3), "8/3 6/3"), ((6, 3), "24/18 13/10"),
    ((5, 4), "24/20 13/8"), ((6, 4), "8/2 6/2"), ((6, 5), "24/13"),
];

/// Describes a play by the points it took men from and put men on.
fn describe(before: &Board, after: &Board) -> String {
    let mut from = Vec::new();
    let mut to = Vec::new();
    for p in (1..=25).rev() {
        let d = after[p].max(0) - before[p].max(0);
        for _ in 0..d.abs() {
            if d < 0 { from.push(p) } else { to.push(p) }
        }
    }
    format!("from {from:?} to {to:?}")
}

fn board(points: &[(usize, i32)]) -> Board {
    let mut b = [0; 26];
    for &(p, n) in points {
        b[p] = n;
    }
    b
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let assets = Path::new(env!("CARGO_MANIFEST_DIR")).join("../vendor/bgsage");
    let started = Instant::now();
    let mut engine = Engine::load(&assets)?;
    println!("loaded in {:?}\n", started.elapsed());

    for ply in [1, 2] {
        println!("opening rolls, {ply}-ply (book play in brackets):");
        let started = Instant::now();
        for (dice, book) in OPENINGS {
            let plays = engine.plays(&STARTING_BOARD, dice, ply, Cube::default())?;
            let best = &plays[0];
            println!(
                "  {}{}: {:<40} eq {:+.3}  [{book}]",
                dice.1, dice.0, describe(&STARTING_BOARD, &best.board), best.equity
            );
        }
        println!("  {:?} per roll\n", started.elapsed() / OPENINGS.len() as u32);
    }

    // Our men positive on points 1-24 counted from our home board; the
    // opponent's negative, its pips counted as 25 - point.
    let cases: [(&str, Board); 4] = [
        ("opening position (no double)", STARTING_BOARD),
        // 81 pips to their 89, on roll, no contact: a double and a take.
        ("81 vs 89 pip race (double/take)", board(&[
            (8, 3), (6, 4), (5, 3), (4, 3), (3, 2),
            (15, -2), (17, -3), (19, -2), (20, -3), (21, -3), (22, -2),
        ])),
        // We are off in two turns for sure; they need two, bar 6-6.
        ("4 men on our 1-2 vs 3 on their 6 (double/pass)", board(&[
            (1, 2), (2, 2), (19, -3),
        ])),
        // Off in two turns against 15 men not yet home: play on for the gammon.
        ("gammon chances (too good to double)", board(&[
            (1, 2), (2, 2),
            (12, -3), (14, -3), (16, -3), (18, -2), (20, -2), (22, -2),
        ])),
    ];
    for ply in [1, 2] {
        println!("cube decisions, {ply}-ply:");
        for (name, b) in &cases {
            let started = Instant::now();
            let d = engine.cube_decision(b, ply, Cube::default())?;
            println!(
                "  {name}: {} (ND {:+.3}, D/T {:+.3}, D/P {:+.3}) in {:?}",
                d.optimal_action, d.equity_nd, d.equity_dt, d.equity_dp, started.elapsed()
            );
        }
        println!();
    }

    // Cube owned by the opponent: the side on roll cannot double.
    let owned = Cube { value: 2, owner: CubeOwner::Opponent, jacoby: false };
    let d = engine.cube_decision(&cases[1].1, 1, owned)?;
    println!("same race with the opponent owning the cube: {}", d.optimal_action);
    Ok(())
}
