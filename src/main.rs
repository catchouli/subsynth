pub mod audio_device;
pub mod midi_device;

use std::{error::Error, thread::sleep, time::Duration};
use midi_control::MidiMessage;

use audio_device::AudioOutput;
use midi_device::MidiInput;

fn main() -> Result<(), Box<dyn Error>> {
    // Initialise logging
    env_logger::init();

    // Create mpsc channel for midi data
    let (sender, receiver) = std::sync::mpsc::channel::<MidiMessage>();

    // Connect to audio output device
    let _audio_output = AudioOutput::connect_default(receiver)?;

    // Enumerate midi devices
    log::info!("Enumerating midi devices:");
    let midi_devices = MidiInput::devices()?;
    midi_devices.iter().for_each(|device| log::info!("    {device}"));

    // Connect to midi input
    let midi_device = midi_devices.first().expect("Failed to find midi input");
    log::info!("Attempting to connect to midi device: {midi_device}");
    let mut _midi_input = MidiInput::connect("SubSynth", midi_device, sender)?;

    // Allow input
    log::info!("Sleeping...");
    sleep(Duration::from_secs(100));

    Ok(())
}
