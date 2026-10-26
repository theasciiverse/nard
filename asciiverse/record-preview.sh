#!/bin/sh
# Record the 100x30 catalog preview, asciiverse/nard.cast, with hub's
# castrec.py: the computer playing itself (--demo), 45 s from the opening
# roll, with a double taken along the way. The engine's files are written
# to a throwaway cache, so the recording machine's own cache is not used.
set -e
cd "$(dirname "$0")/.."
here=$(pwd)
castrec=$(cd ../../hub/tools && pwd)/castrec.py
cargo build --release
tmp=$(mktemp -d)
cp target/release/nard "$tmp/"
(cd "$tmp" && PATH="$tmp:$PATH" HOME="$tmp" XDG_DATA_HOME="$tmp/data" XDG_CACHE_HOME="$tmp/cache" \
  python3 "$castrec" "$here/asciiverse/nard.cast" . "w45;kq;w0.5;kq;w0.5" \
  -- nard --demo --seed "${1:-1}")
rm -rf "$tmp"
