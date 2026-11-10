//! Sound-neutral proof for `m10-m18-tidy-dead-code-and-docs`: a render hash
//! of damaged, stereo friction and hit notes, compared by hand against a
//! clean worktree (hashes aren't portable across libm, so not asserted).
//! Run with `-- --ignored --nocapture`.

use corrosion::offline::{render_preset, OfflineNote};
use corrosion::presets::Preset;

fn hash(frames: &[(f32, f32)]) -> u64 {
    frames
        .iter()
        .flat_map(|&(l, r)| [l, r])
        .fold(0xcbf2_9ce4_8422_2325, |h, s| {
            (h ^ u64::from(s.to_bits())).wrapping_mul(0x0100_0000_01b3)
        })
}

#[test]
#[ignore]
fn print_tidy_render_hashes() {
    for (exciter, name) in [(1, "bow"), (2, "hand strike"), (4, "hard mallet")] {
        let mut preset = Preset::from_params("Tidy", &corrosion::CorrosionParams::default());
        preset.exciter = exciter;
        preset.damage = 6.0;
        preset.rust = 1.5;
        preset.width = 1.5;
        preset.extra.internal_rattle = 0.8;
        let notes = [
            OfflineNote::on(0, 48, 1.0),
            OfflineNote::on(2_400, 55, 0.8),
            OfflineNote::off(48_000, 48),
            OfflineNote::off(52_000, 55),
        ];
        let frames = render_preset(&preset, &notes, 48_000, 2.0);
        println!("HASH {name}: {:#018x}", hash(&frames));
    }
}
