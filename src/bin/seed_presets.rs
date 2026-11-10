//! Factory preset seeder for Corrosion.
//!
//! `cargo run --release --bin seed_presets` writes the factory bank built by
//! `corrosion::presets::factory` to `presets/factory/`:
//! - the curated folders (`bass`, `lead`, `chords`, `drones`, `percussion`),
//! - `drums`, ten Kit-mode drum kits (`per-key-kits`),
//! - `experimental/<exciter>/<object>.corrosion-preset`, one per pair.
//!
//! Every preset is levelled by an offline render (a held middle C, 2 s,
//! through the plugin's output path) to the bank's target loudness and
//! checked against the quality floor. An experimental preset that fails, or
//! whose level moves more than `MAX_LEVEL_INSTABILITY_DB` under tiny
//! parameter changes (a knife edge that each platform's libm can tip either
//! way), is re-drawn with the next seed (up to `MAX_SEED_TRIES`); a curated preset
//! that fails stops the run, to be fixed by hand. A drum kit's pads are
//! levelled one by one, each held alone on its key, and each must meet the
//! floor. The chosen Output and seed
//! of every preset are written to `src/presets/factory/levels.rs`, so the
//! library builds the same bank without rendering.
//!
//! `--target` prints the median RMS of the curated presets at Output 0 dB,
//! from which `TARGET_RMS` was chosen, and writes nothing.

use std::path::{Path, PathBuf};

use clap::Parser;
use corrosion::presets::factory::{
    curated_presets, drum_kits, experimental_entry, level, level_instability_db, level_pad,
    measure_floor, FactoryPreset, EXCITER_COUNT, MAX_LEVEL_INSTABILITY_DB, MAX_SEED_TRIES,
    OBJECT_COUNT,
};
use corrosion::Object;

#[derive(Parser)]
struct Args {
    /// Print the curated presets' median RMS at Output 0 dB and exit.
    #[arg(long)]
    target: bool,
}

/// A levelled preset, its seed try, and its floor result.
struct Levelled {
    entry: FactoryPreset,
    try_index: u32,
    result: Result<(), String>,
}

fn level_curated(mut entry: FactoryPreset) -> Levelled {
    let result = level(&mut entry.preset).check();
    Levelled {
        entry,
        try_index: 0,
        result,
    }
}

fn level_kit(mut entry: FactoryPreset) -> Levelled {
    let keys: Vec<u8> = entry
        .preset
        .kit
        .as_ref()
        .map_or_else(Vec::new, |kit| kit.pads.iter().map(|p| p.key).collect());
    let mut failures = Vec::new();
    for key in keys {
        if let Err(why) = level_pad(&mut entry.preset, key).check() {
            failures.push(format!("pad {key}: {why}"));
        }
    }
    let result = if failures.is_empty() {
        Ok(())
    } else {
        Err(failures.join("; "))
    };
    Levelled {
        entry,
        try_index: 0,
        result,
    }
}

fn level_experimental(exciter: i32, object: Object) -> Levelled {
    let mut last = None;
    for try_index in 0..MAX_SEED_TRIES {
        let mut entry = experimental_entry(exciter, object, try_index);
        let result = level(&mut entry.preset).check().and_then(|()| {
            let spread = level_instability_db(&entry.preset);
            if spread > MAX_LEVEL_INSTABILITY_DB {
                Err(format!(
                    "level moves {spread:.1} dB under tiny parameter changes (≤ {MAX_LEVEL_INSTABILITY_DB} dB)"
                ))
            } else {
                Ok(())
            }
        });
        let done = result.is_ok();
        last = Some(Levelled {
            entry,
            try_index,
            result,
        });
        if done {
            break;
        }
    }
    last.expect("at least one try")
}

/// Run `jobs` on all cores, keeping their order.
fn parallel<T: Send, R: Send>(jobs: Vec<T>, f: impl Fn(T) -> R + Sync) -> Vec<R> {
    let threads = std::thread::available_parallelism().map_or(4, |n| n.get());
    let chunk = jobs.len().div_ceil(threads).max(1);
    let mut jobs = jobs;
    let mut chunks = Vec::new();
    while !jobs.is_empty() {
        let rest = jobs.split_off(chunk.min(jobs.len()));
        chunks.push(std::mem::replace(&mut jobs, rest));
    }
    std::thread::scope(|scope| {
        let handles: Vec<_> = chunks
            .into_iter()
            .map(|chunk| scope.spawn(|| chunk.into_iter().map(&f).collect::<Vec<_>>()))
            .collect();
        handles
            .into_iter()
            .flat_map(|h| h.join().expect("levelling thread"))
            .collect()
    })
}

