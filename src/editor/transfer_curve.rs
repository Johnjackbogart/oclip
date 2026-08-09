//! A static input->output transfer curve, independent of whatever audio happens to be playing.
//! Turning Softness bends the knee from a sharp corner (0%, hard clip) to a smooth curve (100%,
//! tanh saturation) — this makes that shape directly visible, the way GClip's scope does, rather
//! than relying on spotting it in a live waveform.

use crate::dsp;

const CURVE_SEGMENTS: usize = 64;

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
            let painter = ui.painter();
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
