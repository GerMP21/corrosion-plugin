//! Implicit (zero-delay) exciter contact.
//!
//! An exciter whose force depends on the resonator's displacement or velocity
//! used to read them from the previous sample. Above a small loop gain that
//! one-sample lag turns damping into gain and the contact runs away. With
//! `ModalResonator::contact_prediction`, this sample's displacement is a known
//! function of this sample's force, so the contact law becomes `F = h(F)`,
//! solved here. This is the zero-delay feedback idea the ladder filter already
//! uses (`stabilize-mallet-and-grinding-contact`).

/// Most iterations `solve_force` takes. Each iteration at least halves the
/// bracket every other step, so this bounds the per-sample cost; the result
/// is within the bracket even if it isn't fully converged.
const MAX_ITERATIONS: usize = 40;

/// Solve `F = h(F)` for a contact law `h` that does not increase with `F`:
/// pushing harder never increases the penetration (or the slip) that
/// produces the force. There is then exactly one root, between 0 and `h(0)`.
///
/// Uses the Illinois variant of regula falsi, which needs no derivative and
/// never leaves the bracket, with a bisection step whenever an iteration
/// fails to halve the bracket (a stiff contact puts a kink in `h`).
/// Allocation-free.
pub fn solve_force(h: impl Fn(f32) -> f32) -> f32 {
    let h0 = h(0.0);
    if h0 == 0.0 || !h0.is_finite() {
        return 0.0;
    }
    // g(F) = F − h(F) increases with F; g(0) = −h0, and g(h0) has the
    // opposite sign because h(h0) is between 0 and h0.
    let g = |f: f32| f - h(f);
    let (mut lo, mut hi) = if h0 > 0.0 { (0.0, h0) } else { (h0, 0.0) };
    let (mut g_lo, mut g_hi) = (g(lo), g(hi));
    if g_lo >= 0.0 {
        return lo;
    }
    if g_hi <= 0.0 {
        return hi;
    }
    let mut side = 0i8;
    let mut bisect = false;
    for _ in 0..MAX_ITERATIONS {
        let width = hi - lo;
        if width <= 1e-7 * hi.abs().max(lo.abs()) {
            break;
        }
        let secant = (lo * g_hi - hi * g_lo) / (g_hi - g_lo);
        let x = if bisect || !(secant > lo && secant < hi) {
            0.5 * (lo + hi)
        } else {
            secant
        };
        let gx = g(x);
        if gx == 0.0 {
            return x;
        }
        if gx < 0.0 {
            lo = x;
            g_lo = gx;
            if side == -1 {
                g_hi *= 0.5;
            }
            side = -1;
        } else {
            hi = x;
            g_hi = gx;
            if side == 1 {
                g_lo *= 0.5;
            }
            side = 1;
        }
        bisect = hi - lo > 0.5 * width;
    }
    0.5 * (lo + hi)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Brute-force root of `F = h(F)` by f64 bisection.
    fn reference(h: impl Fn(f64) -> f64) -> f64 {
        let h0 = h(0.0);
        let (mut lo, mut hi) = if h0 >= 0.0 { (0.0, h0) } else { (h0, 0.0) };
        for _ in 0..200 {
            let mid = 0.5 * (lo + hi);
            if mid - h(mid) < 0.0 {
                lo = mid;
            } else {
                hi = mid;
            }
        }
        0.5 * (lo + hi)
    }

    fn close(a: f32, b: f64) -> bool {
        (a as f64 - b).abs() <= 1e-5 * b.abs().max(1e-6)
    }

    #[test]
    fn solves_a_hertzian_contact() {
        // F = k·(x − (D + C·F))^1.5 for a range of stiffness and compliance,
        // from a soft contact to one far stiffer than the resonator.
        for (k, c) in [(1.0, 0.01), (10.0, 0.5), (1_000.0, 2.0), (1e5, 10.0)] {
            let (x, d) = (0.2, 0.05);
            let law = |f: f32| k * (x - (d + c * f)).max(0.0).powf(1.5);
            let law64 =
                |f: f64| k as f64 * (x as f64 - (d as f64 + c as f64 * f)).max(0.0).powf(1.5);
            let got = solve_force(law);
            let want = reference(law64);
            assert!(close(got, want), "k {k} c {c}: {got} vs {want}");
        }
    }

    #[test]
    fn solves_a_smooth_friction_law() {
        // F = P·tanh((v_g − (v0 + Y·F)) / v_s): more force, less slip.
        for (p, y, v0) in [(0.24, 0.01, 0.0), (0.24, 5.0, 0.3), (2.0, 50.0, -0.6)] {
            let (vg, vs) = (0.4, 0.02);
            let law = |f: f32| p * ((vg - (v0 + y * f)) / vs).tanh();
            let law64 =
                |f: f64| p as f64 * ((vg as f64 - (v0 as f64 + y as f64 * f)) / vs as f64).tanh();
            let got = solve_force(law);
            let want = reference(law64);
            assert!(close(got, want), "p {p} y {y} v0 {v0}: {got} vs {want}");
        }
    }

    #[test]
    fn negative_force_and_no_contact() {
        // The resonator outruns the grinder: the friction force is negative.
        let law = |f: f32| 0.5 * ((0.1 - (0.4 + 2.0 * f)) / 0.02).tanh();
        let law64 = |f: f64| 0.5 * ((0.1 - (0.4 + 2.0 * f)) / 0.02).tanh();
        assert!(close(solve_force(law), reference(law64)));
        // The mallet is short of the surface: no force.
        assert_eq!(solve_force(|f: f32| (0.01 - (0.05 + f)).max(0.0)), 0.0);
    }
}
