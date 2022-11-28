//! Simple synth host that samples a network and outputs samples to a ring buffer at a given sample
//! rate.

use std::collections::HashSet;
use std::{thread::JoinHandle, mem::MaybeUninit, time::Duration};
use std::sync::{Arc, atomic::{AtomicBool, Ordering}, mpsc::Receiver};

use midi_control::MidiMessage;
use ringbuf::{Producer, SharedRb};

use crate::signal::{Continuous, Discrete};

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
               mut prod: Producer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>,
               sample_rate: usize,
               channel_count: usize,
               mut input_time: Discrete<f64>,
               mut input_notes: Vec<Discrete<u8>>,
               network: Continuous<f64>)
        -> Self
    {
        log::info!("Starting midi synth thread");

        // Create atomic bool for controlling thread exit.
        let thread_run = Arc::new(AtomicBool::new(true));
        let thread_run_clone = thread_run.clone();

        // Create thread
        let time_step = 1.0 / sample_rate as f64;

        let mut time = 0.0;
        let mut voices: HashSet<u8> = HashSet::new();

        let thread_handle = std::thread::spawn(move || {
            // Run until cancellation requested.
            while thread_run_clone.load(Ordering::Relaxed) {
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
                    // A simple averaging coefficient so that the audio doesn't clip
                    // TODO: figure out the 'proper' way to mix multiple voices.
                    //let sample_coeff = if voices.is_empty() { 0.0 } else { 1.0 / voices.len() as f64 };

                    // Update time
                    time += time_step;
                    input_time.push(time);

                    // Update input for each voice
                    let voices: Vec<u8> = voices.iter().map(|x| *x).collect();
                    for (i, input_note) in input_notes.iter_mut().enumerate() {
                        if i < voices.len() {
                            input_note.push(voices[i]);
                        }
                        else {
                            input_note.push(0);
                        }
                    }

                    // Sample network
                    let sample;
                    if let Some(new_sample) = network.sample() {
                        sample = new_sample;
                    }
                    else {
                        sample = 0.0;
                    }

                    // Push one sample for each channel.
                    let mut samples = std::iter::repeat(sample as f32).take(channel_count);
                    prod.push_iter(&mut samples);
                }

                // Sleep for a few ms so we aren't just spinning.
                std::thread::sleep(THREAD_SLEEP);
            }
        });

        Self {
            thread_run,
            thread_handle: Some(thread_handle),
        }
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

