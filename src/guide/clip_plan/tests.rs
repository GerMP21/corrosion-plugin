use super::*;
use std::collections::BTreeSet;

/// Where the manifest and the encoded clips live, from the repository.
const CLIPS_DIR: &str = "web/guide/public/clips";

#[test]
fn guide_clip_plan_covers_every_parameter() {
    let params = CorrosionParams::default();
    let host: Vec<String> = params
        .param_map()
        .into_iter()
        .map(|(id, _, _)| id)
        .collect();
    let plans = parameter_plans();
    let planned: BTreeSet<&str> = plans.iter().map(|p| p.id.as_str()).collect();
    let missing: Vec<&String> = host
        .iter()
        .filter(|id| !planned.contains(id.as_str()))
        .collect();
    let unknown: Vec<&str> = planned
        .iter()
        .copied()
        .filter(|id| !host.iter().any(|h| h == id))
        .collect();
    assert!(
        missing.is_empty() && unknown.is_empty(),
        "parameters without a clip plan or exclusion: {missing:?}; plan entries that aren't parameters: {unknown:?}"
    );
    assert_eq!(plans.len(), host.len(), "a parameter is planned twice");
    for p in &plans {
        if let Demo::Excluded { reason } = p.demo {
            assert!(
                !reason.trim().is_empty(),
                "`{}` is excluded without a reason",
                p.id
            );
        }
        for (point, reason) in &p.quiet {
            assert!(
                !reason.trim().is_empty(),
                "`{}` {point:?} is quiet without a reason",
                p.id
            );
        }
    }
    // Every planned parameter has its three clips.
    let refs: BTreeSet<String> = plan().into_iter().map(|c| c.reference).collect();
    for p in plans
        .iter()
        .filter(|p| !matches!(p.demo, Demo::Excluded { .. }))
    {
        for point in Point::ALL {
            let r = format!("param:{}:{}", p.id, point.key());
            assert!(refs.contains(&r), "no clip {r}");
        }
    }
}

#[test]
fn guide_clip_manifest_is_complete_and_audible() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(CLIPS_DIR);
    let regenerate = "run `scripts/build-guide-clips.sh`";
    let text = std::fs::read_to_string(root.join("clips.json"))
        .unwrap_or_else(|_| panic!("{CLIPS_DIR}/clips.json is missing: {regenerate}"));
    let manifest: Value = serde_json::from_str(&text).expect("clips.json is JSON");
    assert_eq!(
        manifest["fingerprint"].as_str(),
        Some(fingerprint().as_str()),
        "the clip plan changed since the clips were rendered: {regenerate}"
    );
    let clips = manifest["clips"].as_array().expect("a `clips` array");
    let mut problems = Vec::new();
    for spec in plan() {
        let Some(entry) = clips.iter().find(|c| c["ref"] == spec.reference.as_str()) else {
            problems.push(format!("{} is not in the manifest", spec.reference));
            continue;
        };
        let file = entry["file"].as_str().unwrap_or_default();
        if !root.join(file).is_file() {
            problems.push(format!("{} has no file ({file})", spec.reference));
        }
        let rms_db = entry["rms_db"].as_f64().unwrap_or(f64::NEG_INFINITY);
        if spec.quiet.is_none() && rms_db < f64::from(SILENCE_DB) {
            problems.push(format!(
                "{} is silent ({rms_db:.1} dBFS RMS); declare it quiet with a reason if that's expected",
                spec.reference
            ));
        }
    }
    assert!(problems.is_empty(), "{}: {regenerate}", problems.join("; "));
}

