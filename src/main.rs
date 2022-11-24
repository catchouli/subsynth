pub mod audio_device;
pub mod midi_device;
pub mod synth;

use std::{error::Error, thread::sleep, time::Duration};
use midi_control::MidiMessage;
use ringbuf::HeapRb;

use audio_device::AudioOutput;
use midi_device::MidiInput;
use synth::SineWaveOscillator;

/// The size of the audio buffer
const AUDIO_BUFFER_SIZE: usize = 1024;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialise logging
    env_logger::init();

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
    let _oscillator = SineWaveOscillator::new(receiver,
                                             prod,
                                             audio_output.sample_rate(),
                                             audio_output.channel_count() as u32);

    // Allow input
    log::info!("Sleeping...");
    sleep(Duration::from_secs(100));

    Ok(())
}
