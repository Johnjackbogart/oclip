use nice_plug::prelude::*;
use std::sync::Arc;

pub mod dsp;
pub mod editor;
pub mod scope;

/// Frames of headroom for the audio->GUI scope channel: generous enough that a momentary GUI
/// stall doesn't drop data, without meaningfully allocating anything process() touches (the
/// channel is created once, in `Openclip::default`, not per-block).
const SCOPE_CHANNEL_CAPACITY: usize = 32_768;

/// A gain/softness/clip-amount clipper. See `CLAUDE.md` for the DSP design rationale.
struct Openclip {
    params: Arc<OpenclipParams>,
    scope_producer: scope::ScopeProducer,
    scope_consumer: scope::ScopeConsumer,
}

#[derive(Params)]
struct OpenclipParams {
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

impl Default for Openclip {
    fn default() -> Self {
        let (scope_producer, scope_consumer) = scope::channel(SCOPE_CHANNEL_CAPACITY);
        Self {
            params: Arc::new(OpenclipParams::default()),
            scope_producer,
            scope_consumer,
        }
    }
}

impl Default for OpenclipParams {
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

impl Plugin for Openclip {
    const NAME: &'static str = "openclip";
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
        editor::create(self.params.clone(), self.scope_consumer.clone())
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

            // Scope only tracks channel 0 — it's a visualization, not a per-channel meter, so one
            // representative channel per frame is enough (and keeps this at one push per frame,
            // not one per channel).
            let mut scope_frame = None;

            for (channel_index, sample) in channel_samples.iter_mut().enumerate() {
                let pre_clip = *sample * gain_linear;
                let post_clip = dsp::clip_sample(pre_clip, threshold_linear, softness);
                *sample = post_clip;

                if channel_index == 0 {
                    scope_frame = Some((pre_clip, post_clip));
                }
            }

            if let Some((input, output)) = scope_frame {
                self.scope_producer.push(input, output);
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for Openclip {
    const CLAP_ID: &'static str = "org.openclip.openclip";
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

impl Vst3Plugin for Openclip {
    const VST3_CLASS_ID: [u8; 16] = *b"openclipperplugi";
    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Fx, Vst3SubCategory::Distortion];
}

nice_export_clap!(Openclip);
nice_export_vst3!(Openclip);
