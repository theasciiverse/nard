//! bgsage's networks and bearoff database, carried inside the binary and
//! written out once to a cache directory, where the engine reads them.

use std::fs;
use std::io;
use std::path::PathBuf;
use std::sync::atomic::{AtomicU32, Ordering};

/// The vendored bgsage commit, so a newer copy gets its own directory.
const VERSION: &str = "stage11-139427d";

macro_rules! files {
    ($($name:literal),* $(,)?) => {
        &[$(
            (concat!("models/", $name, ".weights.best"),
             include_bytes!(concat!("../../vendor/bgsage/models/", $name, ".weights.best")) as &[u8]),
        )*]
    };
}

const MODELS: &[(&str, &[u8])] = files![
    "sl_s9_purerace", "sl_s9_race_race", "sl_s9_race_att", "sl_s9_race_prim",
    "sl_s9_race_anch", "sl_s9_att_race", "sl_s9_att_att", "sl_s9_att_prim",
    "sl_s9_att_anch", "sl_s9_prim_race", "sl_s9_prim_att", "sl_s9_prim_anch",
    "sl_s9_anch_race", "sl_s9_anch_att", "sl_s11_bg_deep", "sl_s11_bg_middle",
    "sl_s11_bg_double", "sl_s11_bg_p3", "sl_s11_bg_containment", "sl_s11_bg_snake",
    "sl_s11_bg_massive",
];

const BEAROFF: (&str, &[u8]) =
    ("data/bearoff_1sided.db", include_bytes!("../../vendor/bgsage/data/bearoff_1sided.db"));

/// The directory holding the engine's files: `NARD_ASSETS` if set,
/// else a cache directory, filled on first use: `XDG_CACHE_HOME`, on
/// Windows `LOCALAPPDATA` (where HOME is usually not set), else
/// `~/.cache`.
pub fn dir() -> io::Result<PathBuf> {
    if let Some(dir) = std::env::var_os("NARD_ASSETS") {
        return Ok(dir.into());
    }
    let var = |name| std::env::var_os(name).filter(|v| !v.is_empty()).map(PathBuf::from);
    let cache = var("XDG_CACHE_HOME")
        .or_else(|| if cfg!(windows) { var("LOCALAPPDATA") } else { None })
        .or_else(|| var("HOME").map(|home| home.join(".cache")))
        .ok_or(io::ErrorKind::NotFound)?;
    let dir = cache.join("nard").join(VERSION);
    for &(name, bytes) in MODELS.iter().chain([&BEAROFF]) {
        let path = dir.join(name);
        if fs::metadata(&path).is_ok_and(|m| m.len() == bytes.len() as u64) {
            continue;
        }
        fs::create_dir_all(path.parent().unwrap())?;
        // Write beside it and rename, so a half-written file is never used;
        // the temporary name is this writer's own, as two games starting
        // at once may both be writing.
        static WRITES: AtomicU32 = AtomicU32::new(0);
        let n = WRITES.fetch_add(1, Ordering::Relaxed);
        let part = path.with_extension(format!("part-{}-{n}", std::process::id()));
        fs::write(&part, bytes)?;
        fs::rename(&part, &path)?;
    }
    Ok(dir)
}
