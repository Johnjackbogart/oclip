use nice_plug::editor::Editor;
use nice_plug::editor::dpi::LogicalSize;
use nice_plug::util;
use nice_plug_egui::{EguiNiceSettings, EguiState, create_egui_editor};
use std::sync::Arc;

pub mod knob;
pub mod scope;
pub mod transfer_curve;

use crate::OclipParams;
use crate::scope::{ScopeConsumer, ScopeFrame};
use knob::{Knob, KnobValue};
use scope::Scope;
use transfer_curve::TransferCurve;

/// How many past frames the scope displays at once. ~186ms at 44.1kHz — long enough to read
/// several cycles of typical musical content, not so long that it turns into an illegible blob.
/// Redrawing/shifting this is GUI-thread work, not `process()`, so it isn't held to the same
/// real-time constraints.
const SCOPE_WINDOW_LEN: usize = 8192;

/// Fixed width for a label+knob column, wide enough to fit "Clip Amount" (the longest label)
/// without wrapping. Must be a fixed allocation, not `vertical_centered` — that layout centers
/// within the *full remaining width of its parent*, so as the first child in a `horizontal` row
/// it claims nearly the whole row for itself, squeezing whatever comes after it into a sliver
/// (word-wrapping the next label into many lines and pushing everything below off the window).
const KNOB_COLUMN_WIDTH: f32 = 100.0;

