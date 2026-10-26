# bgsage, vendored

From github.com/markbgsage/bgsage, commit
139427d826dd67798226e7d23c003c90315306ba (2026-10-17), by Mark Higgins,
MPL-2.0 (see LICENSE). Copied unchanged; only what the Stage 11 model plays
with:

- `cpp/src`, `cpp/include`: the engine and its C interface (`mobile.cpp`,
  `mobile.h`); the Python bindings, CUDA and benchmark data are left out
- `models/`: the 21 distinct weight files of Stage 11 (`sl_s9_*` less
  `prim_prim`, `anch_prim`, `anch_anch`, which share `prim_anch`; and the 7
  `sl_s11_bg_*`)
- `data/bearoff_1sided.db`

Do not edit these files; MPL-2.0 would require changes to them to stay
open. Patches, if any, go in files of our own.
