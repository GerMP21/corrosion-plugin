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
fn every_clip_leaves_time_for_the_release() {
    for clip in plan() {
        let last_off = clip
            .notes
            .iter()
            .filter(|n| n.kind == crate::offline::OfflineNoteKind::Off)
            .map(|n| n.frame)
            .max()
            .expect("every clip releases its notes");
        let tail = clip.seconds - last_off as f32 / SAMPLE_RATE as f32;
        assert!(
            tail >= RELEASE_TAIL - 1e-3,
            "{} ends {tail:.2} s after its last note-off",
            clip.reference
        );
    }
}
