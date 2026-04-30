use std::f32::consts::PI;
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

    phase: f32,
    freq_hz: f32,
    note_on: bool,
    sample_rate: f32,
    last_output: f32,
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
            phase: 0.0,
            freq_hz: 440.0,
            note_on: false,
            sample_rate: 44100.0,
            last_output: 0.0,
            samples: Arc::new(Mutex::new(Vec::<f32>::with_capacity(VISUAL_BUFFER_SIZE))),
            fft_data: Arc::new(Mutex::new(Vec::<f32>::new())),
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

/// Calcule la magnitude du spectre (FFT) d'un signal réel.
///
/// - `samples` : signal temporel (f32)
/// - Retour : magnitudes des fréquences de 0 à Nyquist (N/2 bins)
///
/// Implémentation :
/// - zero-pad jusqu'à la puissance de 2 >= samples.len()
/// - FFT complexe radix-2 itératif, in-place
/// - retourne |X[k]| pour k ∈ [0, N/2)
pub fn compute_fft(samples: &[f32]) -> Vec<f32> {
    let len = samples.len();
    if len == 0 {
        return Vec::new();
    }

    // taille FFT = puissance de 2 >= len
    let n = len.next_power_of_two();

    // buffer complexe : (re, im)
    let mut buffer = vec![(0.0f32, 0.0f32); n];
    for (i, &s) in samples.iter().enumerate() {
        buffer[i].0 = s;
    }

    // FFT en place
    fft_in_place(&mut buffer);

    // magnitudes sur [0 .. N/2)
    let half = n / 2;
    let mut mags = Vec::with_capacity(half);
    for k in 0..half {
        let (re, im) = buffer[k];
        let mag = (re * re + im * im).sqrt();
        mags.push(mag);
    }

    mags
}

/// FFT complexe radix-2 in-place sur un buffer de (re, im).
///
/// - `buffer.len()` doit être une puissance de 2.
fn fft_in_place(buffer: &mut [(f32, f32)]) {
    let n = buffer.len();
    debug_assert!(n.is_power_of_two());

    // 1) Réorganisation bit-reversed
    bit_reverse_reorder(buffer);

    // 2) Étapes de papillons
    let mut len = 2;
    while len <= n {
        let half = len / 2;
        let theta = -2.0 * PI / (len as f32);
        let wlen = (theta.cos(), theta.sin()); // e^{-i 2π/len}

        let mut i = 0;
        while i < n {
            let mut w = (1.0f32, 0.0f32); // facteur de rotation courant

            for j in 0..half {
                let u = buffer[i + j];
                let t = complex_mul(buffer[i + j + half], w);

                // papillon
                buffer[i + j] = (u.0 + t.0, u.1 + t.1);
                buffer[i + j + half] = (u.0 - t.0, u.1 - t.1);

                w = complex_mul(w, wlen);
            }

            i += len;
        }

        len <<= 1;
    }
}

/// Réorganisation bit-reversed du buffer.
fn bit_reverse_reorder(buffer: &mut [(f32, f32)]) {
    let n = buffer.len();
    let mut j = 0usize;

    for i in 1..(n - 1) {
        let mut bit = n >> 1;
        while j & bit != 0 {
            j &= !bit;
            bit >>= 1;
        }
        j |= bit;

        if i < j {
            buffer.swap(i, j);
        }
    }
}

#[inline]
fn complex_mul(a: (f32, f32), b: (f32, f32)) -> (f32, f32) {
    // (a_re + i a_im) * (b_re + i b_im)
    (a.0 * b.0 - a.1 * b.1, a.0 * b.1 + a.1 * b.0)
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
        self.sample_rate = buffer_config.sample_rate;
        self.phase = 0.0;
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
                if samples_lock.len() < VISUAL_BUFFER_SIZE {
                    samples_lock.push(value);
                }
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