/// A label above a knob, centered within a fixed-width column. See [`KNOB_COLUMN_WIDTH`] for why
/// this can't just be `ui.vertical_centered`. Place these inside `ui.horizontal_top`, not plain
/// `ui.horizontal` — the latter's default `Align::Center` cross-axis placement doesn't reliably
/// line up equal-height columns in immediate-mode layout (confirmed empirically: two knob columns
/// of identical height ended up ~28px apart vertically under `ui.horizontal`).
fn knob_column(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::Rect {
    ui.allocate_ui_with_layout(
        egui::vec2(KNOB_COLUMN_WIDTH, 0.0),
        egui::Layout::top_down(egui::Align::Center),
        add_contents,
    )
    .response
    .rect
}

pub(crate) fn default_state() -> Arc<EguiState> {
    EguiState::from_size(LogicalSize::new(260.0, 360.0))
}

/// Rects of the interactive elements [`build_ui`] placed. Exists mainly so layout properties
/// ("Gain and Clip Amount sit at the same height", "the transfer curve is actually next to
/// Softness") are testable directly against the real layout code — see the `tests` module below —
/// rather than only checkable by eye.
#[derive(Debug, Clone, Copy)]
pub struct EditorLayoutRects {
    pub gain: egui::Rect,
    pub clip_amount: egui::Rect,
    /// The Softness knob's own rect (label excluded) — comparable to `gain`/`clip_amount` above,
    /// which are also bare-knob rects.
    pub softness: egui::Rect,
    /// The Softness *column*'s rect (label + knob together) — this, not `softness`, is what's
    /// top-aligned with `transfer_curve`: the curve has no label above it, so it lines up with
    /// the top of the whole column, not with the bare knob sitting below that column's label.
    pub softness_column: egui::Rect,
    pub transfer_curve: egui::Rect,
}

/// Builds the actual editor UI: heading, scope, and the three knob rows. This is the single
/// source of truth for the layout — both `create` (the real plugin editor) and
/// `examples/editor_preview.rs` (the standalone dev harness) call this directly, rather than each
/// keeping their own hand-copied version that could silently drift out of sync with the other.
pub fn build_ui<G: KnobValue, C: KnobValue, S: KnobValue>(
    ui: &mut egui::Ui,
    gain_knob: Knob<G>,
    clip_amount_knob: Knob<C>,
    softness_knob: Knob<S>,
    softness_normalized: f32,
    ceiling: f32,
    history: &[ScopeFrame],
) -> EditorLayoutRects {
    let mut rects = EditorLayoutRects {
        gain: egui::Rect::NOTHING,
        clip_amount: egui::Rect::NOTHING,
        softness: egui::Rect::NOTHING,
        softness_column: egui::Rect::NOTHING,
        transfer_curve: egui::Rect::NOTHING,
    };

    ui.vertical_centered(|ui| {
        ui.add_space(8.0);
        ui.heading("oclip");
        ui.add_space(12.0);

        ui.add(Scope::new(history, ceiling));
        ui.add_space(12.0);

        ui.horizontal_top(|ui| {
            knob_column(ui, |ui| {
                ui.label("Gain");
                rects.gain = ui.add(gain_knob).rect;
            });
            knob_column(ui, |ui| {
                ui.label("Clip Amount");
                rects.clip_amount = ui.add(clip_amount_knob).rect;
            });
        });
        ui.add_space(12.0);

        ui.horizontal_top(|ui| {
            rects.softness_column = knob_column(ui, |ui| {
                ui.label("Softness");
                rects.softness = ui.add(softness_knob).rect;
            });
            ui.add_space(8.0);
            rects.transfer_curve = ui.add(TransferCurve::new(ceiling, softness_normalized)).rect;
        });
    });

    rects
}

pub(crate) fn create(
    params: Arc<OclipParams>,
    scope_consumer: ScopeConsumer,
) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        Vec::<ScopeFrame>::new(),
        EguiNiceSettings::new().with_tile("oclip"),
        |_ctx, _commands, _state| {},
        move |ui, setter, _commands, history: &mut Vec<ScopeFrame>| {
            scope_consumer.drain_into(history);
            let excess = history.len().saturating_sub(SCOPE_WINDOW_LEN);
            if excess > 0 {
                history.drain(0..excess);
            }

            let ceiling = util::db_to_gain(params.clip_amount.value());
            let softness_normalized = params.softness.value();
            build_ui(
                ui,
                Knob::for_param(&params.gain, setter),
                Knob::for_param(&params.clip_amount, setter),
                Knob::for_param(&params.softness, setter),
                softness_normalized,
                ceiling,
                history,
            );
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `KnobValue` that just holds a fixed normalized position — enough to build a `Knob` for
    /// layout testing, without needing a real `nice_plug` param or the mock harness's UI-facing
    /// formatting.
    struct FixedKnobValue {
        normalized: f32,
    }

    impl KnobValue for FixedKnobValue {
        fn name(&self) -> String {
            "test".to_string()
        }
        fn normalized(&self) -> f32 {
            self.normalized
        }
        fn default_normalized(&self) -> f32 {
            0.5
        }
        fn display(&self) -> String {
            String::new()
        }
        fn begin_set(&mut self) {}
        fn set_normalized(&mut self, value: f32) {
            self.normalized = value;
        }
        fn end_set(&mut self) {}
    }

    /// Runs `build_ui` in a headless `egui::Context` (no window, no host) and returns both the
    /// painted shapes and the widget rects it reports, so tests can assert on the real layout
    /// code path instead of a re-implemented stand-in. Runs twice: egui's immediate-mode
    /// auto-sized containers (like `Area`) use their first pass just to measure content, and only
    /// actually settle/paint on a second pass.
    fn run_build_ui(softness_normalized: f32) -> (egui::FullOutput, EditorLayoutRects) {
        let ctx = egui::Context::default();
        let raw_input = || egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(260.0, 360.0),
            )),
            ..Default::default()
        };
        let history: Vec<ScopeFrame> = Vec::new();

        let mut output = None;
        let mut rects = None;
        for _ in 0..2 {
            ctx.begin_pass(raw_input());
            egui::Area::new(egui::Id::new("test-area"))
                .fixed_pos(egui::Pos2::ZERO)
                .show(&ctx, |ui| {
                    rects = Some(build_ui(
                        ui,
                        Knob::new(FixedKnobValue { normalized: 0.5 }),
                        Knob::new(FixedKnobValue { normalized: 0.5 }),
                        Knob::new(FixedKnobValue {
                            normalized: softness_normalized,
                        }),
                        softness_normalized,
                        1.0,
                        &history,
                    ));
                });
            output = Some(ctx.end_pass());
        }
        (
            output.expect("loop runs at least once"),
            rects.expect("build_ui should have run inside the Area closure"),
        )
    }

    #[test]
    fn gain_and_clip_amount_sit_side_by_side_at_the_same_height() {
        let (_output, rects) = run_build_ui(0.5);
        assert_eq!(
            rects.gain.top(),
            rects.clip_amount.top(),
            "Gain and Clip Amount should be at the exact same height, got {:?} vs {:?}",
            rects.gain,
            rects.clip_amount
        );
        assert!(
            rects.clip_amount.left() >= rects.gain.right(),
            "Clip Amount should be beside Gain, not overlapping/stacked: {:?} vs {:?}",
            rects.gain,
            rects.clip_amount
        );
    }

    #[test]
    fn softness_row_sits_below_gain_and_clip_amount_with_curve_alongside() {
        let (_output, rects) = run_build_ui(0.5);
        assert!(
            rects.softness.top() > rects.gain.bottom(),
            "Softness should be below the Gain/Clip Amount row: {:?} vs {:?}",
            rects.softness,
            rects.gain
        );
        assert!(
            rects.transfer_curve.left() >= rects.softness_column.right(),
            "the transfer curve should sit beside the Softness column, not overlap it: {:?} vs {:?}",
            rects.transfer_curve,
            rects.softness_column
        );
        // Compared against the *column* (label + knob), not the bare knob: the curve has no
        // label above it, so it lines up with the top of the whole column, not with the knob
        // that sits below that column's own label.
        assert_eq!(
            rects.softness_column.top(),
            rects.transfer_curve.top(),
            "the Softness column and its transfer curve should be top-aligned"
        );
    }

    #[test]
    fn softness_transfer_curve_is_actually_painted() {
        let (output, rects) = run_build_ui(0.5);
        // transfer_curve.rs draws its curve as a single Shape::Path stroke with
        // CURVE_SEGMENTS + 1 points; confirm one exists and falls within the widget's own rect
        // (not just that space was reserved for it).
        let found = output.shapes.iter().any(|clipped| match &clipped.shape {
            egui::Shape::Path(path) => {
                path.points.len() == transfer_curve::CURVE_SEGMENTS + 1
                    && path
                        .points
                        .iter()
                        .all(|p| rects.transfer_curve.expand(1.0).contains(*p))
            }
            _ => false,
        });
        assert!(found, "expected the softness transfer curve to be painted inside its widget rect");
    }
}