#[test]
fn home_demos_cover_each_curated_folder_once() {
    let folders: Vec<&str> = HOME_DEMOS.iter().map(|d| d.0).collect();
    let mut expected: Vec<&str> = factory::CURATED_FOLDERS.to_vec();
    let mut got = folders.clone();
    expected.sort_unstable();
    got.sort_unstable();
    assert_eq!(got, expected, "one home demo per curated folder");
    let demos: Vec<ClipSpec> = plan()
        .into_iter()
        .filter(|c| c.set.starts_with("set:demos:"))
        .collect();
    for &(folder, name, root, phrase) in HOME_DEMOS {
        let clip = demos
            .iter()
            .find(|c| c.file == format!("demos/{folder}"))
            .expect("a demo clip per folder");
        assert_eq!(clip.preset.name, name);
        let ons: Vec<&OfflineNote> = clip
            .notes
            .iter()
            .filter(|n| matches!(n.kind, crate::offline::OfflineNoteKind::On { .. }))
            .collect();
        let mut first: Vec<u8> = ons
            .iter()
            .filter(|n| n.frame == 0)
            .map(|n| n.note)
            .collect();
        first.sort_unstable();
        let (expected, count): (Vec<u8>, usize) = match phrase {
            DemoPhrase::Notes => (vec![root], 4),
            DemoPhrase::Triads => (vec![root, root + 4, root + 7], 12),
            DemoPhrase::Held => (vec![root], 1),
        };
        assert_eq!(first, expected, "`{folder}` demo's first notes");
        assert_eq!(ons.len(), count, "`{folder}` demo's note count");
    }
}

#[test]
fn every_clip_renders_its_release_before_trimming() {
    for clip in plan() {
        let last_off = clip
            .notes
            .iter()
            .filter(|n| n.kind == crate::offline::OfflineNoteKind::Off)
            .map(|n| n.frame)
            .max()
            .expect("every clip releases its notes");
        let tail = clip.seconds - last_off as f32 / SAMPLE_RATE as f32;
        let expected = if clip.set.starts_with("set:demos:") {
            DEMO_RELEASE_TAIL
        } else {
            RELEASE_TAIL
        };
        assert!(
            tail >= expected - 1e-3,
            "{} ends {tail:.2} s after its last note-off",
            clip.reference
        );
    }
}

#[test]
fn trimming_cuts_where_the_tail_falls_silent() {
    let rate = SAMPLE_RATE as f32;
    // A tone decaying 20 dB a second for 2 s, then 6 s of silence.
    let mut frames: Vec<(f32, f32)> = (0..(rate * 8.0) as usize)
        .map(|i| {
            let t = i as f32 / rate;
            let x = if t < 2.0 {
                (t * 440.0 * std::f32::consts::TAU).sin() * 10f32.powf(-t)
            } else {
                0.0
            };
            (x, x)
        })
        .collect();
    trim_to_silence(&mut frames);
    let seconds = frames.len() as f32 / rate;
    assert!(
        (1.9..2.1).contains(&seconds),
        "the 2 s decay (to -40 dB) is kept, the silence cut: {seconds:.2} s"
    );
    assert!(frames.last().unwrap().0.abs() < 1e-6, "the cut is faded");
    // A tail that falls 60 dB under the peak is cut there.
    let mut long: Vec<(f32, f32)> = (0..(rate * 8.0) as usize)
        .map(|i| {
            let x = 10f32.powf(-(i as f32 / rate)) * if i % 2 == 0 { 1.0 } else { -1.0 };
            (x, x)
        })
        .collect();
    trim_to_silence(&mut long);
    let seconds = long.len() as f32 / rate;
    assert!(
        (3.0..3.1).contains(&seconds),
        "cut at -60 dB: {seconds:.2} s"
    );
    // Silence is cut to the fade.
    let mut silent = vec![(0.0, 0.0); 48_000];
    trim_to_silence(&mut silent);
    assert_eq!(silent.len(), (rate * TRIM_FADE) as usize);
}

/// A recipe for the tests: Low Sheet Bow with more Rust and a darker filter.
const SAMPLE_RECIPE: Recipe = Recipe {
    slug: "rusted-bass",
    folder: "bass",
    preset: "Low Sheet Bow",
    root: NOTE - 12,
    phrase: DemoPhrase::Notes,
    changes: &[("rust", 2.0), ("filter_cutoff", 2_000.0)],
};

