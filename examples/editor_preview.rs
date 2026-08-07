//! Standalone dev harness: opens the editor UI in a plain window with no DAW/VST/CLAP host at
//! all, using `egui-baseview` directly. Run with:
//!
//! ```sh
//! cargo run --example editor_preview
//! ```
//!
//! This exists to develop and debug the GUI locally — including reproducing the macOS crash
//! described in `CLAUDE.md` "Known issues", since it opens the same baseview window/view (and
//! therefore the same AppKit hitTest/cursor-routing machinery implicated there) that the real
//! plugin editor does. `Knob` is generic over `oclip::editor::knob::KnobValue`, so this harness
//! drives the exact same widget code as the real editor, just bound to plain in-memory values
//! instead of real `nice_plug` parameters.

use baseview::dpi::{LogicalSize, Size};
use egui::{CentralPanel, Context, FullOutput, Ui, ViewportOutput};
use egui_baseview::{EguiWindow, EguiWindowSettings, ExtraOutputCommands};
use oclip::editor::knob::{Knob, KnobValue};

fn fmt_gain(normalized: f32) -> String {
    format!("{:.2} dB", -24.0 + normalized * 48.0)
}

fn fmt_clip_amount(normalized: f32) -> String {
    format!("{:.2} dB", -24.0 + normalized * 24.0)
}

fn fmt_softness(normalized: f32) -> String {
    format!("{:.0}%", normalized * 100.0)
}

/// Binds [`KnobValue`] to a plain normalized `f32` in [`PreviewState`], mirroring how
/// `ParamKnobValue` binds to a real `nice_plug` parameter.
struct MockKnobValue<'a> {
    name: &'static str,
    normalized: &'a mut f32,
    default_normalized: f32,
    format: fn(f32) -> String,
}

impl KnobValue for MockKnobValue<'_> {
    fn name(&self) -> String {
        self.name.to_string()
    }

    fn normalized(&self) -> f32 {
        *self.normalized
    }

    fn default_normalized(&self) -> f32 {
        self.default_normalized
    }

    fn display(&self) -> String {
        (self.format)(*self.normalized)
    }

    fn begin_set(&mut self) {}

    fn set_normalized(&mut self, value: f32) {
        *self.normalized = value;
    }

    fn end_set(&mut self) {}
}

struct PreviewState {
    gain: f32,
    clip_amount: f32,
    softness: f32,
}

impl PreviewState {
    fn new() -> Self {
        Self {
            gain: 0.5,        // 0 dB
            clip_amount: 1.0, // 0 dB
            softness: 0.5,    // 50%
        }
    }
}

fn main() {
    let state = PreviewState::new();

    EguiWindow::open_blocking(
        EguiWindowSettings::new()
            .with_tile("oclip preview")
            .with_size(Size::Logical(LogicalSize {
                width: 220.0,
                height: 300.0,
            })),
        state,
        |_ctx: &Context, _commands: &mut ExtraOutputCommands, _state: &mut PreviewState| {},
        |_output: &FullOutput, _viewport_output: &ViewportOutput, _state: &mut PreviewState| {},
        |ui: &mut Ui, _commands: &mut ExtraOutputCommands, state: &mut PreviewState| {
            CentralPanel::default().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.heading("oclip");
                    ui.add_space(12.0);

                    ui.label("Gain");
                    ui.add(Knob::new(MockKnobValue {
                        name: "Gain",
                        normalized: &mut state.gain,
                        default_normalized: 0.5,
                        format: fmt_gain,
                    }));
                    ui.add_space(12.0);

                    ui.label("Clip Amount");
                    ui.add(Knob::new(MockKnobValue {
                        name: "Clip Amount",
                        normalized: &mut state.clip_amount,
                        default_normalized: 1.0,
                        format: fmt_clip_amount,
                    }));
                    ui.add_space(12.0);

                    ui.label("Softness");
                    ui.add(Knob::new(MockKnobValue {
                        name: "Softness",
                        normalized: &mut state.softness,
                        default_normalized: 0.5,
                        format: fmt_softness,
                    }));
                });
            });
        },
    );
}
