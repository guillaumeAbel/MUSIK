use std::f32::consts::PI;

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

pub fn midi_to_frequency(note: u8) -> f32 {
    let n = note as f32;
    440.0 * 2f32.powf((n - 69.0) / 12.0)
}

pub enum EWave {
    Sin,
    Saw,
    Square,
    Triangle
}

pub struct Oscillator {
    sample_rate: f32,
    wave: EWave,
    level: f32,
    panning: f32,
}

impl Oscillator {
    pub fn new(sample_rate: f32, wave: Optional<EWave>, level: Optional<f32>) -> Oscillator {
        let init_wave = if wave == None {EWave::Saw} else {wave.value};
        let init_level = if level == None {0.4} else {level.value};
        Oscillator {sample_rate, init_wave, init_level, 0.0}
    }

    fn poly_blep(&self, mut t: f32) -> f32 {
        if t < delta {
            t /= delta;
            return t + t - t * t - 1.0;
        } else if t > 1.0 - delta {
            t = (t - 1.0) / delta;
            return t * t + t + t + 1.0;
        }
        0.0
    }

    fn compute_sample(delta: f32, phase: f32, wave: EWave, last_output: f32) {
        let mut value: f32 = 0.0;
        if (wave == 0) {
            if phase < 0.5 {
                value = 1.0;
            } else {
                value = -1.0;
            }
            value += poly_blep(phase, delta);
            let t = (phase + 0.5) % 1.0;
            value -= poly_blep(t, delta);
            //from square to triangle
            value = delta * value + (1.0 - delta) * last_output;
            last_output = value;
            value *= self.level;
        }
    }

    pub fn compute_wave(&self, freq: f32/*, vélocitée en param par defaut ?*/) -> Vec<Vec<f32>> {
        let mut last_output: f32 = 0.0;
        let mut phase = 0.0;
        let delta = freq / self.sample_rate.max(1.0); 
        let mut samples = Vec<f32>::with_capacity(self.sample_rate);

        for sample in samples {
            *sample = compute_sample(freq, phase, wave/* wave doit il etre un param de l'osc ou de la wave, sachant que le type de wave peut etre différent entre 2 notes dans le meme osc, avec l'option "notes"(cf serum)"*/, last_output);
            phase += delta;
            if phase >= 1.0 {
                phase -= 1.0;
            }
        }
        //gerer mono/stereo et panning;
        [samples, samples]
    }

    pub fn reset_phase(&self) {
        self.phase = 0.0;
    }

    pub fn set_level(&self, l: f32) {
        self.level = l;
    }
}
