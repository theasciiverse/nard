# Nard

Nard: a full-screen terminal backgammon for the Asciiverse catalog, its own
title (David, 2026-10-25), separate from the Roman Tabula in Ancient
Games. Decided to build (David, 2026-10-25), beside chess-tui and
termsuji (Go), which stay as they are for now. Milestone 0, the engine
test, done 2026-10-26 (below); milestone 1, a playable money game,
2026-10-26 (see "The game"). Own git repo, branch `main`, private at
github.com/theasciiverse/nard.

On the website mockup since 2026-10-26 with the NEW! banner: an entry in
`websites/Asciiverse/dev/mock-titles.toml` (original game, licence not
chosen, so not in `hub/catalog` yet), preview `asciiverse/nard.cast`
(`asciiverse/record-preview.sh`, hub's `castrec.py`: `--demo --seed 1`,
45 s, still at 24 s) copied to the site's `casts/`, local Linux build
packaged by the site's `dev/build_catalog.py` into `dev-releases/`.

## Why build it

Checked 2026-10-25 (web, GitHub, the projects' own pages and code): the
strong engines have no real terminal interface, and the terminal games
have no real opponent.

- **GNU Backgammon** (`gnubg`, GPL, Ubuntu 1.08.003): the strongest free
  engine. Its `-t`/`--tty` mode is a command line that prints an ASCII
  board and takes typed commands (`roll`, `move 8/5 6/5`, `double`); no
  cursor, no mouse, no screen redrawn in place. Its real interface is the
  GTK window. Known from its man page, manual and gnu.org; not run here.
- **bgsage** (Open Sage, github.com/markbgsage/bgsage, MPL-2.0 for code,
  trained weights in `models/` and bearoff database in `data/`): a Python
  and C++ engine library with a neural network, multi-ply, truncated and
  full rollouts, and cube decisions (no double, double/take, double/pass).
  Claims about the strength of eXtreme Gammon ("some weak evidence that
  Sage 3T is a bit stronger"; its claim, not tested). Python and C++
  calls only, no CLI or engine protocol. Started 2026-02, active
  (pushed 2026-10-17), 13 stars; the README says most of its code was
  written by Claude Code and Codex. MPL is file-level copyleft: files of
  it we change must stay open, our own files stay ours.
- **Backgammon TUI** (Pouya Hadidi, github.com/hadidipouya/nodejs-backgammon,
  npm `backgammon-tui`, MIT): a good-looking full-screen board in
  TypeScript, React and Ink (Node), with pip counts, undo and themes, but
  a weak opponent: `src/ai/RandomStrategy.ts` (50 lines) scores +10 a hit
  and +5 a man borne off and picks at random among the best; no blot
  safety (its web page claims otherwise), no doubling cube. One day of
  commits (2026-09-20), 0 stars. Seen from its code and demo GIF; not run.
- **bgammon.org** (Trevor Slocum, AGPL-3.0): an online server; terminal
  play over `ssh bgammon.org -p 5000` and the `bgammon-cli` client
  (codeberg.org/tslocum/bgammon-cli, last commit 2024-07); against people,
  not a local computer. Slocum's Boxcars client is graphical.
- **BSD backgammon** (Alan Char, 1980): already in Asciiverse in the
  NetBSD games collection, with `teachgammon`; 80x24, weak computer.
- Small GitHub projects only otherwise (largest quuixly/Backgammon, C and
  ncurses, MIT, 8 stars; cursefibs, an ncurses FIBS client, quiet since
  2023).

## Engine sizes (measured 2026-10-25)

- bgsage: the repo is about 590 MB, nearly all training and benchmark
  data. What it plays with is the production model `stage11`
  (`python/bgsage/weights.py`): 24 networks, Stage 9's 17 `sl_s9_*` plus
  7 `sl_s11_bg_*`, and `data/bearoff_1sided.db`: 25 files, 14.2 MB. C++
  source about 1.3 MB.
- GNU Backgammon (Ubuntu 1.08.003 packages): the network `gnubg.wd`
  0.4 MB and the one-sided bearoff database `gnubg_os0.bd` 1.5 MB; the
  `gnubg` binary 1.9 MB (with the GTK interface). Its data package is
  7.1 MB to download, mostly manuals, sounds and translations.

## Engine test (milestone 0, 2026-10-26)

bgsage builds and plays from Rust with no Python and no CMake. It has a C
interface of its own, `cpp/src/mobile.cpp` / `mobile.h` (added
2026-09-07): create an engine from weight paths, then ask for every legal
play of a roll ranked by cubeful equity, or for a cube decision (No
Double, Double/Take, Double/Pass, with the equities), as JSON. 1- or
2-ply only, a time budget of at most 1000 ms per call.

- Layout: `vendor/bgsage` (pinned copy, see its `VENDORED.md`; 14 MB, of
  which 12.5 MB is networks and the bearoff database), `bgsage-sys` (a
  `build.rs` that compiles the C++ with the `cc` crate, and the FFI
  declarations), `bgsage` (safe `Engine`: `load`, `plays`,
  `cube_decision`), `bgsage/examples/probe.rs`.
- Measured on this machine (8 cores): the C++ compiles in about 11 s;
  the probe binary is 1.1 MB (0.9 MB stripped), linking only libstdc++
  and libc; the engine loads in 30 ms; a play takes 0.14 ms at 1-ply and
  26 ms at 2-ply from the opening position; a cube decision 0.01 ms at
  1-ply and 1-13 ms at 2-ply.
- Play: for the 15 opening rolls it picks the book play or a modern
  near-equal one (2-ply: 46 24/18 13/9, 52 24/22 13/8, 43 24/21 13/9).
  The cube answers were right on four test positions: the opening (no
  double), an 81-89 race (double/take), a near-certain win (double/pass)
  and a likely gammon (too good to double).
- Catch: on x86 the networks are always compiled for AVX2
  (`neural_net.cpp` defines `BGBOT_USE_AVX2` on any x86), so the game needs
  an x86 CPU from about 2013 on, or ARM. A scalar path would mean patching
  their file.
- The engine answers in boards, not moves: our rules code must turn the
  chosen board back into a move (and check it is legal).
- MPL-2.0 holds only for bgsage's files; our crates can carry any licence.

Decided: bgsage (David, 2026-10-26); gnubg is not needed.

## The game (milestone 1, 2026-10-26)

`cargo run --release` (then `-- --demo`, `-- --seed N`, `-- --help`).
Crate `game/`, package and binary `nard`:

- `rules.rs`: position (each side in its own numbering, 25 bar, 0 off),
  every legal play in every hop order (most dice, the higher die), the
  distinct plays with standard notation, cube, gammons and backgammons.
  Tested against bgsage: over 3,000 positions from random games, our
  distinct plays end on exactly bgsage's candidate boards.
- `ai.rs`: bgsage picks, our rules spell it. Hard 2-ply, normal 1-ply,
  easy a random play within 0.12 equity of the best and never doubles.
- `assets.rs`: the engine files are inside the binary (15.9 MB) and
  written once to `~/.cache/nard/stage11-139427d/`
  (`NARD_ASSETS` overrides).
- `app.rs`: turns, the opening throw, the cube offered and answered, a
  person's play built hop by hop (one man may use both dice in one go),
  Enter to finish, Esc to take back.
- `ui.rs`: Hadidi's look, in four sizes by terminal width (points 5, 7 or
  9 wide), more men shown on taller terminals, centred; 100x26 at least.
  Owned cube in the owner's half of the tray, centred cube on the bar.
  Dressed as a tavla board: double brass wall, walnut field, points
  painted ivory and crimson in turn, a wood bar.
- `ornament.rs`: braille decoration (2x4 dots a cell, near-square dots):
  a rim of linked lozenges inside the wall, and round the board on large
  terminals a ground of eight-pointed stars (khatam) in dim brass and
  turquoise. O toggles it, `--plain` starts without; some fonts draw
  braille thin.
- Running: `cargo run --release -- --demo` (game options after `--`), or
  `cargo demo`.
- `term.rs`, `main.rs`: the loop, start screen, F1 help (`HELP.txt`).
  crossterm can deliver Esc joined to the next key as Alt+key, so Alt+key
  is taken as Esc then the key.

Not yet: themes, bigger men on the larger sizes, the Jacoby rule, matches,
saving a game.

## The plan so far

The look: David likes Hadidi's Backgammon TUI (2026-10-25): the board on
the left with point numbers and triangles, a side panel of round, whose
turn it is, pips and men off for each side, the dice and moves left, and
recent moves; a status line at the foot. It is MIT, so its layout and
drawing may be followed or ported, crediting Pouya Hadidi; its demo GIF
is in the repo. Its rules engine (`src/core/rules/BackgammonRules.ts`,
maximal move sequences) can be checked against ours.

A Rust + Ratatui board in that look, scaling with the terminal (100x30
at the least, larger points and more men shown on larger screens) and
always centred; keys and mouse, F1 help, `--demo`, `--seed`; driving
bgsage's C++ engine for the computer opponent and the doubling cube.

## Settled

- Its own title, not part of the Ancient Games collection (David,
  2026-10-25)
- A graphical style of its own, clearly different from the Ancient
  Games, led by Hadidi's Backgammon TUI, whose look David likes (see
  "The look" above; David, 2026-10-25)
- Freestanding: zero dependency on ancient-games, in code, UI and
  style; nothing shared or copied, a new take (David, 2026-10-26)
- The board scales with the terminal and stays centred (David,
  2026-10-26)
- The engine test first, then the game (David, 2026-10-26)
- The engine is bgsage (David, 2026-10-26, after the engine test)
- The name is Nard, command `nard` (David, 2026-10-26); every
  description still says backgammon ("Backgammon in the terminal").
  `backgammon` is taken on Debian (`bsdgames`) and FreeBSD
  (`games/bsdgames`) by Alan Char's BSD game; gammon and tavla were
  turned down (ham; a mistake in Swedish).
- Money play first: single games with the cube, gammons and backgammons
  (David, 2026-10-26)

## Open questions (David's to decide)

- Matches with the cube and the Crawford rule, later or not at all
- Licence (chosen case by case)
