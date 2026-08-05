use nice_plug::editor::Editor;
use nice_plug::editor::dpi::LogicalSize;
use nice_plug_egui::{EguiNiceSettings, EguiState, create_egui_editor};
use std::sync::Arc;

mod knob;

use crate::OclipParams;
use knob::Knob;

pub(crate) fn default_state() -> Arc<EguiState> {
    EguiState::from_size(LogicalSize::new(220.0, 300.0))
}

pub(crate) fn create(params: Arc<OclipParams>) -> Option<Box<dyn Editor>> {
    create_egui_editor(
        params.editor_state.clone(),
        (),
        EguiNiceSettings::new().with_tile("oclip"),
        |_ctx, _commands, _state| {},
        move |ui, setter, _commands, _state| {
            ui.vertical_centered(|ui| {
                ui.add_space(8.0);
                ui.heading("oclip");
                ui.add_space(12.0);

                ui.label("Gain");
                ui.add(Knob::for_param(&params.gain, setter));
                ui.add_space(12.0);

                ui.label("Clip Amount");
                ui.add(Knob::for_param(&params.clip_amount, setter));
                ui.add_space(12.0);

                ui.label("Softness");
                ui.add(Knob::for_param(&params.softness, setter));
            });
        },
    )
}
