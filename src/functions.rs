//! Basic functions, such as shape generators.

use std::f64::consts::PI;

pub fn midi_note_to_frequency(midi_note: u8) -> f64 {
    440.0 * f64::powf(2.0, (midi_note as f64 - 69.0) / 12.0)
}

pub fn sine_wave(time: f64, frequency: f64) -> f64 {
    f64::sin(2.0 * PI * time * frequency)
}
