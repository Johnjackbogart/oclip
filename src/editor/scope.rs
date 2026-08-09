//! An oscilloscope widget showing what the clipper is doing to the signal in real time: the
//! pre-clip trace (dim) overlaid with the post-clip trace (bright), against the clip ceiling. The
//! gap between the two traces *is* the amount being clipped.

use crate::scope::ScopeFrame;

/// A scrolling two-trace oscilloscope: pre-clip (dim) and post-clip (bright), sharing the same
/// linear-amplitude y-axis, with the clip ceiling marked.
pub struct Scope<'a> {
    /// Oldest frame first, newest last — drawn left to right.
    frames: &'a [ScopeFrame],
    /// Linear (not dB) clip threshold, for the ceiling lines.
    ceiling: f32,
    size: egui::Vec2,
}

impl<'a> Scope<'a> {
    pub fn new(frames: &'a [ScopeFrame], ceiling: f32) -> Self {
        Self {
            frames,
            ceiling: ceiling.max(1e-6),
            size: egui::vec2(240.0, 100.0),
        }
    }
}

impl egui::Widget for Scope<'_> {
    fn ui(self, ui: &mut egui::Ui) -> egui::Response {
        let (rect, response) = ui.allocate_exact_size(self.size, egui::Sense::hover());

        if ui.is_rect_visible(rect) {
            let painter = ui.painter();
            let visuals = ui.visuals();

            painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);

            // Headroom above the ceiling so a hot pre-clip signal doesn't slam the top edge.
            let y_range = self.ceiling * 1.3;
            let map_y = |value: f32| {
                let t = (value / y_range).clamp(-1.0, 1.0);
                rect.center().y - t * rect.height() * 0.5
            };

            let zero_stroke = egui::Stroke::new(1.0, visuals.widgets.noninteractive.bg_fill);
            painter.hline(rect.x_range(), map_y(0.0), zero_stroke);

            let ceiling_stroke = egui::Stroke::new(1.0, visuals.widgets.inactive.bg_fill);
            painter.hline(rect.x_range(), map_y(self.ceiling), ceiling_stroke);
            painter.hline(rect.x_range(), map_y(-self.ceiling), ceiling_stroke);

            if self.frames.len() >= 2 {
                let dx = rect.width() / (self.frames.len() - 1) as f32;
                let x_at = |i: usize| rect.left() + i as f32 * dx;
                let points_for = |pick: fn(&ScopeFrame) -> f32| -> Vec<egui::Pos2> {
                    self.frames
                        .iter()
                        .enumerate()
                        .map(|(i, frame)| egui::pos2(x_at(i), map_y(pick(frame))))
                        .collect()
                };

                // Shade the gap between the two traces, one quad per sample interval, so the
                // amount being clipped reads as a visible wedge rather than something you have to
                // spot between two thin overlapping lines. This is what actually makes softness's
                // effect on the knee shape legible at a glance.
                let accent = visuals.selection.bg_fill;
                let fill_color = egui::Color32::from_rgba_unmultiplied(
                    accent.r(),
                    accent.g(),
                    accent.b(),
                    60,
                );
                for (idx, window) in self.frames.windows(2).enumerate() {
                    let (a, b) = (window[0], window[1]);
                    let quad = vec![
                        egui::pos2(x_at(idx), map_y(a.input)),
                        egui::pos2(x_at(idx + 1), map_y(b.input)),
                        egui::pos2(x_at(idx + 1), map_y(b.output)),
                        egui::pos2(x_at(idx), map_y(a.output)),
                    ];
                    painter.add(egui::Shape::convex_polygon(
                        quad,
                        fill_color,
                        egui::Stroke::NONE,
                    ));
                }

                let dim_stroke = egui::Stroke::new(1.0, visuals.widgets.inactive.bg_fill);
                painter.add(egui::Shape::line(points_for(|f| f.input), dim_stroke));

                let bright_stroke = egui::Stroke::new(1.5, visuals.selection.bg_fill);
                painter.add(egui::Shape::line(points_for(|f| f.output), bright_stroke));
            }
        }

        response
    }
}
