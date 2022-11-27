//! Basic oscillators that can output simple shapes like sine waves.

use std::f64::consts::PI;

use crate::{types::*, signal::OldSignal};

/// A simple sine wave oscillator that can be sampled at a given time and frequency.
pub struct SineWaveOscillator {
}

impl SineWaveOscillator {
    /// Create a new sine wave oscillator with the given frequency.
    pub fn new() -> Self {
        Self {}
    }

    /// Sample a sine wave at a given time and frequency.
    fn sample_sine(time: Time, frequency: Frequency) -> Sample {
        f64::sin(2.0 * PI * time * frequency)
    }
}

impl OldSignal<(Time, Frequency), Sample> for SineWaveOscillator {
    fn evaluate(&mut self, (time, frequency): (Time, Frequency)) -> Sample {
        SineWaveOscillator::sample_sine(time, frequency)
    }
}