#[test]
fn a_recipe_renders_before_and_after_from_its_preset() {
    let bank = factory::bank();
    let clips = recipe_clips(&[SAMPLE_RECIPE], &bank);
    assert_eq!(clips.len(), 2);
    let (before, after) = (&clips[0], &clips[1]);
    assert_eq!(before.reference, "set:recipes:rusted-bass:before");
    assert_eq!(after.reference, "set:recipes:rusted-bass:after");
    assert_eq!(before.file, "recipes/rusted-bass/before");
    assert_eq!(before.set, after.set, "one gain, so the comparison is fair");
    assert!(before.is_reference && !after.is_reference);
    let preset = &bank
        .iter()
        .find(|e| e.folder == "bass" && e.preset.name == "Low Sheet Bow")
        .expect("the preset")
        .preset;
    let json = |p: &Preset| serde_json::to_value(p).expect("presets serialize");
    assert_eq!(
        json(&before.preset),
        json(preset),
        "before is the preset as it ships"
    );
    assert_eq!(
        json(&after.preset),
        json(&with_values(preset, SAMPLE_RECIPE.changes)),
        "after is the preset with exactly the listed changes"
    );
    assert_eq!(before.notes, after.notes);
    assert_eq!(before.notes[0].note, NOTE - 12);
}

#[test]
fn a_recipe_must_name_a_real_preset_parameters_and_values() {
    let bank = factory::bank();
    assert_eq!(check_recipe(&SAMPLE_RECIPE, &bank), Ok(()));
    let bad = |r: Recipe| check_recipe(&r, &bank).expect_err("should be refused");
    assert!(bad(Recipe {
        preset: "Low Sheet Bowl",
        ..SAMPLE_RECIPE
    })
    .contains("no factory preset bass/Low Sheet Bowl"));
    assert!(bad(Recipe {
        changes: &[("rusty", 1.0)],
        ..SAMPLE_RECIPE
    })
    .contains("`rusty` is not a parameter"));
    assert!(bad(Recipe {
        changes: &[("rust", 50.0)],
        ..SAMPLE_RECIPE
    })
    .contains("`rust` = 50 is outside its range"));
    assert!(bad(Recipe {
        changes: &[],
        ..SAMPLE_RECIPE
    })
    .contains("changes nothing"));
}

#[test]
fn every_recipe_in_the_plan_is_valid() {
    let bank = factory::bank();
    for recipe in RECIPES {
        assert_eq!(
            check_recipe(recipe, &bank),
            Ok(()),
            "recipe `{}`",
            recipe.slug
        );
    }
    let slugs: BTreeSet<&str> = RECIPES.iter().map(|r| r.slug).collect();
    assert_eq!(slugs.len(), RECIPES.len(), "recipe slugs are unique");
}

/// Level, attack brightness and ring length of one note.
fn note_features(frames: &[(f32, f32)]) -> (f32, f32, f32) {
    let mono: Vec<f32> = frames.iter().map(|(l, r)| 0.5 * (l + r)).collect();
    let db = |x: f32| 20.0 * x.max(1e-9).log10();
    let rms = (mono.iter().map(|x| x * x).sum::<f32>() / mono.len() as f32).sqrt();
    let peak = mono.iter().fold(0.0f32, |m, x| m.max(x.abs()));
    let onset = mono.iter().position(|x| x.abs() > 0.05 * peak).unwrap_or(0);
    // Attack brightness: zero-crossing rate of the first 30 ms's
    // first difference over its own, a cheap centroid proxy in hertz.
    let attack = &mono[onset..(onset + 1440).min(mono.len())];
    let energy: f32 = attack.iter().map(|x| x * x).sum();
    let diff: f32 = attack.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
    let bright =
        (diff / energy.max(1e-12)).sqrt() * SAMPLE_RATE as f32 / (2.0 * std::f32::consts::PI);
    // Ring: the last 10 ms window within 40 dB of the loudest window.
    let windows: Vec<f32> = mono
        .chunks(480)
        .map(|w| (w.iter().map(|x| x * x).sum::<f32>() / w.len() as f32).sqrt())
        .collect();
    let loudest = windows.iter().cloned().fold(0.0, f32::max);
    let last = windows
        .iter()
        .rposition(|w| *w > loudest * 0.01)
        .unwrap_or(0);
    (db(rms), bright, (last + 1) as f32 * 0.01)
}

