#![feature(associated_type_defaults)]

pub mod audio_device;
pub mod midi_device;
pub mod synth;
pub mod signal;
pub mod types;
pub mod oscillators;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{error::Error, thread::sleep, time::Duration};
use midi_control::MidiMessage;
use ringbuf::HeapRb;

use crate::audio_device::AudioOutput;
use crate::midi_device::MidiInput;
use crate::oscillators::SineWaveOscillator;
use crate::synth::MidiSynth;

/// The size of the audio buffer.
const AUDIO_BUFFER_SIZE: usize = 2048;

/// Start standalone command-line synth application.
fn main() -> Result<(), Box<dyn Error>> {
    // Initialise logging.
    env_logger::init();

    signal::test();
    panic!("Done");

    // Create synth network.
    //let mut evt_freq = Event::<f64, Frequency>::new(0.0, 0.0);
    //evt_freq.push(0.0, 1.0)?;
    //evt_freq.push(0.0, 2.0)?;
    //evt_freq.push(1.0, 3.0)?;
    //evt_freq.push(2.0, 4.0)?;
    //evt_freq.push(9.0, 100.0)?;

    //let mut signal = OnEventSignal::new(0.0, evt_freq, |time: f64, cur_value: f64, event: Frequency| {
    //    let new_value = cur_value + event;
    //    println!("Updating signal value from {cur_value} to {new_value} at time {time}");
    //    new_value
    //});

    //for i in 0..10 {
    //    println!("Evaluating signal at time {i}");
    //    println!("Signal value at time {}: {}", i, signal.evaluate(i as f64));
    //}

    //let mut sine_signal = OnEventSignal::new(0.0, Box::new(always()), |time: f64, _: f64, _: ()| {
    //    let new_value = f64::sin(time);
    //    println!("new value for sine signal at time {}: {}", time, new_value);
    //    new_value
    //});

    //for i in 0..1000 {
    //    let time = i as f64 / 100.0;
    //    println!("Evaluating sine signal at time {time}");
    //    println!("Sine signal value at time {}: {}", i, sine_signal.evaluate(time));
    //}

    //panic!("Done");

    let network = SineWaveOscillator::new();

    // Create mpsc channel for midi data.
    let (sender, receiver) = std::sync::mpsc::channel::<MidiMessage>();

    // Create audio ring buffer.
    let (prod, cons) = HeapRb::<f32>::new(AUDIO_BUFFER_SIZE).split();

    // Connect to audio output device.
    let audio_output = AudioOutput::connect_default(cons)?;

    // Enumerate midi devices.
    log::info!("Enumerating midi devices:");
    let midi_devices = MidiInput::devices()?;
    midi_devices.iter().for_each(|device| log::info!("    {device}"));

    // Connect to midi input.
    let midi_device = midi_devices.first().expect("Failed to find midi input");
    log::info!("Attempting to connect to midi device: {midi_device}");
    let mut _midi_input = MidiInput::connect("SubSynth", midi_device, sender)?;

    // Create sine wave oscillator.
    let _oscillator = MidiSynth::new(receiver,
                                     prod,
                                     audio_output.sample_rate() as usize,
                                     audio_output.channel_count() as usize,
                                     Box::new(network));

    // Register ctrl-c handler for clean exit.
    let should_exit = Arc::new(AtomicBool::new(false));
    signal_on_ctrlc(should_exit.clone())?;

    // Allow input.
    log::info!("Running... press ctrl-C to exit.");
    while !should_exit.load(Ordering::Relaxed) {
        sleep(Duration::from_millis(100));
    }

    log::info!("Exit requested");
    Ok(())
}

/// Adds a handler for ctrl-c that signals that it has been pressed using the given AtomicBool.
fn signal_on_ctrlc(ctrlc_sent: Arc<AtomicBool>) -> Result<(), Box<dyn Error>> {
    ctrlc_sent.store(false, Ordering::Relaxed);

    ctrlc::set_handler(move || {
        if !ctrlc_sent.load(Ordering::Relaxed) {
            // If this is the first time, signal for exit.
            ctrlc_sent.store(true, Ordering::Relaxed);
        }
        else {
            // If ctrl-c is sent more than once, exit immediately.
            log::info!("Immediate exit requested");
            std::process::exit(1);
        }
    })?;

    Ok(())
}
