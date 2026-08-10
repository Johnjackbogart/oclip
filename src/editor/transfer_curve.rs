//! A static input->output transfer curve, independent of whatever audio happens to be playing.
//! Turning Softness bends the knee from a sharp corner (0%, hard clip) to a smooth curve (100%,
//! tanh saturation) — this makes that shape directly visible, the way GClip's scope does, rather
//! than relying on spotting it in a live waveform.

use crate::dsp;

/// Resolution of the plotted curve. `pub(crate)` so tests elsewhere (this file's own, and
/// `editor::mod`'s layout tests) can recognize the curve's painted shape by its exact point count
/// rather than guessing.
pub(crate) const CURVE_SEGMENTS: usize = 64;

pub struct TransferCurve {
    /// Linear (not dB) clip threshold.
    ceiling: f32,
    /// 0..1, blends hard clip (0.0) to soft tanh saturation (1.0). Same units as the Softness
    /// param's plain value.
    softness: f32,
    size: egui::Vec2,
}

impl TransferCurve {
    pub fn new(ceiling: f32, softness: f32) -> Self {
        Self {
            ceiling: ceiling.max(1e-6),
            softness,
            size: egui::vec2(80.0, 80.0),
        }
    }
}

impl egui::Widget for TransferCurve {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(self.size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            // Clipped to `rect`, same reasoning as `Scope`: keeps the curve from ever bleeding
            // past the widget's own bounds.
            let painter = ui.painter_at(rect);
            let visuals = ui.visuals();

            painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);

            // Domain/range shown: some headroom past the ceiling so the flattened top and bottom
            // of the curve are both visible, not clipped by the widget's own edges.
            let extent = self.ceiling * 1.6;
            let to_pos = |x: f32, y: f32| {
                egui::pos2(
                    rect.center().x + (x / extent).clamp(-1.0, 1.0) * rect.width() * 0.5,
                    rect.center().y - (y / extent).clamp(-1.0, 1.0) * rect.height() * 0.5,
                )
            };

            // Unity reference (what the curve would look like with no clipping at all), subtle.
            let unity_stroke = egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_fill);
            painter.line_segment(
                [to_pos(-extent, -extent), to_pos(extent, extent)],
                unity_stroke,
            );

            let points: Vec<egui::Pos2> = (0..=CURVE_SEGMENTS)
                .map(|i| {
                    let x = -extent + 2.0 * extent * (i as f32 / CURVE_SEGMENTS as f32);
                    let y = dsp::clip_sample(x, self.ceiling, self.softness);
                    to_pos(x, y)
                })
                .collect();
            let curve_stroke = egui::Stroke::new(1.5, visuals.selection.bg_fill);
            painter.add(egui::Shape::line(points, curve_stroke));
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs `add_contents` in a headless `egui::Context`, twice: egui's immediate-mode
    /// auto-sized containers (like `Area`) use their first pass just to measure content, and only
    /// actually paint once layout has settled on a second pass. Returns the second pass's output.
    fn run_ui(mut add_contents: impl FnMut(&mut egui::Ui)) -> egui::FullOutput {
        let ctx = egui::Context::default();
        let raw_input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(200.0, 200.0),
            )),
            ..Default::default()
        };

        let mut output = None;
        for _ in 0..2 {
            ctx.begin_pass(raw_input());
            egui::Area::new(egui::Id::new("test-area"))
                .fixed_pos(egui::Pos2::ZERO)
                .show(&ctx, &mut add_contents);
            output = Some(ctx.end_pass());
        }
        output.expect("loop runs at least once")
    }

    /// Finds the curve's own painted line (its fingerprint is having exactly
    /// `CURVE_SEGMENTS + 1` points — nothing else this widget paints has that many).
    fn curve_points(output: &egui::FullOutput) -> Vec<egui::Pos2> {
        output
            .shapes
            .iter()
            .find_map(|clipped| match &clipped.shape {
                egui::Shape::Path(path) if path.points.len() == CURVE_SEGMENTS + 1 => {
                    Some(path.points.clone())
                }
                _ => None,
            })
            .expect("transfer curve line not found among painted shapes")
    }

    #[test]
    fn softness_curve_is_displayed() {
        let output = run_ui(|ui| {
            ui.add(TransferCurve::new(1.0, 0.5));
        });
        assert_eq!(curve_points(&output).len(), CURVE_SEGMENTS + 1);
    }

    #[test]
    fn curve_shape_changes_with_softness() {
        let hard = curve_points(&run_ui(|ui| {
            ui.add(TransferCurve::new(1.0, 0.0));
        }));
        let soft = curve_points(&run_ui(|ui| {
            ui.add(TransferCurve::new(1.0, 1.0));
        }));

        // At the domain's outer edge (well past the ceiling, guaranteed clipping), hard clamping
        // and full tanh saturation must land at different y — that's the entire point of the
        // widget. If softness stopped affecting the curve, this would start failing.
        let last = CURVE_SEGMENTS;
        assert_ne!(
            hard[last].y, soft[last].y,
            "hard-clip and full-softness curves should diverge at the domain edge"
        );
    }
}
