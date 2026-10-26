# Backgammon

Backgammon in the terminal, against a strong computer or a friend, on a
board dressed like an inlaid tavla board: a brass wall, a braille-inlaid
rim, ivory and crimson points, and a ground of eight-pointed stars around
it on large terminals.

The computer is bgsage (Open Sage), a neural-network engine, at three
strengths: easy (picks among the plays close to the best, never doubles),
normal (one ply) and hard (two plies). Money play with the doubling cube,
gammons and backgammons, the score kept across the session.

- Keys and mouse: pick up a man and put it down, or use the letter and
  number keys listed under the board; a man may use both dice in one go.
  Enter finishes the turn, Esc takes back a move.
- F1 for the rules of backgammon and the cube, and the keys.
- The board scales with the terminal and stays centred: 100x26 at the
  least, larger points and the ornament on larger screens. True colour
  and UTF-8; O switches the braille ornament off for fonts that draw it
  badly.

Options: `--seed N` (the same dice again), `--demo` (the computer plays
itself), `--plain` (no ornament). For a session log:
`RUST_LOG=debug backgammon 2>game.log`.

Build: `cargo build --release` (Rust 1.88 or newer and a C++17 compiler,
for bgsage). One binary of about 16 MB, the engine's networks inside it;
on first run they are written to `~/.cache/backgammon/`. On Intel and AMD
the engine needs a processor with AVX2 (2013 or newer).

Run: `cargo run --release`, or `cargo demo` to watch the computer play
itself (options to the game go after `--`: `cargo run --release -- --seed 7`).

## Credits

- The engine: [bgsage](https://github.com/markbgsage/bgsage) by Mark
  Higgins, MPL-2.0; its files are in `vendor/bgsage`, unchanged (see
  `VENDORED.md` there).
- The board, side panel and colours follow
  [Backgammon TUI](https://github.com/hadidipouya/nodejs-backgammon) by
  Pouya Hadidi (MIT).

Part of [Asciiverse](https://asciiverse.sh).
