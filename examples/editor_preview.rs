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
use oclip::dsp;
use oclip::editor::knob::{Knob, KnobValue};
use oclip::editor::scope::Scope;
use oclip::editor::transfer_curve::TransferCurve;
use oclip::scope::ScopeFrame;

/// How many synthetic samples the scope preview displays at once, matching the real editor's
/// window (see `SCOPE_WINDOW_LEN` in `src/editor/mod.rs`).
const SCOPE_WINDOW_LEN: usize = 8192;
/// Nominal sample rate used only to make the synthetic preview tone look plausible on screen —
/// there's no real audio thread in this harness.
const PREVIEW_SAMPLE_RATE: f32 = 44_100.0;
/// New synthetic samples generated per GUI redraw, so the trace visibly scrolls.
const SAMPLES_PER_FRAME: usize = 64;
/// Synthetic tone amplitude multiplier, independent of the Gain knob: pushes the default-settings
/// preview well past the default 0dB ceiling so clipping (and softness's effect on its shape) is
/// visible immediately, without needing to touch a knob first.
const PREVIEW_TONE_DRIVE: f32 = 1.8;
/// Fixed width for a label+knob column, matching `KNOB_COLUMN_WIDTH` in `src/editor/mod.rs` (see
/// that constant's comment for why this can't be `ui.vertical_centered`).
const KNOB_COLUMN_WIDTH: f32 = 100.0;

fn knob_column(
    ui: &mut egui::Ui,
    add_contents: impl FnOnce(&mut egui::Ui),
) -> egui::InnerResponse<()> {
    ui.allocate_ui_with_layout(
        egui::vec2(KNOB_COLUMN_WIDTH, 0.0),
        egui::Layout::top_down(egui::Align::Center),
        add_contents,
    )
}

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
    /// Phase accumulator for the synthetic preview tone (radians).
    phase: f32,
    scope_history: Vec<ScopeFrame>,
}

impl PreviewState {
    fn new() -> Self {
        Self {
            gain: 0.5,        // 0 dB
            clip_amount: 1.0, // 0 dB
            softness: 0.5,    // 50%
            phase: 0.0,
            scope_history: Vec::new(),
        }
    }

    /// Feeds a chunk of a synthetic 220 Hz tone through the real clipping math, using the
    /// preview's current knob positions, so the scope widget has something real to draw.
    fn advance_scope(&mut self) {
        let gain_db = -24.0 + self.gain * 48.0;
        let clip_amount_db = -24.0 + self.clip_amount * 24.0;
        let gain_linear = nice_plug::util::db_to_gain(gain_db);
        let threshold_linear = nice_plug::util::db_to_gain(clip_amount_db);

        const FREQUENCY_HZ: f32 = 220.0;
        let phase_step = FREQUENCY_HZ * std::f32::consts::TAU / PREVIEW_SAMPLE_RATE;

        for _ in 0..SAMPLES_PER_FRAME {
            let raw = self.phase.sin() * PREVIEW_TONE_DRIVE;
            self.phase = (self.phase + phase_step) % std::f32::consts::TAU;

            let pre_clip = raw * gain_linear;
            let post_clip = dsp::clip_sample(pre_clip, threshold_linear, self.softness);
            self.scope_history.push(ScopeFrame {
                input: pre_clip,
                output: post_clip,
            });
        }

        let excess = self.scope_history.len().saturating_sub(SCOPE_WINDOW_LEN);
        if excess > 0 {
            self.scope_history.drain(0..excess);
        }
    }
}

fn main() {
    let state = PreviewState::new();

    EguiWindow::open_blocking(
        EguiWindowSettings::new()
            .with_tile("oclip preview")
            .with_size(Size::Logical(LogicalSize {
                width: 260.0,
                height: 360.0,
            })),
        state,
        |_ctx: &Context, _commands: &mut ExtraOutputCommands, _state: &mut PreviewState| {},
        |_output: &FullOutput, _viewport_output: &ViewportOutput, _state: &mut PreviewState| {},
        |ui: &mut Ui, _commands: &mut ExtraOutputCommands, state: &mut PreviewState| {
            state.advance_scope();
            ui.ctx().request_repaint(); // keep the synthetic tone animating

            CentralPanel::default().show(ui, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(8.0);
                    ui.heading("oclip");
                    ui.add_space(12.0);

                    let clip_amount_db = -24.0 + state.clip_amount * 24.0;
                    let ceiling = nice_plug::util::db_to_gain(clip_amount_db);
                    ui.add(Scope::new(&state.scope_history, ceiling));
                    ui.add_space(12.0);

                    let row1 = ui.horizontal_top(|ui| {
                        let gain_col = knob_column(ui, |ui| {
                            ui.label("Gain");
                            ui.add(Knob::new(MockKnobValue {
                                name: "Gain",
                                normalized: &mut state.gain,
                                default_normalized: 0.5,
                                format: fmt_gain,
                            }));
                        });
                        eprintln!("DEBUG gain_col.rect = {:?}", gain_col.response.rect);
                        let clip_col = knob_column(ui, |ui| {
                            ui.label("Clip Amount");
                            ui.add(Knob::new(MockKnobValue {
                                name: "Clip Amount",
                                normalized: &mut state.clip_amount,
                                default_normalized: 1.0,
                                format: fmt_clip_amount,
                            }));
                        });
                        eprintln!("DEBUG clip_col.rect = {:?}", clip_col.response.rect);
                    });
                    eprintln!("DEBUG row1.rect = {:?}", row1.response.rect);
                    ui.add_space(12.0);

                    let row2 = ui.horizontal(|ui| {
                        knob_column(ui, |ui| {
                            ui.label("Softness");
                            ui.add(Knob::new(MockKnobValue {
                                name: "Softness",
                                normalized: &mut state.softness,
                                default_normalized: 0.5,
                                format: fmt_softness,
                            }));
                        });
                        ui.add_space(8.0);
                        let curve_resp = ui.add(TransferCurve::new(ceiling, state.softness));
                        eprintln!("DEBUG curve_resp.rect = {:?}", curve_resp.rect);
                    });
                    eprintln!("DEBUG row2.rect = {:?}", row2.response.rect);
                });
            });
        },
    );
}
