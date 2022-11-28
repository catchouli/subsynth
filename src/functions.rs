//! Basic functions, such as shape generators.

use std::f64::consts::PI;

/// Convert a midi note to a frequency, with note 60 as middle C, tuned equal temperament to A 440.
pub fn midi_note_to_frequency(midi_note: u8) -> f64 {
    440.0 * f64::powf(2.0, (midi_note as f64 - 69.0) / 12.0)
}

/// Generate a sine wave of a given frequency at a given time.
pub fn sine_wave(time: f64, frequency: f64) -> f64 {
    let ft = frequency * time;
    f64::sin(2.0 * PI * ft)
}

/// Generate a square wave of a given frequency at a given time.
pub fn square_wave(time: f64, frequency: f64) -> f64 {
    // https://en.wikipedia.org/wiki/Square_wave#Definitions
    let ft = frequency * time;
    (2.0 * f64::floor(ft) - f64::floor(2.0 * ft)) * 2.0 + 1.0
}

/// Generate a triangle wave of a given frequency at a given time.
pub fn triangle_wave(time: f64, frequency: f64) -> f64 {
    // https://en.wikipedia.org/wiki/Triangle_wave#Definition
    let ft = frequency * time;
    2.0 * f64::abs(ft - f64::floor(ft + 0.5))
}

/// Generate a sawtooth wave of a given frequency at a given time.
pub fn saw_wave(time: f64, frequency: f64) -> f64 {
    // https://en.wikipedia.org/wiki/Sawtooth_wave
    let ft = frequency * time;
    ft - f64::floor(ft)
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::{assert_relative_eq, relative_ne};

    #[test]
    fn test_midi_note_to_frequency() {
        // 69 should be exactly 440 as it's the reference pitch.
        assert_eq!(midi_note_to_frequency(69), 440.0);

        // 60 should be middle C, and we can check the other Cs in different octaves too to check
        // it doesn't get inaccurate as the value increases/decreases.
        assert_relative_eq!(midi_note_to_frequency(60), 261.63, epsilon = 0.005); //< C4
        assert_relative_eq!(midi_note_to_frequency(12),  16.35, epsilon = 0.005); //< C0
        assert_relative_eq!(midi_note_to_frequency(24),  32.70, epsilon = 0.005); //< C1
        assert_relative_eq!(midi_note_to_frequency(36),  65.41, epsilon = 0.005); //< C2
        assert_relative_eq!(midi_note_to_frequency(48), 130.81, epsilon = 0.005); //< C3
        assert_relative_eq!(midi_note_to_frequency(72), 523.25, epsilon = 0.005); //< C5
        assert_relative_eq!(midi_note_to_frequency(84), 1046.5, epsilon = 0.005); //< C6
        assert_relative_eq!(midi_note_to_frequency(96), 2093.0, epsilon = 0.005); //< C7

        // And let's just check the rest of octave 4 to make sure it's in tune.
        assert_relative_eq!(midi_note_to_frequency(61), 277.18, epsilon = 0.005); //< C#/Db
        assert_relative_eq!(midi_note_to_frequency(62), 293.66, epsilon = 0.005); //< D4
        assert_relative_eq!(midi_note_to_frequency(63), 311.13, epsilon = 0.005); //< D#/Eb
        assert_relative_eq!(midi_note_to_frequency(64), 329.63, epsilon = 0.005); //< E4
        assert_relative_eq!(midi_note_to_frequency(65), 349.23, epsilon = 0.005); //< F4
        assert_relative_eq!(midi_note_to_frequency(66), 369.99, epsilon = 0.005); //< F#/Gb
        assert_relative_eq!(midi_note_to_frequency(67), 392.00, epsilon = 0.005); //< G4
        assert_relative_eq!(midi_note_to_frequency(68), 415.30, epsilon = 0.005); //< G#/Ab
        assert_relative_eq!(midi_note_to_frequency(70), 466.16, epsilon = 0.005); //< A#/Bb
        assert_relative_eq!(midi_note_to_frequency(71), 493.88, epsilon = 0.005); //< B4
    }

    #[test]
    fn test_sine_wave() {
        const ONE_OVER_2_PI: f64 = 1.0 / (2.0 * PI);

        // A sine wave of frequency 1/(2PI) (aka with a period of 2PI) should just be a normal sine function.
        for i in 0..100 {
            let time = f64::from(i) * 0.1;
            assert_relative_eq!(sine_wave(time, ONE_OVER_2_PI), f64::sin(time), epsilon = 10e-10);
        }

        // These boundary conditions should hold true for the above wave.
        assert_eq!(sine_wave(0.0, ONE_OVER_2_PI), 0.0);
        assert_eq!(sine_wave(PI / 2.0, ONE_OVER_2_PI), 1.0);
        assert_relative_eq!(sine_wave(PI, ONE_OVER_2_PI), 0.0, epsilon = 10e-10);
        assert_eq!(sine_wave(PI * 3.0 / 2.0, ONE_OVER_2_PI), -1.0);
        assert_relative_eq!(sine_wave(2.0 * PI, ONE_OVER_2_PI), 0.0, epsilon = 10e-10);
        // These ones shouldn't though, but will if we double the frequency.
        assert_ne!(sine_wave(PI / 4.0, ONE_OVER_2_PI), 1.0);
        assert_ne!(sine_wave(PI / 2.0, ONE_OVER_2_PI), 0.0);
        assert_ne!(sine_wave(PI * 3.0 / 4.0, ONE_OVER_2_PI), -1.0);

        // Doubling the frequency should half the period of the wave form (and increase the octave).
        assert_eq!(sine_wave(0.0, 2.0 * ONE_OVER_2_PI), 0.0);
        assert_eq!(sine_wave(PI / 4.0, 2.0 * ONE_OVER_2_PI), 1.0);
        assert_relative_eq!(sine_wave(PI / 2.0, 2.0 * ONE_OVER_2_PI), 0.0);
        assert_eq!(sine_wave(PI * 3.0 / 4.0, 2.0 * ONE_OVER_2_PI), -1.0);
        assert_relative_eq!(sine_wave(PI, 2.0 * ONE_OVER_2_PI), 0.0, epsilon = 10e-10);
    }

    #[test]
    fn test_square_wave() {
        for i in 0..5000 {
            const FREQUENCY: f64 = 100.0;
            let time = f64::from(i) * 0.001;

            // A square wave should be the sign of a sine wave. If the sine is very close to 0 then the
            // exact crossing time for each wave may not match our sample rate and produce false negatives
            // so we filter those out.
            let sine = sine_wave(time, FREQUENCY);
            if relative_ne!(sine, 0.0, epsilon = 1e10) {
                let square = square_wave(time, FREQUENCY);
                assert_eq!(square, sine.signum());
            }
        }
    }
}