/// Prints, for every exciter and object slot, one note at the slot's
/// minimum and maximum on its clip patch: RMS (dB), attack brightness (Hz,
/// a centroid proxy) and ring length (s, to −40 dB). The guide's accuracy
/// pass (`write-user-guide-content` 2.3) compares it with the help texts.
#[test]
#[ignore = "measurement table; run with --ignored --nocapture"]
fn print_slot_effects() {
    let clips = plan();
    let one_note = vec![
        OfflineNote::on(0, NOTE, 100.0 / 127.0),
        OfflineNote::off(2 * SAMPLE_RATE as usize, NOTE),
    ];
    let render = |reference: String| {
        let clip = clips.iter().find(|c| c.reference == reference)?;
        Some(note_features(&render_preset_at_tempo(
            &clip.preset,
            &one_note,
            SAMPLE_RATE,
            4.0,
            None,
        )))
    };
    // Mean absolute difference of the first second's octave-band levels
    // (63 Hz to 16 kHz, each band's energy from a direct DFT of
    // 4096-sample frames), so a control that only reshapes the spectrum
    // still shows.
    let bands = |frames: &[(f32, f32)]| -> Vec<f32> {
        let mono: Vec<f32> = frames
            .iter()
            .take(SAMPLE_RATE as usize)
            .map(|(l, r)| 0.5 * (l + r))
            .collect();
        let n = 4096;
        let mut energy = [1e-12f32; 9];
        for frame in mono.chunks_exact(n) {
            for (b, e) in energy.iter_mut().enumerate() {
                let (lo, hi) = (44.0 * 2f32.powi(b as i32), 88.0 * 2f32.powi(b as i32));
                let (k0, k1) = (
                    (lo * n as f32 / SAMPLE_RATE as f32) as usize,
                    (hi * n as f32 / SAMPLE_RATE as f32) as usize,
                );
                for k in k0.max(1)..=k1 {
                    let (mut re, mut im) = (0.0f32, 0.0f32);
                    for (i, x) in frame.iter().enumerate() {
                        let w = 2.0 * std::f32::consts::PI * (k * i) as f32 / n as f32;
                        re += x * w.cos();
                        im -= x * w.sin();
                    }
                    *e += re * re + im * im;
                }
            }
        }
        energy.iter().map(|e| 10.0 * e.log10()).collect()
    };
    let shape = |reference: String| {
        let clip = clips.iter().find(|c| c.reference == reference)?;
        Some(bands(&render_preset_at_tempo(
            &clip.preset,
            &one_note,
            SAMPLE_RATE,
            1.0,
            None,
        )))
    };
    println!(
        "{:26} {:>26} {:>26} {:>9}",
        "parameter", "min: dB / Hz / s", "max: dB / Hz / s", "bands dB"
    );
    for plan in parameter_plans() {
        let (Some(lo), Some(hi)) = (
            render(format!("param:{}:min", plan.id)),
            render(format!("param:{}:max", plan.id)),
        ) else {
            continue;
        };
        let (a, b) = (
            shape(format!("param:{}:min", plan.id)).unwrap_or_default(),
            shape(format!("param:{}:max", plan.id)).unwrap_or_default(),
        );
        let level = (lo.0 - hi.0).abs();
        let spread = a
            .iter()
            .zip(&b)
            .map(|(x, y)| ((x - y).abs() - level).abs())
            .sum::<f32>()
            / a.len().max(1) as f32;
        println!(
            "{:26} {:8.1} {:8.0} {:6.2}   {:8.1} {:8.0} {:6.2} {:9.2}",
            plan.id, lo.0, lo.1, lo.2, hi.0, hi.1, hi.2, spread
        );
    }
}

