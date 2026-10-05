//! Regression guard for review finding C1: at the default 20 kHz filter
//! cutoff the post chain used to latch to a DC rail on the first note. Drive
//! the chain the way `Corrosion::process` does, using the plugin's default
//! parameter values, and check the output is bounded and returns to zero.

use corrosion::dsp::{PostProcessingChain, PostQualityMode, SpaceMode};
use corrosion::CorrosionParams;

fn default_chain(sample_rate: f32) -> PostProcessingChain {
    let params = CorrosionParams::default();
    let mut chain = PostProcessingChain::new();
    chain.set_sample_rate(sample_rate);
    chain.set_quality_mode(PostQualityMode::Normal);
    chain.set_filter_params(
        params.filter_cutoff.value(),
        params.filter_resonance.value(),
        params.component_tolerance.value(),
    );
    chain.set_drive_params(
        params.drive_amount.value(),
        params.bias_starvation.value(),
        params.chaos_depth.value(),
    );
    chain.set_body_params(
        params.chassis_material.value(),
        params.chassis_volume.value(),
    );
    chain.set_spread_params(
        params.spread_width.value(),
        params.listener_proximity.value(),
    );
    // Space off so the tail measurement isolates the filter/drive/body path.
    chain.set_space_mode(SpaceMode::Off);
    chain.set_space_amount(params.space_amount.value());
    chain.set_clipper_params(params.analog_ceiling.value(), params.diode_softness.value());
    chain
}

#[test]
fn default_post_chain_has_no_dc_after_a_note() {
    let sample_rate = 48_000.0_f32;
    let mut chain = default_chain(sample_rate);
    assert_eq!(CorrosionParams::default().filter_cutoff.value(), 20_000.0);

    let burst = (sample_rate * 0.5) as usize;
    let silence = sample_rate as usize * 2;
    let tail = (sample_rate * 0.1) as usize;
    let mut tail_sum = [0.0_f64; 2];
    let mut peak = 0.0_f32;

    for i in 0..burst + silence {
        // Decaying 220 Hz "note" with a slight stereo offset.
        let x = if i < burst {
            let t = i as f32 / sample_rate;
            0.5 * (-6.0 * t).exp() * (2.0 * std::f32::consts::PI * 220.0 * t).sin()
        } else {
            0.0
        };
        let (left, right) = chain.process(x, 0.9 * x);
        assert!(
            left.is_finite() && right.is_finite(),
            "non-finite output at {i}"
        );
        peak = peak.max(left.abs()).max(right.abs());
        if i >= burst + silence - tail {
            tail_sum[0] += left as f64;
            tail_sum[1] += right as f64;
        }
    }

    assert!(peak <= 1.0, "output exceeded full scale: {peak}");
    for (channel, sum) in tail_sum.iter().enumerate() {
        let mean = sum / tail as f64;
        assert!(
            mean.abs() < 1e-3,
            "channel {channel} holds a DC offset of {mean} after the note ended"
        );
    }
}
