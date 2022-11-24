use std::{sync::mpsc::Sender, error::Error};

use midi_control::MidiMessage;

/// An abstraction which allows you to open a midi device and receive midi inputs from it
pub struct MidiInput {
    connection: Option<midir::MidiInputConnection<Sender<MidiMessage>>>,
}

impl MidiInput {
    /// Connect to a midi input device, writing events to the specified sender
    ///
    /// The device to be connected to can be specified by passing in a value obtained from
    /// MidiInput::devices() to the parameter `device_name`. The device will then send midi
    /// messages using the specified `sender` until its value is dropped.
    pub fn connect(client_name: &str, device_name: &str, sender: Sender<MidiMessage>)
        -> Result<Self, Box<dyn Error>>
    {
        // Create new midi input
        let midi_input = midir::MidiInput::new(client_name)?;

        // Find desired port
        let midi_ports = midi_input.ports();
        let midi_port = midi_ports
            .iter()
            .find(|port| {
                let port_name = midi_input.port_name(port);
                port_name.is_ok() && port_name.unwrap() == device_name
            })
            .ok_or(format!("Failed to find desired midi device {device_name}"))?;

        // Connect to midi port
        log::info!("Connecting to midi port: {}", device_name);
        let connection = midi_input.connect(
            midi_port,
            device_name,
            move |_timestamp, data, sender| {
                let msg = MidiMessage::from(data);
                sender
                    .send(msg)
                    .expect("Unable to send midi message");
            },
            sender)?;

        log::info!("Midi port connected");

        Ok(Self {
            connection: Some(connection),
        })
    }

    /// Get a list of all midi device names
    pub fn devices() -> Result<Vec<String>, Box<dyn Error>> {
        // Create temporary midi input
        let midi_input = midir::MidiInput::new("SubSynth_EnumerateDevices")?;

        // Enumerate devices and return port names
        let ports: Vec<String> = midi_input
        .ports()
        .iter()
        .map(|port| midi_input.port_name(&port))
        .filter_map(|res| res.ok())
        .collect();

        Ok(ports)
    }
}

impl Drop for MidiInput {
    fn drop(&mut self) {
        if let Some(connection) = self.connection.take() {
            log::info!("Closing midi connection");
            connection.close();
        }
    }
}

