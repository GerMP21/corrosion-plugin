use super::*;

/// Prints every sound-shaping control's response in its context: the
/// largest level, ring, band-shape and width difference between its
/// minimum, middle and maximum, and whether it reaches the floor.
#[test]
#[ignore = "measurement table; run with --release -- --ignored --nocapture"]
fn print_control_responses() {
    println!(
        "{:26} {:>8} {:>7} {:>8} {:>8} {:>8}  acts",
        "parameter", "level", "ring", "bands", "width", "env"
    );
    // CONTROLS=id,id limits the table to those controls.
    let only: Option<Vec<String>> = std::env::var("CONTROLS")
        .ok()
        .map(|v| v.split(',').map(str::to_string).collect());
    let rows: Vec<(String, Response)> = match &only {
        Some(ids) => parameter_plans()
            .into_iter()
            .filter(|p| ids.contains(&p.id))
            .map(|p| (p.id.clone(), response(&measure_points(&p))))
            .collect(),
        None => responses(),
    };
    for (id, r) in rows {
        println!(
            "{:26} {:8.2} {:6.0}% {:8.2} {:8.2} {:8.2}  {}",
            id,
            r.level_db,
            r.ring * 100.0,
            r.bands_db,
            r.width_db,
            r.envelope_db,
            if r.acts() { "yes" } else { "NO" }
        );
    }
}

#[test]
fn symmetric_ends_are_caught_by_the_middle() {
    let flat = |level: f32| Features {
        level_db: level,
        ring: 1.0,
        bands: vec![level; 9],
        width_db: -40.0,
        envelope: vec![level; 10],
    };
    // Equal ends, a different middle (Strike Position's shape).
    let points = vec![vec![flat(-20.0)], vec![flat(-14.0)], vec![flat(-20.0)]];
    assert!(response(&points).acts());
    let same = vec![vec![flat(-20.0)]; 3];
    assert!(!response(&same).acts());
}

#[test]
fn a_velocity_control_counts_by_the_gap_it_changes() {
    let at = |level: f32| Features {
        level_db: level,
        ring: 1.0,
        bands: vec![level; 9],
        width_db: -40.0,
        envelope: vec![level; 10],
    };
    // The hard note stays put; the soft one moves 3 dB.
    let points = vec![
        vec![at(-26.0), at(-14.0)],
        vec![at(-23.0), at(-14.0)],
        vec![at(-20.0), at(-14.0)],
    ];
    assert!(response(&points).acts());
}

/// Spec "Every sound-shaping control changes its note": every control
/// reaches the floor in its context, and a control that stops acting fails
/// here by name (`fix-inert-controls`). About 70 s in the test profile.
#[test]
#[ignore = "red until fix-inert-controls task 2 makes the 20 listed controls act"]
fn every_sound_shaping_control_changes_its_note() {
    let inert: Vec<String> = responses()
        .into_iter()
        .filter(|(id, r)| !r.acts() && !UNDER_FLOOR.iter().any(|(u, _)| u == id))
        .map(|(id, r)| {
            format!(
                "{id} (level {:.2} dB, ring {:.0} %, bands {:.2} dB, width {:.2} dB, envelope {:.2} dB)",
                r.level_db,
                r.ring * 100.0,
                r.bands_db,
                r.width_db,
                r.envelope_db
            )
        })
        .collect();
    assert!(
        inert.is_empty(),
        "{} controls don't change their note:\n{}",
        inert.len(),
        inert.join("\n")
    );
}
