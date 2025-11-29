use nih_plug::prelude::*;
use std::num::NonZeroU32;
use std::sync::Arc;

#[derive(Params)]
struct SineParams {}

struct SimpleSine {
    params: Arc<SineParams>,

    phase: f32,
    freq_hz: f32,
    note_on: bool,
    sample_rate: f32,
}

impl Default for SineParams {
    fn default() -> Self {
        SineParams {}
    }
}

impl Default for SimpleSine {
    fn default() -> Self {
        SimpleSine {
            params: Arc::new(SineParams::default()),
            phase: 0.0,
            freq_hz: 440.0,
            note_on: false,
            sample_rate: 44100.0,
        }
    }
}

fn midi_note_to_freq(note: u8) -> f32 {
    let n = note as f32;
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

impl Plugin for SimpleSine {
    const VENDOR: &'static str = env!("CARGO_PKG_AUTHORS");
    const NAME: &'static str = "Simple Sine";
    const VERSION: &'static str = env!("CARGO_PKG_VERSION");
    const URL: &'static str = env!("CARGO_PKG_HOMEPAGE");
    const EMAIL: &'static str = "you@example.com";

    const AUDIO_IO_LAYOUTS: &'static [AudioIOLayout] = &[
        AudioIOLayout {
            main_input_channels: None,
            main_output_channels: NonZeroU32::new(2),
            aux_input_ports: &[],
            aux_output_ports: &[],
            names: PortNames::const_default(),
        },
    ];

    const MIDI_INPUT: MidiConfig = MidiConfig::Basic;
    const MIDI_OUTPUT: MidiConfig = MidiConfig::None;

    const SAMPLE_ACCURATE_AUTOMATION: bool = false;
    const HARD_REALTIME_ONLY: bool = false;

    type SysExMessage = ();
    type BackgroundTask = ();

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.sample_rate = buffer_config.sample_rate;
        self.phase = 0.0;
        true
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.note_on = false;
    }

    fn process(
        &mut self,
        buffer: &mut Buffer,
        _aux: &mut AuxiliaryBuffers,
        context: &mut impl ProcessContext<Self>,
    ) -> ProcessStatus {
        while let Some(event) = context.next_event() {
            use nih_plug::midi::NoteEvent;

            match event {
                NoteEvent::NoteOn { note, .. } => {
                    self.freq_hz = midi_note_to_freq(note);
                    self.note_on = true;
                }
                NoteEvent::NoteOff { .. } => {
                    self.note_on = false;
                    self.phase = 0.0;
                }
                _ => {
                }
            }
        }

        let sr = self.sample_rate.max(1.0);
        let freq = self.freq_hz;

        for mut channel_samples in buffer.iter_samples() {
            let sample_value: f32;
            if self.note_on {
                sample_value = (2.0 * std::f32::consts::PI * self.phase).sin();

                self.phase += freq / sr;
                if self.phase >= 1.0 {
                    self.phase = 0.0;
                }
            } else {
                sample_value = 0.0;
            };
            for sample in channel_samples.iter_mut() {
                *sample = sample_value;
            }
        }

        ProcessStatus::Normal
    }
}

impl ClapPlugin for SimpleSine {
    const CLAP_ID: &'static str = "com.example.simple_sine";
    const CLAP_DESCRIPTION: Option<&'static str> =
        Some("Sine test synth controlled by MIDI");
    const CLAP_MANUAL_URL: Option<&'static str> = None;
    const CLAP_SUPPORT_URL: Option<&'static str> = None;

    const CLAP_FEATURES: &'static [ClapFeature] = &[
        ClapFeature::Instrument,
        ClapFeature::Stereo,
    ];
}

impl Vst3Plugin for SimpleSine {
    const VST3_CLASS_ID: [u8; 16] = *b"SimpleSineSynth!";

    const VST3_SUBCATEGORIES: &'static [Vst3SubCategory] =
        &[Vst3SubCategory::Instrument];
}

nih_export_clap!(SimpleSine);
nih_export_vst3!(SimpleSine);
