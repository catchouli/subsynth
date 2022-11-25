//! Simple synth host that samples a network and outputs samples to a ring buffer at a given sample
//! rate.

use std::collections::HashSet;
use std::{thread::JoinHandle, mem::MaybeUninit, time::Duration};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::Receiver};

use midi_control::MidiMessage;
use ringbuf::{Producer, SharedRb};

use crate::signal::Signal;
use crate::types::*;

/// The amount of time for the thread to sleep between processing new midi inputs and re-filling
/// the output ringbuffer.
const THREAD_SLEEP: Duration = Duration::from_millis(1);

/// A midi synth that accepts midi input and samples one or more oscillators to produce audio samples.
pub struct MidiSynth {
    thread_run: Arc<AtomicBool>,
    thread_handle: Option<JoinHandle<()>>,
}

impl MidiSynth {
    /// Create a new midi synth controlled by midi messages, producing samples to the
    /// given ring buffer, at the given sample rate and number of channels.
    pub fn new(receiver: Receiver<MidiMessage>,
               prod: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
               sample_rate: usize,
               channel_count: usize,
               network: Box<dyn Signal<(Frequency, Time), Sample>>)
        -> Self
    {
        log::info!("Starting oscillator thread");

        // Create atomic bool for controlling thread exit.
        let thread_run = Arc::new(AtomicBool::new(true));

        // Spawn worker thread.
        let thread_handle = Some(Self::spawn_thread(
            receiver,
            prod,
            sample_rate,
            channel_count,
            network,
            thread_run.clone()
        ));

        Self {
            thread_run,
            thread_handle,
        }
    }

    /// Spawn the sampling thread.
    fn spawn_thread(receiver: Receiver<MidiMessage>,
                    mut prod: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
                    sample_rate: usize,
                    channel_count: usize,
                    mut network: Box<dyn Signal<(Frequency, Time), Sample>>,
                    thread_run: Arc<AtomicBool>)
        -> JoinHandle<()>
    {
        let time_step = 1.0 / sample_rate as f64;

        let mut time = 0.0;
        let mut voices: HashSet<u8> = HashSet::new();

        std::thread::spawn(move || {
            // Run until cancellation requested.
            while thread_run.load(Ordering::SeqCst) {
                // Receive new midi notes.
                while let Ok(msg) = receiver.try_recv() {
                    match msg {
                        MidiMessage::NoteOn(_, e) => {
                            log::debug!("Got note down: {}", e.key);
                            voices.insert(e.key);
                        },
                        MidiMessage::NoteOff(_, e) => {
                            log::debug!("Got note up: {}", e.key);
                            voices.remove(&e.key);
                        },
                        _ => {}
                    }
                }

                // Fill audio buffer.
                while prod.free_len() > channel_count {
                    let mut sample = 0.0;

                    // A simple averaging coefficient so that the audio doesn't clip
                    // TODO: figure out the 'proper' way to mix multiple voices.
                    let sample_coeff = if voices.is_empty() { 0.0 } else { 1.0 / voices.len() as f64 };

                    // Update time
                    time += time_step;

                    for midi_note in &voices {
                        // Calculate frequency of midi note.
                        let freq = 440.0 * f64::powf(2.0, (*midi_note as f64 - 69.0) / 12.0);

                        // Sample the network at multiple octaves.
                        const OCTAVES: usize = 7;
                        let mut amplitude = 0.5;
                        let mut octave_freq = freq;

                        for _ in 0..OCTAVES {
                            sample += network.evaluate((time, octave_freq)) * amplitude * sample_coeff;
                            amplitude *= 0.3;
                            octave_freq *= 2.0;
                        }
                    }

                    // Push one sample for each channel.
                    let mut samples = std::iter::repeat(sample as f32).take(channel_count);
                    prod.push_iter(&mut samples);
                }

                // Sleep for a few ms so we aren't just spinning.
                std::thread::sleep(THREAD_SLEEP);
            }
        })
    }
}

impl Drop for MidiSynth {
    fn drop(&mut self) {
        log::info!("Waiting for midi synth thread to exit...");
        self.thread_run.store(false, Ordering::SeqCst);
        if let Some(thread_handle) = std::mem::take(&mut self.thread_handle) {
            thread_handle.join().unwrap();
        }
    }
}

