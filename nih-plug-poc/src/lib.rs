use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

const VISUAL_BUFFER_SIZE: usize = 2048;

#[derive(Params)]
struct SineParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
}

struct SimpleSine {
    params: Arc<SineParams>,

    phase: f32,
    freq_hz: f32,
    note_on: bool,
    sample_rate: f32,
    last_output: f32,
    samples: Arc<Mutex<Vec<f32>>>
}

impl Default for SineParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(420, 220),
        }
    }
}

impl Default for SimpleSine {
    fn default() -> Self {
        Self {
            params: Arc::new(SineParams::default()),
            phase: 0.0,
            freq_hz: 440.0,
            note_on: false,
            sample_rate: 44100.0,
            last_output: 0.0,
            samples: Arc::new(Mutex::new(Vec::<f32>::with_capacity(VISUAL_BUFFER_SIZE))),
        }
    }
}

fn midi_note_to_freq(note: u8) -> f32 {
    let n = note as f32;
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

fn poly_blep(mut t: f32, delta: f32) -> f32 {
    if t < delta {
        t /= delta;
        return t + t - t * t - 1.0;
    } else if t > 1.0 - delta {
        t = (t - 1.0) / delta;
        return t * t + t + t + 1.0;
    }
    0.0
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
        self.last_output = 0.0;
        self.note_on = false;
    }

    fn editor(&mut self, _async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let egui_state = self.params.editor_state.clone();
        let ptr = self.samples.clone();

        create_egui_editor(
            egui_state,
            (),                 // no user_state needed
            |_ctx, _state| {},  // build()
            move |ctx, _setter, _state| {
                use nih_plug_egui::egui::*;

                CentralPanel::default().show(ctx, |ui| {
                    ui.heading("Wave");

                    let (rect, _) = ui.allocate_exact_size(
                        vec2(ui.available_width(), 140.0),
                        Sense::hover(),
                    );
                    let painter = ui.painter_at(rect);

                    let mut samples = vec![];
                    let mut samples_lock = ptr.lock().unwrap();
                    while let Some(s) = samples_lock.pop() {
                        samples.push(s);
                    }
                    drop(samples_lock);

                    if samples.len() >= 2 {
                        let w = rect.width().max(1.0);
                        let h = rect.height().max(1.0);

                        let to_pos = |i: usize, s: f32| {
                            let x = rect.left() + (i as f32 / (samples.len() - 1) as f32) * w;
                            let y = rect.center().y - s.clamp(-1.0, 1.0) * (h * 0.45);
                            pos2(x, y)
                        };

                        let points: Vec<Pos2> = samples.iter().enumerate().map(|(i, &s)| to_pos(i, s)).collect();
                        painter.add(Shape::line(points, Stroke::new(1.0, Color32::GREEN)));
                        painter.rect_stroke(rect, 0.0, Stroke::new(1.0, Color32::DARK_GRAY), StrokeKind::Outside);
                    } else {
                        ui.label("wave needs at least 2 samples");
                    }
                });
            },
        )
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

        let delta = self.freq_hz / self.sample_rate.max(1.0); 
        let level: f32 = 0.3;

        for mut channel_samples in buffer.iter_samples() {
            let mut value: f32;
            if self.note_on {
                if self.phase < 0.5 {
                    value = 1.0;
                } else {
                    value = -1.0;
                }
                value += poly_blep(self.phase, delta);
                let t = (self.phase + 0.5) % 1.0;
                value -= poly_blep(t, delta);
                //from square to triangle
                value = delta * value + (1.0 - delta) * self.last_output;
                self.last_output = value;
                value *= level;

                let ptr = self.samples.clone();
                let mut samples_lock = ptr.lock().unwrap();
                samples_lock.push(value);
                drop(samples_lock);

                self.phase += delta;
                if self.phase >= 1.0 {
                    self.phase -= 1.0;
                }
            } else {
                value = 0.0;
            }
            for sample in channel_samples.iter_mut() {
                *sample = value;
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
