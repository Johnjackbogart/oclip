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

pub(crate) fn default_state() -> Arc<EguiState> {
    EguiState::from_size(LogicalSize::new(260.0, 420.0))
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

                ui.label("Gain");
                ui.add(Knob::for_param(&params.gain, setter));
                ui.add_space(12.0);

                ui.label("Clip Amount");
                ui.add(Knob::for_param(&params.clip_amount, setter));
                ui.add_space(12.0);

                ui.label("Softness");
                ui.horizontal(|ui| {
                    ui.add(Knob::for_param(&params.softness, setter));
                    ui.add(TransferCurve::new(ceiling, params.softness.value()));
                });
            });
        },
    )
}
