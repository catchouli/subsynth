//! Types for interacting with audio devices.

use std::{sync::Arc, error::Error, mem::MaybeUninit};
use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use ringbuf::{Consumer, SharedRb};

/// An abstraction which allows you to open an audio device and send samples to it.
pub struct AudioOutput {
    config: cpal::StreamConfig,
    stream: cpal::Stream,
}

impl AudioOutput {
    /// Connect to the default audio device with the maximum sample rate and return an AudioOutput
    /// instance allowing it to be written to.
    ///
    /// TODO: allow enumeration of devices instead of using default device.
    pub fn connect_default(mut cons: Consumer<f32, Arc<SharedRb<f32, Vec<MaybeUninit<f32>>>>>)
        -> Result<Self, Box<dyn Error>>
    {
        log::info!("Connecting to default audio device");

        // Get default host and output device.
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .ok_or("Failed to get default output device")?;

        // Get the supported output config with the max sample rate.
        let config = device
            .supported_output_configs()?
            .next()
            .ok_or("No supported output configs")?
            .with_max_sample_rate()
            .config();

        // Build output stream.
        log::info!("Building output stream");
        let stream = device.build_output_stream(
            &config,
            move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                // Feed samples from ring buffer.
                for sample in data.iter_mut() {
                    if let Some(next_sample) = cons.pop() {
                        *sample = next_sample;
                    }
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

    /// Get the sample rate of the device.
    pub fn sample_rate(&self) -> u32 {
        self.config.sample_rate.0
    }

    /// Get the number of channels the device has.
    pub fn channel_count(&self) -> u16 {
        self.config.channels
    }
}

impl Drop for AudioOutput {
    fn drop(&mut self) {
        log::info!("Closing audio device...");
        drop(&mut self.stream);
    }
}