/// The numbers behind each recipe's text (task 2.8): level, brightness and
/// the level after the last note-off, before and after.
#[test]
#[ignore = "measurement table; run with --ignored --nocapture"]
fn print_recipe_measurements() {
    let bank = factory::bank();
    let db = |x: f32| 20.0 * x.max(1e-9).log10();
    let rms = |s: &[(f32, f32)]| {
        (s.iter().map(|(l, r)| l * l + r * r).sum::<f32>() / (2 * s.len().max(1)) as f32).sqrt()
    };
    // Brightness proxy: RMS of the first difference over RMS, in Hz
    // (for a sine, rate · |2 sin(πf/rate)| / 2π ≈ f).
    let centroid = |s: &[(f32, f32)]| {
        let mono: Vec<f32> = s.iter().map(|(l, r)| l + r).collect();
        let d: f32 = mono.windows(2).map(|w| (w[1] - w[0]).powi(2)).sum();
        let e: f32 = mono.iter().map(|x| x * x).sum();
        (d / e.max(1e-12)).sqrt() * SAMPLE_RATE as f32 / std::f32::consts::TAU
    };
    // Movement: standard deviation, in dB, of 20 ms RMS over 0.5–3 s.
    let wobble = |s: &[(f32, f32)]| {
        let w = SAMPLE_RATE as usize / 50;
        let start = (SAMPLE_RATE as usize / 2).min(s.len());
        let end = (SAMPLE_RATE as usize * 3).min(s.len());
        let levels: Vec<f32> = s[start..end]
            .chunks(w)
            .map(|c| 20.0 * rms(c).max(1e-9).log10())
            .collect();
        let mean = levels.iter().sum::<f32>() / levels.len().max(1) as f32;
        (levels.iter().map(|x| (x - mean).powi(2)).sum::<f32>() / levels.len().max(1) as f32).sqrt()
    };
    println!("| clip | RMS dB | bright Hz | attack bright Hz | wobble dB | last 0.5 s dB | max after off dB | max before off dB |");
    for clip in recipe_clips(RECIPES, &bank) {
        let out = render_clip(&clip);
        let off = clip
            .notes
            .iter()
            .filter(|n| matches!(n.kind, crate::offline::OfflineNoteKind::Off))
            .map(|n| n.frame)
            .max()
            .unwrap_or(0);
        let peak = |s: &[(f32, f32)]| {
            s.iter()
                .map(|(l, r)| l.abs().max(r.abs()))
                .fold(0.0, f32::max)
        };
        let half = SAMPLE_RATE as usize / 2;
        let before_off = &out[off.saturating_sub(half).min(out.len())..off.min(out.len())];
        let after_off = &out[off.min(out.len())..(off + half).min(out.len())];
        let tail = &out[out.len().saturating_sub(half)..];
        println!(
            "| {} | {:.1} | {:.0} | {:.0} | {:.1} | {:.1} | {:.1} | {:.1} |",
            clip.reference,
            db(rms(&out)),
            centroid(&out),
            {
                let w = SAMPLE_RATE as usize * 3 / 100;
                let ons: Vec<usize> = clip
                    .notes
                    .iter()
                    .filter(|n| !matches!(n.kind, crate::offline::OfflineNoteKind::Off))
                    .map(|n| n.frame)
                    .collect();
                ons.iter()
                    .map(|&f| centroid(&out[f.min(out.len())..(f + w).min(out.len())]))
                    .sum::<f32>()
                    / ons.len().max(1) as f32
            },
            wobble(&out),
            db(rms(tail)),
            db(peak(after_off)),
            db(peak(before_off)),
        );
    }
}
