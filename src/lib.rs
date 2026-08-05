use nice_plug::prelude::*;
use std::sync::Arc;

mod dsp;
mod editor;

/// A gain/softness/clip-amount clipper. See `CLAUDE.md` for the DSP design rationale.
struct Oclip {
    params: Arc<OclipParams>,
}

#[derive(Params)]
struct OclipParams {
    #[persist = "editor-state"]
    editor_state: Arc<nice_plug_egui::EguiState>,

    /// Input drive applied before the clipper, in dB.
    #[id = "gain"]
    gain: FloatParam,

    /// The level at which clipping engages, in dB. Lower means more aggressive clipping.
    #[id = "clip_amount"]
    clip_amount: FloatParam,

    /// Blends between a hard brick-wall clip (0%) and a smooth tanh saturation (100%).
    #[id = "softness"]
    softness: FloatParam,
}

impl Default for Oclip {
    fn default() -> Self {
        Self {
            params: Arc::new(OclipParams::default()),
        }
    }
}

impl Default for OclipParams {
    fn default() -> Self {
        Self {
            editor_state: editor::default_state(),

            gain: FloatParam::new(
                "Gain",
                0.0,
                FloatRange::Linear {
                    min: -24.0,
                    max: 24.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            clip_amount: FloatParam::new(
                "Clip Amount",
                0.0,
                FloatRange::Linear {
                    min: -24.0,
                    max: 0.0,
                },
            )
            .with_smoother(SmoothingStyle::Linear(10.0))
            .with_unit(" dB")
            .with_value_to_string(formatters::v2s_f32_rounded(2)),

            softness: FloatParam::new("Softness", 0.5, FloatRange::Linear { min: 0.0, max: 1.0 })
                .with_smoother(SmoothingStyle::Linear(10.0))
                .with_unit("%")
                .with_value_to_string(formatters::v2s_f32_percentage(0))
                .with_string_to_value(formatters::s2v_f32_percentage()),
        }
    }
}

impl Plugin for Oclip {
    const NAME: &'static str = "oclip";
    const VENDOR: &'static str = "r tech";
    const URL: &'static str = "https://github.com/johnjackbogart/oclip";
    const EMAIL: &'static str = "johnjackbogart@gmail.com";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[AudioIOLayout {
        main_input_channels: NonZeroU32::new(2),
        main_output_channels: NonZeroU32::new(2),
        ..AudioIOLayout::const_default()
    }];

    const MIDI_INPUT: MidiConfig = MidiConfig::None;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;
    const SAMPLE_ACCURATE_AUTOMATION: bool = true;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        // Temporarily disabled: opening the custom GUI segfaults on macOS (stack overflow via
        // runaway recursion during AppKit hitTest/cursor routing — see CLAUDE.md "Known
        // issues"). Falls back to the host's generic parameter panel until that's root-caused.
        let _ = editor::create; // silence unused-function warning while disabled
        None
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        _context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        for mut channel_samples in buffer.iter_samples() {
            let gain = self.params.gain.smoothed.next();
            let clip_amount = self.params.clip_amount.smoothed.next();
            let softness = self.params.softness.smoothed.next();

            let gain_linear = util::db_to_gain(gain);
            let threshold_linear = util::db_to_gain(clip_amount);

            for sample in channel_samples.iter_mut() {
                *sample = dsp::process_sample(*sample, gain_linear, threshold_linear, softness);
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Oclip {
    const CLAP_ID: &'static str = "org.oclip.oclip";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("A clipper: adjustable gain, clip amount, and softness.");
    const CLAP_MANUAL_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_SUPPORT_URL: Option<&'static str> = Some(Self::URL);
    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::AudioEffect,
        ClapFeature::Distortion,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for Oclip {
    const VST3_CLASS_ID: [u8; 16] = *b"oclipclipperplug";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

nice_export_clap!(Oclip);
nice_export_vst3!(Oclip);
