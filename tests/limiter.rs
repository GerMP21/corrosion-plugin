use corrosion::{apply_output_limiter, LIMITER_THRESHOLD};

#[test]
fn output_limiter_hard_clamps_to_threshold() {
    let samples = [
        -10.0,
        -1.0,
        -LIMITER_THRESHOLD,
        0.0,
        LIMITER_THRESHOLD,
        1.0,
        10.0,
    ];

    for sample in samples {
        let limited = apply_output_limiter(sample, LIMITER_THRESHOLD);
        assert!(limited.abs() <= LIMITER_THRESHOLD);
        let low = apply_output_limiter(sample, 0.5);
        assert!(low.abs() <= 0.5);
    }
}

#[test]
fn limiter_threshold_matches_negative_point_three_dbfs() {
    assert!((LIMITER_THRESHOLD - 0.9661).abs() < f32::EPSILON);
}
