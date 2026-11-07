#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RandomizerMode {
    #[default]
    Safe,
    Object,
    Damage,
    Full,
}

impl RandomizerMode {
    pub fn name(self) -> &'static str {
        match self {
            RandomizerMode::Safe => "Safe",
            RandomizerMode::Object => "Object",
            RandomizerMode::Damage => "Damage",
            RandomizerMode::Full => "Full",
        }
    }
}

pub fn safe_ranges() -> ParamRanges {
    ParamRanges {
        size: (0.5, 1.5),
        rust: (0.0, 0.5),
        damage: (0.0, 0.5),
        drive: (0.0, 0.5),
        width: (0.3, 0.8),
        body: (0.0, 2.5),
        output: (0.5, 1.0),
    }
}

pub fn object_ranges() -> ParamRanges {
    ParamRanges {
        size: (0.25, 2.0),
        rust: (0.0, 0.0),
        damage: (0.0, 0.0),
        drive: (0.0, 0.0),
        width: (0.5, 0.5),
        body: (0.0, 0.0),
        output: (0.7, 0.7),
    }
}

pub fn damage_ranges() -> ParamRanges {
    ParamRanges {
        size: (0.8, 1.2),
        rust: (0.0, 1.0),
        damage: (0.0, 1.0),
        drive: (0.0, 0.3),
        width: (0.5, 0.5),
        body: (0.0, 0.0),
        output: (0.7, 0.7),
    }
}

pub fn full_ranges() -> ParamRanges {
    ParamRanges {
        size: (0.25, 2.0),
        rust: (0.0, 1.0),
        damage: (0.0, 1.0),
        drive: (0.0, 1.0),
        width: (0.0, 1.0),
        body: (0.0, 5.0),
        output: (0.5, 1.0),
    }
}

pub struct ParamRanges {
    pub size: (f32, f32),
    pub rust: (f32, f32),
    pub damage: (f32, f32),
    pub drive: (f32, f32),
    pub width: (f32, f32),
    pub body: (f32, f32),
    pub output: (f32, f32),
}

pub fn ranges_for_mode(mode: RandomizerMode) -> ParamRanges {
    match mode {
        RandomizerMode::Safe => safe_ranges(),
        RandomizerMode::Object => object_ranges(),
        RandomizerMode::Damage => damage_ranges(),
        RandomizerMode::Full => full_ranges(),
    }
}

pub fn random_value(min: f32, max: f32, t: f32) -> f32 {
    min + t * (max - min)
}

pub fn randomize_params(mode: RandomizerMode, t_values: &[f32]) -> RandomizedParams {
    let ranges = ranges_for_mode(mode);

    RandomizedParams {
        size: random_value(ranges.size.0, ranges.size.1, t_values[0]),
        rust: random_value(ranges.rust.0, ranges.rust.1, t_values[1]),
        damage: random_value(ranges.damage.0, ranges.damage.1, t_values[2]),
        drive: random_value(ranges.drive.0, ranges.drive.1, t_values[3]),
        width: random_value(ranges.width.0, ranges.width.1, t_values[4]),
        body: random_value(ranges.body.0, ranges.body.1, t_values[5]),
        output: random_value(ranges.output.0, ranges.output.1, t_values[6]),
    }
}

pub struct RandomizedParams {
    pub size: f32,
    pub rust: f32,
    pub damage: f32,
    pub drive: f32,
    pub width: f32,
    pub body: f32,
    pub output: f32,
}

pub fn mutate_value(current: f32, amount: f32, t: f32) -> f32 {
    let jitter = (t - 0.5) * 2.0 * amount;
    (current + jitter).clamp(0.0, 1.0)
}

pub struct SafetyConstraints;

impl SafetyConstraints {
    pub fn is_safe_patch(params: &RandomizedParams) -> bool {
        if params.output < 0.01 {
            return false;
        }

        if params.damage > 0.8 && params.drive > 0.8 {
            return false;
        }

        if !(0.1..=3.0).contains(&params.size) {
            return false;
        }

        true
    }

    pub fn clamp_to_safe_ranges(params: &mut RandomizedParams) {
        params.size = params.size.clamp(0.25, 2.0);
        params.rust = params.rust.clamp(0.0, 1.0);
        params.damage = params.damage.clamp(0.0, 1.0);
        params.drive = params.drive.clamp(0.0, 1.0);
        params.width = params.width.clamp(0.0, 1.0);
        params.body = params.body.clamp(0.0, 5.0);
        params.output = params.output.clamp(0.1, 1.2);
    }
}

impl RandomizerMode {
    pub fn from_int(v: i32) -> Self {
        match v {
            1 => RandomizerMode::Object,
            2 => RandomizerMode::Damage,
            3 => RandomizerMode::Full,
            _ => RandomizerMode::Safe,
        }
    }

    pub fn to_int(self) -> i32 {
        match self {
            RandomizerMode::Safe => 0,
            RandomizerMode::Object => 1,
            RandomizerMode::Damage => 2,
            RandomizerMode::Full => 3,
        }
    }
}

/// How many times `randomize` redraws a patch that fails the safety
/// constraints before clamping it into the safe ranges instead.
const MAX_REDRAWS: usize = 16;

/// A random patch for `mode`, drawn from `noise`: redrawn while it fails
/// `SafetyConstraints::is_safe_patch`, and clamped into the safe ranges if
/// it still fails after `MAX_REDRAWS` draws (`h05-expose-missing-gui-controls`).
/// BODY is on the parameter's 0–5 scale.
pub fn randomize(
    mode: RandomizerMode,
    noise: &mut crate::dsp::utils::noise::Noise,
) -> RandomizedParams {
    let mut draw = || {
        let t: [f32; 7] = std::array::from_fn(|_| noise.unit());
        randomize_params(mode, &t)
    };
    let mut patch = draw();
    for _ in 0..MAX_REDRAWS {
        if SafetyConstraints::is_safe_patch(&patch) {
            return patch;
        }
        patch = draw();
    }
    if !SafetyConstraints::is_safe_patch(&patch) {
        SafetyConstraints::clamp_to_safe_ranges(&mut patch);
    }
    patch
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp::utils::noise::Noise;

    #[test]
    fn safe_mode_stays_in_its_ranges() {
        let mut noise = Noise::new(7);
        for _ in 0..100 {
            let p = randomize(RandomizerMode::Safe, &mut noise);
            for (name, value, lo, hi) in [
                ("size", p.size, 0.5, 1.5),
                ("rust", p.rust, 0.0, 0.5),
                ("damage", p.damage, 0.0, 0.5),
                ("drive", p.drive, 0.0, 0.5),
                ("width", p.width, 0.3, 0.8),
                ("body", p.body, 0.0, 2.5),
                ("output", p.output, 0.5, 1.0),
            ] {
                assert!(
                    (lo..=hi).contains(&value),
                    "Safe {name} {value} outside {lo}..{hi}"
                );
            }
        }
    }

    #[test]
    fn full_mode_patches_are_safe() {
        let mut noise = Noise::new(11);
        for _ in 0..1000 {
            let p = randomize(RandomizerMode::Full, &mut noise);
            assert!(SafetyConstraints::is_safe_patch(&p));
        }
    }

    #[test]
    fn draws_differ() {
        let mut noise = Noise::new(3);
        let (a, b) = (
            randomize(RandomizerMode::Full, &mut noise),
            randomize(RandomizerMode::Full, &mut noise),
        );
        assert!(
            a.size != b.size || a.rust != b.rust || a.damage != b.damage || a.body != b.body,
            "two Full draws came out identical"
        );
    }
}
