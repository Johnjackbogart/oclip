//! An oscilloscope widget showing what the clipper is doing to the signal in real time: the
//! pre-clip trace (dim) overlaid with the post-clip trace (bright), against the clip ceiling. The
//! gap between the two traces *is* the amount being clipped.

use crate::scope::ScopeFrame;

/// Fixed linear-amplitude axis range the scope displays, independent of the current clip
/// threshold. Clip Amount's linear range tops out at 1.0 (0dB); at 1.1 that lands the ceiling
/// lines at ~91% of the widget's half-height when Clip Amount is maxed out, with a bit of
/// headroom left for a hot pre-clip signal above it. Crucially, keeping this fixed (rather than
/// scaling with the current ceiling) is what makes the ceiling lines actually move as Clip Amount
/// changes — if the axis rescaled to the ceiling too, the lines would land at the same apparent
/// height regardless of the threshold, which is the bug this fixes.
const Y_RANGE: f32 = 1.1;

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
            // Clipped to `rect` so nothing drawn here — traces, fill, stroke width, or any
            // future off-by-a-pixel math — can ever bleed past the widget's own bounds.
            let painter = ui.painter_at(rect);
            let visuals = ui.visuals();

            painter.rect_filled(rect, 4.0, visuals.extreme_bg_color);

            let map_y = |value: f32| {
                let t = (value / Y_RANGE).clamp(-1.0, 1.0);
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

                // Shade the gap between the two traces so the amount being clipped reads as a
                // visible wedge rather than something you have to spot between two thin
                // overlapping lines — this is what makes softness's effect on the knee shape
                // legible at a glance. Built as one raw triangle mesh (a quad per sample interval,
                // sharing vertices) rather than one `Shape::convex_polygon` per interval: each
                // `Shape` gets its own anti-aliased silhouette, so N adjacent shapes sharing edges
                // produced a visible seam at every single sample boundary. A single mesh has only
                // one outer boundary to feather.
                let accent = visuals.selection.bg_fill;
                let fill_color = egui::Color32::from_rgba_unmultiplied(
                    accent.r(),
                    accent.g(),
                    accent.b(),
                    60,
                );
                let mut fill_mesh = egui::Mesh::default();
                for (idx, window) in self.frames.windows(2).enumerate() {
                    let (a, b) = (window[0], window[1]);
                    let base = fill_mesh.vertices.len() as u32;
                    fill_mesh.colored_vertex(egui::pos2(x_at(idx), map_y(a.input)), fill_color);
                    fill_mesh
                        .colored_vertex(egui::pos2(x_at(idx + 1), map_y(b.input)), fill_color);
                    fill_mesh
                        .colored_vertex(egui::pos2(x_at(idx + 1), map_y(b.output)), fill_color);
                    fill_mesh.colored_vertex(egui::pos2(x_at(idx), map_y(a.output)), fill_color);
                    fill_mesh.add_triangle(base, base + 1, base + 2);
                    fill_mesh.add_triangle(base, base + 2, base + 3);
                }
                painter.add(egui::Shape::mesh(fill_mesh));

                let dim_stroke = egui::Stroke::new(1.0, visuals.widgets.inactive.bg_fill);
                painter.add(egui::Shape::line(points_for(|f| f.input), dim_stroke));

                let bright_stroke = egui::Stroke::new(1.5, visuals.selection.bg_fill);
                painter.add(egui::Shape::line(points_for(|f| f.output), bright_stroke));
            }
        }

        response
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dsp;

    /// Runs `add_contents` in a headless `egui::Context`, twice: egui's immediate-mode
    /// auto-sized containers (like `Area`) use their first pass just to measure content, and only
    /// actually paint once layout has settled on a second pass. Returns the second pass's output.
    fn run_ui(mut add_contents: impl FnMut(&mut egui::Ui)) -> egui::FullOutput {
        let ctx = egui::Context::default();
        let raw_input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(400.0, 400.0),
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

    /// A signal that clips on both excursions, so the shaded fill (the gap between the pre- and
    /// post-clip traces) has real, nonzero area to draw — otherwise the mesh would be trivially
    /// empty and this test wouldn't actually exercise the fill path.
    fn clipping_frames(count: usize) -> Vec<ScopeFrame> {
        (0..count)
            .map(|i| {
                let raw = (i as f32 / 8.0).sin() * 1.5;
                ScopeFrame {
                    input: raw,
                    output: dsp::clip_sample(raw, 1.0, 0.5),
                }
            })
            .collect()
    }

    #[test]
    fn waveform_fill_is_one_mesh_not_many_polygons() {
        let frames = clipping_frames(64);
        let output = run_ui(|ui| {
            ui.add(Scope::new(&frames, 1.0));
        });

        let meshes: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Mesh(mesh) => Some(mesh),
                _ => None,
            })
            .collect();
        assert_eq!(
            meshes.len(),
            1,
            "expected exactly one combined mesh for the shaded fill, found {}",
            meshes.len()
        );

        // One quad (4 vertices, 2 triangles = 6 indices) per sample interval, confirming the mesh
        // covers the whole frame range rather than some fixed/truncated subset.
        let mesh = meshes[0];
        assert_eq!(mesh.vertices.len(), (frames.len() - 1) * 4);
        assert_eq!(mesh.indices.len(), (frames.len() - 1) * 6);

        // Regression guard for the prior approach (one `Shape::convex_polygon` — a filled,
        // independently anti-aliased `Shape::Path` — per sample interval): adjacent shapes
        // sharing edges each got their own feathered silhouette, producing a visible seam at
        // every single sample boundary. There should be zero filled Path shapes now; the fill
        // comes only from the mesh above, and the trace lines are stroke-only (fill transparent).
        let filled_paths = output
            .shapes
            .iter()
            .filter(|clipped| {
                matches!(&clipped.shape, egui::Shape::Path(path) if path.fill != egui::Color32::TRANSPARENT)
            })
            .count();
        assert_eq!(
            filled_paths, 0,
            "fill should come from the mesh, not from individually-filled Path shapes (the seam bug)"
        );
    }

    #[test]
    fn with_fewer_than_two_frames_nothing_is_drawn_and_it_does_not_panic() {
        let output = run_ui(|ui| {
            ui.add(Scope::new(&[], 1.0));
        });
        let has_mesh = output
            .shapes
            .iter()
            .any(|clipped| matches!(clipped.shape, egui::Shape::Mesh(_)));
        assert!(!has_mesh, "no frames means no fill mesh to draw");
    }
}
