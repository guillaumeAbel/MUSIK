use nih_plug::prelude::*;
use nih_plug_egui::{create_egui_editor, EguiState};
use std::num::NonZeroU32;
use std::sync::{Arc, Mutex};

const VISUAL_BUFFER_SIZE: usize = 44100;

#[derive(Params)]
struct SineParams {
    #[persist = "editor-state"]
    pub editor_state: Arc<EguiState>,
}

enum BgTask {
    RecomputeFft,
}

struct SimpleSine {
    params: Arc<SineParams>,
    note_on: bool,
    samples: Arc<Mutex<Vec<f32>>>,
    fft_data: Arc<Mutex<Vec<f32>>>,
}

impl Default for SineParams {
    fn default() -> Self {
        Self {
            editor_state: EguiState::from_size(820, 800),
        }
    }
}

impl Default for SimpleSine {
    fn default() -> Self {
        Self {
            params: Arc::new(SineParams::default()),
            note_on: false,
            samples: Arc::new(Mutex::new(Vec::<f32>::with_capacity(VISUAL_BUFFER_SIZE))),
            fft_data: Arc::new(Mutex::new(Vec::<f32>::new())),
        }
    }

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
    type BackgroundTask = BgTask;

    fn task_executor(&mut self) -> TaskExecutor<Self> {
        let time_buffer = self.samples.clone();
        let fft_buffer = self.fft_data.clone();

        Box::new(move |task| match task {
            BgTask::RecomputeFft => {
                // 1) récupérer un snapshot des samples temps
                let mut lock = time_buffer.lock().unwrap();
                let samples = lock.clone();
                lock.clear();
                drop(lock);

                // 2) calcul FFT (fonction que je t’ai donnée)
                let spectrum = compute_fft(&samples);

                // 3) stocker le résultat pour le thread GUI
                let mut fft_lock = fft_buffer.lock().unwrap();
                *fft_lock = spectrum;
            }
        })
    }

    fn params(&self) -> Arc<dyn Params> {
        self.params.clone()
    }

    fn initialize(
        &mut self,
        _audio_io_layout: &AudioIOLayout,
        buffer_config: &BufferConfig,
        _context: &mut impl InitContext<Self>,
    ) -> bool {
        self.osc = Oscillator::new(buffer_config.sample_rate, None, Optional<f32>::New(0.25));
        true
    }

    fn reset(&mut self) {
        self.phase = 0.0;
        self.last_output = 0.0;
        self.note_on = false;
    }

