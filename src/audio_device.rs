use std::{sync::mpsc::Receiver, error::Error, f64::consts::PI};

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use midi_control::MidiMessage;

/// An abstraction which allows you to open an audio device and send samples to it
pub struct AudioOutput {
    config: cpal::StreamConfig,
    stream: cpal::Stream,
}

impl AudioOutput {
    /// Connect to the default audio device with the maximum sample rate and return an AudioOutput
    /// instance allowing it to be written to
    ///
    /// TODO: allow enumeration of devices instead of using default device
    pub fn connect_default(receiver: Receiver<MidiMessage>) -> Result<Self, Box<dyn Error>> {
        log::info!("Connecting to default audio device");

        // Get default host and output device
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("Failed to get default output device")?;

        // Get the supported output config with the max sample rate
        let config = device
            .supported_output_configs()?
            .next()
            .ok_or("No supported output configs")?
            .with_max_sample_rate()
            .config();

        // Build output stream
        log::info!("Building output stream");
        let mut sample_index = 0;
        let mut midi_note: Option<u8> = None;
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
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

                for frame in data.chunks_mut(config.channels as usize) {
                    let time: f64 = sample_index as f64 / config.sample_rate.0 as f64;
                    for sample in frame.iter_mut() {
                        if let Some(midi_note) = midi_note {
                            let freq = 440.0 * f64::powf(2.0, (midi_note as f64 - 69.0) / 12.0);
                            *sample = Self::sample_sine(time, freq) as f32;
                        }
                        else {
                            *sample = 0.0;
                        }
                    }
                    sample_index += 1;
                }
            },
            move |err| {
                log::info!("Stream error: {:?}", err);
            })?;

        log::info!("Starting output stream...");
        stream.play()?;

        Ok(Self {
            config,
            stream,
        })
    }

    /// Get the sample rate of the device
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    /// Get the number of channels the device has
    pub fn channel_count(&self) -> u16 {
        self.config.channels
    }

    /// Sample a sine wave at a given time and frequency
    fn sample_sine(time: f64, frequency: f64) -> f64 {
        f64::sin(2.0 * PI * time * frequency)
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        drop(&mut self.stream);
    }
}
