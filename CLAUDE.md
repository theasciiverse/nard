# Backgammon

A full-screen terminal backgammon for the Asciiverse catalog, its own
title (David, 2026-10-25), separate from the Roman Tabula in Ancient
Games. Decided to build (David, 2026-10-25), beside chess-tui and
termsuji (Go), which stay as they are for now. Not started: this folder
and these notes only, no code yet. Own git repo, branch `main`, private
at github.com/theasciiverse/backgammon.

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

## The plan so far

The look: David likes Hadidi's Backgammon TUI (2026-10-25): the board on
the left with point numbers and triangles, a side panel of round, whose
turn it is, pips and men off for each side, the dice and moves left, and
recent moves; a status line at the foot. It is MIT, so its layout and
drawing may be followed or ported, crediting Pouya Hadidi; its demo GIF
is in the repo. Its rules engine (`src/core/rules/BackgammonRules.ts`,
maximal move sequences) can be checked against ours.

A Rust + Ratatui board in that look, with the conveniences the Ancient
Games have (keys and mouse, F1 help, `--demo`, `--seed`) but not their
graphical style, driving bgsage's C++ engine for the
computer opponent and the doubling cube; GNU Backgammon's engine (GPL)
as the fallback if bgsage proves hard to embed.

## Settled

- Its own title, not part of the Ancient Games collection (David,
  2026-10-25)
- A graphical style of its own, clearly different from the Ancient
  Games, led by Hadidi's Backgammon TUI, whose look David likes (see
  "The look" above; David, 2026-10-25)

## Open questions (David's to decide)

- bgsage or gnubg as the engine, after trying to build bgsage's C++ core
  without Python
- Money play, matches with the cube and the Crawford rule, or both
- Name (working name: Backgammon)
- Licence (chosen case by case)