fn print_target() {
    let mut levels: Vec<(f32, String)> = parallel(curated_presets(), |mut entry| {
        entry.preset.output = 1.0;
        (measure_floor(&entry.preset).rms, entry.relative_path())
    });
    levels.sort_by(|a, b| a.0.total_cmp(&b.0));
    for (rms, path) in &levels {
        println!("{rms:.5}  {path}");
    }
    if !levels.is_empty() {
        let n = levels.len();
        let median = if n % 2 == 1 {
            levels[n / 2].0
        } else {
            0.5 * (levels[n / 2 - 1].0 + levels[n / 2].0)
        };
        println!("median RMS at Output 0 dB: {median:.5}");
    }
}

fn write_levels(levelled: &[Levelled], path: &Path) -> std::io::Result<()> {
    let mut source = String::from(
        "//! Stored levels, written by `cargo run --release --bin seed_presets`.\n\
         //! Do not edit. (relative path, seed try, Output)\n\n\
         use super::StoredLevel;\n\n\
         const LEVELS: &[(&str, u32, f32)] = &[\n",
    );
    for l in levelled {
        source += &format!(
            "    (\"{}\", {}, {:?}),\n",
            l.entry.relative_path(),
            l.try_index,
            l.entry.preset.output
        );
    }
    source += "];\n\n\
        /// The stored level of the preset at `path` (relative to `presets/factory`).\n\
        pub(super) fn stored(path: &str) -> Option<StoredLevel> {\n    \
            LEVELS\n        \
                .iter()\n        \
                .find(|(p, _, _)| *p == path)\n        \
                .map(|&(_, try_index, output)| StoredLevel { try_index, output })\n\
        }\n\n\
        const PAD_LEVELS: &[(&str, u8, f32)] = &[\n";
    for l in levelled {
        if let Some(kit) = &l.entry.preset.kit {
            for pad in &kit.pads {
                source += &format!(
                    "    (\"{}\", {}, {:?}),\n",
                    l.entry.relative_path(),
                    pad.key,
                    pad.level_db
                );
            }
        }
    }
    source += "];\n\n\
        /// The stored level (dB) of the pad on `key` of the kit at `path`.\n\
        pub(super) fn stored_pad(path: &str, key: u8) -> Option<f32> {\n    \
            PAD_LEVELS\n        \
                .iter()\n        \
                .find(|(p, k, _)| *p == path && *k == key)\n        \
                .map(|&(_, _, level)| level)\n\
        }\n";
    std::fs::write(path, source)
}

/// Remove the preset files (and then-empty folders) under `dir`.
fn clear(dir: &Path) -> std::io::Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let path = entry?.path();
        if path.is_dir() {
            clear(&path)?;
            if std::fs::read_dir(&path)?.next().is_none() {
                std::fs::remove_dir(&path)?;
            }
        } else if path
            .extension()
            .is_some_and(|e| e == corrosion::presets::PRESET_EXTENSION)
        {
            std::fs::remove_file(&path)?;
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    let args = Args::parse();
    if args.target {
        print_target();
        return Ok(());
    }
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    let mut levelled = parallel(curated_presets(), level_curated);
    levelled.extend(parallel(drum_kits(), level_kit));
    let pairs: Vec<(i32, Object)> = (1..=EXCITER_COUNT)
        .flat_map(|e| (0..OBJECT_COUNT).map(move |o| (e, Object::from_int(o))))
        .collect();
    levelled.extend(parallel(pairs, |(e, o)| level_experimental(e, o)));

    let failures: Vec<String> = levelled
        .iter()
        .filter_map(|l| {
            l.result.as_ref().err().map(|why| {
                format!(
                    "{} (seed try {}): {why}",
                    l.entry.relative_path(),
                    l.try_index
                )
            })
        })
        .collect();
    let redrawn: Vec<String> = levelled
        .iter()
        .filter(|l| l.try_index > 0)
        .map(|l| format!("{} → try {}", l.entry.relative_path(), l.try_index))
        .collect();
    if !redrawn.is_empty() {
        println!("Re-drawn seeds:\n  {}", redrawn.join("\n  "));
    }
    if !failures.is_empty() {
        eprintln!(
            "Presets below the quality floor, nothing written:\n  {}",
            failures.join("\n  ")
        );
        std::process::exit(1);
    }

    let dir = root.join("presets/factory");
    std::fs::create_dir_all(&dir)?;
    clear(&dir)?;
    for l in &levelled {
        l.entry.preset.save(l.entry.path_under(&dir))?;
    }
    write_levels(&levelled, &root.join("src/presets/factory/levels.rs"))?;
    println!(
        "Seeded {} factory presets in {}",
        levelled.len(),
        dir.display()
    );
    Ok(())
}