    fn editor(&mut self, async_executor: AsyncExecutor<Self>) -> Option<Box<dyn Editor>> {
        let egui_state = self.params.editor_state.clone();
        //let ptr = self.samples.clone();
        let fft_ptr = self.fft_data.clone();
        let exec = async_executor.clone();

        create_egui_editor(
            egui_state,
            (),
            |_ctx, _state| {},
            move |ctx, _setter, _state| {
                use nih_plug_egui::egui::*;
                    CentralPanel::default().show(ctx, |ui| {

                    if ui.button("Recompute FFT").clicked() {
                        exec.execute_background(BgTask::RecomputeFft);
                    }

                    ui.heading("Wave (time domain)");

                    // --- 1) PREMIER RECTANGLE : WAVEFORM TEMPS ---
                    {
                        let (rect, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), 140.0),
                            Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);

                        let samples = Vec::new();
                        //let mut samples_lock = ptr.lock().unwrap();
                        //while let Some(s) = samples_lock.pop() {
                        //    samples.push(s);
                        //}
                        //drop(samples_lock);

                        if samples.len() >= 2 {
                            let w = rect.width().max(1.0);
                            let h = rect.height().max(1.0);

                            let to_pos = |i: usize, s: f32| {
                                let x = rect.left() + (i as f32 / (samples.len() - 1) as f32) * w;
                                let y = rect.center().y - s.clamp(-1.0, 1.0) * (h * 0.45);
                                pos2(x, y)
                            };

                            let points: Vec<Pos2> = samples
                                .iter()
                                .enumerate()
                                .map(|(i, &s)| to_pos(i, s))
                                .collect();

                            painter.add(Shape::line(points, Stroke::new(1.0, Color32::GREEN)));
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(1.0, Color32::DARK_GRAY),
                                StrokeKind::Outside,
                            );
                        } else {
                            ui.label("wave needs at least 2 samples");
                        }
                    }

                    ui.add_space(12.0); // petit gap entre les deux

                    // --- 2) DEUXIÈME RECTANGLE : FFT ---
                    ui.heading("Spectrum (FFT)");

                    {
                        // même taille que le scope temps
                        let (rect, _) = ui.allocate_exact_size(
                            vec2(ui.available_width(), 440.0), // <-- idem
                            Sense::hover(),
                        );
                        let painter = ui.painter_at(rect);

                        // ici tu lis tes données FFT depuis un autre shared state :
                        // par ex. Arc<Mutex<Vec<f32>>> => fft_ptr

                        let mut fft_samples = Vec::new();
                        let fft_lock = fft_ptr.lock().unwrap();
                        fft_samples.extend(fft_lock.iter().copied());
                        drop(fft_lock);

                        // mapping fréquence
                        let min_freq: f32 = 20.0;
                        let max_freq: f32 = 20000.0;
                        let log_min = min_freq.log10();
                        let log_range = max_freq.log10() - log_min;
                        let min_db = -25.0;
                        let max_db = 110.0;
                        let db_range = max_db - min_db;
                        //compute les coordonnées écran une seule fois, en background task ?

                        let samples_len = fft_samples.len() as f32;
                        if samples_len >= 2.0 {
                            let plot_bottom = rect.bottom() - 22.0;
                            let plot_height = (plot_bottom - rect.top()).max(1.0);
                            let freq_to_x = |freq: f32| {
                                let f = freq.clamp(min_freq, max_freq);
                                let x_norm = (f.log10() - log_min) / log_range;
                                rect.left() + x_norm * rect.width()
                            };

                            let to_pos = |i: usize, mag: f32| {
                                let freq = (i as f32 / samples_len) * max_freq;
                                let x = freq_to_x(freq);
                                let mut m = mag.max(1e-9);      // éviter log10(0)
                                m = 20.0 * m.log10();
                                let db = m.clamp(min_db, max_db);
                                let y_norm = (db - min_db) / db_range;
                                let y = plot_bottom - y_norm * plot_height;
                                pos2(x, y)
                            };

                            let points: Vec<Pos2> = fft_samples
                                .iter()
                                .enumerate()
                                .map(|(i, &s)| to_pos(i, s))
                                .collect();

                            painter.add(Shape::line(points, Stroke::new(1.0, Color32::LIGHT_BLUE)));
                            painter.rect_stroke(
                                rect,
                                0.0,
                                Stroke::new(1.0, Color32::DARK_GRAY),
                                StrokeKind::Outside,
                            );

                            let freq_ticks = [
                                (20.0, "20"),
                                (50.0, "50"),
                                (100.0, "100"),
                                (200.0, "200"),
                                (500.0, "500"),
                                (1000.0, "1K"),
                                (2000.0, "2K"),
                                (5000.0, "5K"),
                                (10000.0, "10K"),
                            ];

                            for (freq, label) in freq_ticks {
                                let x = freq_to_x(freq);
                                let label_x = if freq <= 20.0 { x + 7.0 } else { x };
                                painter.line_segment(
                                    [pos2(x, plot_bottom), pos2(x, plot_bottom + 6.0)],
                                    Stroke::new(1.0, Color32::GRAY),
                                );
                                painter.text(
                                    pos2(label_x, rect.bottom() - 2.0),
                                    Align2::CENTER_BOTTOM,
                                    label,
                                    FontId::proportional(11.0),
                                    Color32::LIGHT_GRAY,
                                );
                            }
                        } else {
                            ui.label("FFT: no data yet");
                        }
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
                    let wave = self.osc.compute_wave(midi_to_frequency(note)); // Vec<Vec<f32>>

                    //trouver meilleur méthode, en passant un itérator à l'obj oscillator ?
                    for (i, frame) in buffer.iter_samples().enumerate() {
                        for (c, dst) in frame.into_iter().enumerate() {
                            *dst = wave
                                .get(c)
                                .and_then(|ch| ch.get(i))
                                .copied()
                                .unwrap_or(0.0);
                        }
                    }
                NoteEvent::NoteOff { .. } => {
                    self.osc.reset_phase();
                }
                _ => {
                }
            }
        }

        //push to visual buffer
        let ptr = self.samples.clone();
        let mut samples_lock = ptr.lock().unwrap();
        if samples_lock.len() < VISUAL_BUFFER_SIZE {
            
        }
        drop(samples_lock);

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
