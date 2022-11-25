pub mod audio_device;
pub mod midi_device;
pub mod synth;
pub mod signal;
pub mod types;
pub mod oscillators;

use std::{error::Error, thread::sleep, time::Duration};
use midi_control::MidiMessage;
use ringbuf::HeapRb;

use crate::audio_device::AudioOutput;
use crate::midi_device::MidiInput;
use crate::oscillators::SineWaveOscillator;
use crate::synth::MidiSynth;

/// The size of the audio buffer
const AUDIO_BUFFER_SIZE: usize = 1024;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialise logging
    env_logger::init();

    // Create synth network
    let network = SineWaveOscillator::new();

    // Create mpsc channel for midi data
    let (sender, receiver) = std::sync::mpsc::channel::<MidiMessage>();

    // Create audio ring buffer
    let (prod, cons) = HeapRb::<f32>::new(AUDIO_BUFFER_SIZE).split();

    // Connect to audio output device
    let audio_output = AudioOutput::connect_default(cons)?;

    // Enumerate midi devices
    log::info!("Enumerating midi devices:");
    let midi_devices = MidiInput::devices()?;
    midi_devices.iter().for_each(|device| log::info!("    {device}"));

    // Connect to midi input
    let midi_device = midi_devices.first().expect("Failed to find midi input");
    log::info!("Attempting to connect to midi device: {midi_device}");
    let mut _midi_input = MidiInput::connect("SubSynth", midi_device, sender)?;

    // Create sine wave oscillator
    let _oscillator = MidiSynth::new(receiver,
                                     prod,
                                     audio_output.sample_rate() as usize,
                                     audio_output.channel_count() as usize,
                                     Box::new(network));

    // Allow input
    log::info!("Sleeping...");
    sleep(Duration::from_secs(100));

    Ok(())
}
