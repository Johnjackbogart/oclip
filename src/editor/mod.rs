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
use knob::Knob;
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
fn knob_column(ui: &mut egui::Ui, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.allocate_ui_with_layout(
        egui::vec2(KNOB_COLUMN_WIDTH, 0.0),
        egui::Layout::top_down(egui::Align::Center),
        add_contents,
    );
}

pub(crate) fn default_state() -> Arc<EguiState> {
    EguiState::from_size(LogicalSize::new(260.0, 360.0))
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

            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading("oclip");
                ui.add_space(12.0);

                let ceiling = util::db_to_gain(params.clip_amount.value());
                ui.add(Scope::new(history, ceiling));
                ui.add_space(12.0);

                ui.horizontal_top(|ui| {
                    knob_column(ui, |ui| {
                        ui.label("Gain");
                        ui.add(Knob::for_param(&params.gain, setter));
                    });
                    knob_column(ui, |ui| {
                        ui.label("Clip Amount");
                        ui.add(Knob::for_param(&params.clip_amount, setter));
                    });
                });
                ui.add_space(12.0);

                ui.horizontal_top(|ui| {
                    knob_column(ui, |ui| {
                        ui.label("Softness");
                        ui.add(Knob::for_param(&params.softness, setter));
                    });
                    ui.add_space(8.0);
                    ui.add(TransferCurve::new(ceiling, params.softness.value()));
                });
            });
        },
    )
}
