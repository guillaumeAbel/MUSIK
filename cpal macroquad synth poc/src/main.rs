use std::f32::consts::PI;
use cpal::{SupportedBufferSize, BufferSize, traits::{DeviceTrait, HostTrait, StreamTrait}};
use macroquad::prelude::*;
use ringbuf::{traits::*, HeapRb, HeapProd};

const VISUAL_BUFFER_SIZE: usize = 512;

fn start_audio(mut producer: HeapProd<f32>) -> cpal::Stream {
    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .expect("Aucun périphérique de sortie audio");
    let mut supported = device.supported_output_configs().expect("output config");
    let supported_config = supported
        .find(|c| c.sample_format() == cpal::SampleFormat::F32)
        .unwrap_or_else(|| supported.next().expect("Pas de format audio disponible"))
        .with_max_sample_rate();

    let supported_buffer = supported_config.buffer_size();
    let mut config = supported_config.config();
    config.buffer_size = match supported_buffer {
        SupportedBufferSize::Range{min:x, max:_} => BufferSize::Fixed(*x),
        SupportedBufferSize::Unknown => BufferSize::Default
    };
    let sample_rate = config.sample_rate.0 as f32;
    let channels = config.channels as usize;

    let freq_hz = 440.0f32;
    let level_db = -12.0f32;
    let gain = 10f32.powf(level_db / 20.0);

    let mut phase = 0.0f32;
    let err_fn = |err| eprintln!("Erreur sur le stream: {err}");

    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _| {
            let phase_inc = freq_hz / sample_rate;
            for frame in data.chunks_mut(channels) {
                let s = (2.0 * PI * phase).sin() * gain;

                let _ = producer.try_push(s);

                phase += phase_inc;
                if phase >= 1.0 {
                    phase -= 1.0;
                }

                for ch in frame.iter_mut() {
                    *ch = s;
                }
            }
        },
        err_fn,
        None,
    ).expect("stream");

    stream.play().expect("play");
    stream
}

fn draw_waveform(samples: &[f32]) {
    let w = screen_width();
    let h = screen_height();
    let mid_y = h * 0.5;
    let margin = 20.0;
    let amp_pixels = (h * 0.5) - margin;

    if samples.len() < 2 {
        return;
    }

    for i in 0..(samples.len() - 1) {
        let x1 = (i as f32 / (samples.len() - 1) as f32) * w;
        let x2 = ((i + 1) as f32 / (samples.len() - 1) as f32) * w;

        let y1 = mid_y - samples[i] * amp_pixels;
        let y2 = mid_y - samples[i + 1] * amp_pixels;

        draw_line(x1, y1, x2, y2, 2.0, GREEN);
    }
}

#[macroquad::main("CPAL + Macroquad Sine Wave")]
async fn main() {
    let rb = HeapRb::<f32>::new(VISUAL_BUFFER_SIZE);
    let (prod, mut cons) = rb.split();

    let _stream = start_audio(prod);

    loop {
        clear_background(BLACK);

        let mut samples = Vec::with_capacity(VISUAL_BUFFER_SIZE);
        while let Some(s) = cons.try_pop() {
            samples.push(s);
        }

        draw_waveform(&samples);

        draw_text(
            "Sinus 440 Hz -12 dBFS",
            20.0,
            30.0,
            24.0,
            WHITE,
        );

        next_frame().await;
    }
}
