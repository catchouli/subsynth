use std::{thread::JoinHandle, mem::MaybeUninit, time::Duration, f64::consts::PI};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::Receiver};

use midi_control::MidiMessage;
use ringbuf::{Producer, SharedRb};

/// A sine wave oscillator that accepts midi input and outputs a monophonic pure sine wave tone
pub struct SineWaveOscillator {
    thread_run: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl SineWaveOscillator {
    /// Create a new sine wave oscillator controlled by midi messages
    pub fn new(receiver: Receiver<MidiMessage>,
               mut prod: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
               sample_rate: u32,
               channel_count: u32)
        -> Self
    {
        let samples_per_second = sample_rate * channel_count;
        let time_step = 1.0 / samples_per_second as f64;

        // Start thread
        log::info!("Starting oscillator thread");
        let thread_run_local = Arc::new(AtomicBool::new(true));
        let thread_run = thread_run_local.clone();

        let thread_handle = std::thread::spawn(move || {
            let mut midi_note = None;
            let mut time = 0.0;

            // Run until cancellation requested
            while thread_run.load(Ordering::SeqCst) {
                // Receive new midi note
                while let Ok(msg) = receiver.try_recv() {
                    match msg {
                        MidiMessage::NoteOn(_, e) => {
                            log::info!("Got note down: {}", e.key);
                            midi_note = Some(e.key);
                        },
                        MidiMessage::NoteOff(_, e) => {
                            log::info!("Got note up: {}", e.key);
                            if let Some(cur_key) = midi_note {
                                if cur_key == e.key {
                                    midi_note = None;
                                }
                            }
                        },
                        _ => {}
                    }
                }

                // Fill audio buffer
                while prod.free_len() > channel_count as usize {
                    if let Some(midi_note) = midi_note {
                        // Update time
                        // TODO: this is maybe a bit weird, since we could be putting different
                        // time samples in different channels. Ideally we'd generate the samples
                        // for each channel at the same time.
                        time += time_step;

                        // Calculate frequency and sample sine wave
                        let freq = 440.0 * f64::powf(2.0, (midi_note as f64 - 69.0) / 12.0);
                        let sample = SineWaveOscillator::sample_sine(time, freq);

                        prod.push(sample as f32).unwrap();
                    }
                    else {
                        prod.push(0.0).unwrap();
                    }
                }
                std::thread::sleep(Duration::from_millis(5));
            }
        });

        Self {
            thread_run: thread_run_local,
            thread_handle: Some(thread_handle),
        }
    }

    /// Sample a sine wave at a given time and frequency
    fn sample_sine(time: f64, frequency: f64) -> f64 {
        f64::sin(2.0 * PI * time * frequency)
    }
}

impl Drop for SineWaveOscillator {
    fn drop(&mut self) {
        log::info!("Waiting for oscillator thread to exit...");
        self.thread_run.store(false, Ordering::SeqCst);
        if let Some(thread_handle) = std::mem::take(&mut self.thread_handle) {
            thread_handle.join().unwrap();
        }
    }
}

