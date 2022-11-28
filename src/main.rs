pub mod audio_device;
pub mod midi_device;
pub mod synth;
pub mod signal;
pub mod types;
pub mod functions;

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::{error::Error, thread::sleep, time::Duration};
use midi_control::MidiMessage;
use functions::{midi_note_to_frequency, sine_wave};
use ringbuf::HeapRb;
use signal::Continuous;

use crate::audio_device::AudioOutput;
use crate::midi_device::MidiInput;
use crate::signal::{Discrete, lift2};
use crate::synth::MidiSynth;

/// The size of the audio buffer.
const AUDIO_BUFFER_SIZE: usize = 2048;

/// Create a simple synth network that takes a time and midi note as input and outputs a simple sine wave.
fn synth_network(input_time: &mut Discrete<f64>, input_note: &mut Discrete<u8>) -> Continuous<f64> {
    // Create time signal.
    let mut time = input_time.hold();

    // Create frequency signal.
    let mut frequency = input_note.hold().map(midi_note_to_frequency);

    // Create oscillator.
    let oscillator = lift2(time.as_mut(), frequency.as_mut(), sine_wave);

    oscillator
}

/// A standalone command-line midi synth host.
fn midi_synth_host(input_time: Discrete<f64>,
                   input_note: Discrete<u8>,
                   network: Continuous<f64>)
    -> Result<(), Box<dyn Error>>
{
    // Initialise logging.
    env_logger::init();

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
    let _midi_synth = MidiSynth::new(receiver,
                                     prod,
                                     audio_output.sample_rate() as usize,
                                     audio_output.channel_count() as usize,
                                     input_time,
                                     input_note,
                                     network);

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

/// Entry point
fn main() -> Result<(), Box<dyn Error>> {
    // Create synth network.
    let mut input_time = Discrete::<f64>::new();
    let mut input_note = Discrete::<u8>::new();

    let network = synth_network(input_time.as_mut(), input_note.as_mut());

    // Start standalone synth host.
    midi_synth_host(input_time, input_note, network)
}
