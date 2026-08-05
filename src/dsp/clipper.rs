//! The core waveshaping algorithm, kept independent of plugin/param glue so it can be unit
//! tested and benchmarked without a host or a `Params` struct.

/// Clips a single sample.
///
/// - `threshold` is the linear (not dB) level at which clipping engages. Must be > 0.
/// - `softness` blends between a hard brick-wall clip (0.0) and a `tanh`-based soft saturation
///   (1.0) at the same threshold. Because `tanh` approaches its asymptote gradually, higher
///   softness settings top out below `threshold` rather than hitting it exactly — this is the
///   expected, audibly softer character of the effect.
#[inline]
pub fn clip_sample(x: f32, threshold: f32, softness: f32) -> f32 {
    let hard = x.clamp(-threshold, threshold);
    let soft = threshold * (x / threshold).tanh();
    hard + (soft - hard) * softness
}

/// Combines input gain with [`clip_sample`]. `gain` is linear.
#[inline]
pub fn process_sample(x: f32, gain: f32, threshold: f32, softness: f32) -> f32 {
    clip_sample(x * gain, threshold, softness)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hard_clip_matches_clamp_at_zero_softness() {
        let threshold = 0.5;
        for x in [-2.0_f32, -0.6, -0.5, -0.1, 0.0, 0.1, 0.5, 0.6, 2.0] {
            let y = clip_sample(x, threshold, 0.0);
            assert_eq!(y, x.clamp(-threshold, threshold));
        }
    }

    #[test]
    fn soft_clip_matches_tanh_at_full_softness() {
        let threshold = 0.5;
        for x in [-2.0_f32, -0.6, 0.0, 0.3, 2.0] {
            let y = clip_sample(x, threshold, 1.0);
            let expected = threshold * (x / threshold).tanh();
            assert!((y - expected).abs() < 1e-6);
        }
    }

    #[test]
    fn signal_within_threshold_and_zero_softness_is_unaffected() {
        let threshold = 0.8;
        for x in [-0.7_f32, -0.1, 0.0, 0.3, 0.79] {
            assert_eq!(clip_sample(x, threshold, 0.0), x);
        }
    }

    #[test]
    fn output_never_exceeds_threshold() {
        let threshold = 0.3;
        for softness in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
            for x in [-10.0_f32, -1.0, 1.0, 10.0] {
                let y = clip_sample(x, threshold, softness);
                assert!(y.abs() <= threshold + 1e-6, "y={y} threshold={threshold}");
            }
        }
    }

    #[test]
    fn blend_stays_between_hard_and_soft_curves() {
        let threshold = 0.6;
        for x in [-3.0_f32, -1.0, -0.1, 0.1, 1.0, 3.0] {
            let hard = x.clamp(-threshold, threshold);
            let soft = threshold * (x / threshold).tanh();
            let (lo, hi) = if hard <= soft { (hard, soft) } else { (soft, hard) };
            for softness in [0.0_f32, 0.25, 0.5, 0.75, 1.0] {
                let y = clip_sample(x, threshold, softness);
                assert!(y >= lo - 1e-6 && y <= hi + 1e-6, "y={y} lo={lo} hi={hi}");
            }
        }
    }

    #[test]
    fn gain_is_applied_before_clipping() {
        let threshold = 0.5;
        // Below threshold with unity gain, doubled by gain=2.0 it should now clip.
        let y = process_sample(0.4, 2.0, threshold, 0.0);
        assert_eq!(y, threshold);
    }
}
