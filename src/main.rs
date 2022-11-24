use std::{fs::File, path::Path, error::Error, f64::consts::PI, thread::sleep, time::Duration};

use cpal::traits::{HostTrait, DeviceTrait, StreamTrait};
use midi_control::MidiMessage;
use wav::{WAV_FORMAT_IEEE_FLOAT, Header};

fn main() -> Result<(), Box<dyn Error>> {
    // Create wav header
    let header = Header::new(
        WAV_FORMAT_IEEE_FLOAT,
        1,
        44100,
        32,
    );

    // Frequency (middle C)
    let length_seconds = 8.0;
    let sample_rate = header.sampling_rate as f64;
    let sample_count = (length_seconds * sample_rate) as i64;

    let notes = [0.0, 2.0, 4.0, 5.0, 7.0, 9.0, 11.0, 12.0];
    
    // Generate samples (as f64)
    let a = f64::powf(2.0, 1.0 / 12.0);
    let samples = (0..sample_count).map(|sample_index| {
        let time = sample_index as f64 / sample_rate;
        let note = notes[f64::floor(time) as usize];
        let frequency = 261.63 * f64::powf(a, note);
        sample_sine(time, frequency)
    }).collect::<Vec<f64>>();

    // Convert samples to f32 and create wav container for them
    let samples = samples.into_iter().map(|sample| sample as f32).collect();
    let samples_container = wav::BitDepth::ThirtyTwoFloat(samples);

    // Write wav file
    let mut out_file = File::create(Path::new("out.wav"))?;
    wav::write(header, &samples_container, &mut out_file)?;

    // Open audio device
    let host = cpal::default_host();
    let device = host.default_output_device().expect("No output device available");
    let mut supported_configs_range = device.supported_output_configs()
        .expect("Error while querying configs");
    let supported_config = supported_configs_range.next()
        .expect("No supported config")
        .with_max_sample_rate();

    let config = supported_config.config();

    let channel_count = config.channels;
    let sample_rate = config.sample_rate.0;

    let (sender, receiver) = std::sync::mpsc::channel::<MidiMessage>();

    let mut sample_index = 0;
    let mut midi_note: Option<u8> = None;
    let stream = device.build_output_stream(
        &config,
        move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
            while let Ok(msg) = receiver.try_recv() {
                match msg {
                    MidiMessage::NoteOn(_, e) => {
                        println!("Got note down: {}", e.key);
                        //if midi_note.is_none() {
                            midi_note = Some(e.key);
                        //}
                    },
                    MidiMessage::NoteOff(_, e) => {
                        println!("Got note up: {}", e.key);
                        if let Some(cur_key) = midi_note {
                            if cur_key == e.key {
                                midi_note = None;
                            }
                        }
                    },
                    _ => {}
                }
            }

            for frame in data.chunks_mut(channel_count as usize) {
                let time: f64 = sample_index as f64 / sample_rate as f64;
                //let note = notes[f64::floor(time) as usize];
                //let frequency = 261.63 * f64::powf(a, note);
                for sample in frame.iter_mut() {
                    if let Some(midi_note) = midi_note {
                        let freq = 440.0 * f64::powf(2.0, (midi_note as f64 - 69.0) / 12.0);
                        *sample = sample_sine(time, freq) as f32;
                    }
                    else {
                        *sample = 0.0;
                    }
                }
                sample_index += 1;
            }
        },
        move |err| {
            println!("Stream callback: {:?}", err);
        })?;

    // Midi device
    let midi_input = midir::MidiInput::new("Test")?;

    println!("Enumerating midi devices");
    for port in midi_input.ports() {
        if let Ok(port_name) = midi_input.port_name(&port) {
            println!("Midi device: {port_name}");
        }
    }

    let ports = midi_input.ports();
    let port = ports.first();
    let port = port.as_ref().unwrap();
    let port_name = midi_input.port_name(&port)?;
    println!("Connecting to midi port: {}", port_name);
    let _connect_in = midi_input.connect(
        port,
        &port_name,
        move |_timestamp, data, sender| {
            let msg = MidiMessage::from(data);
            sender.send(msg).unwrap();
        },
        sender,
    );
    
    // Start stream
    println!("Playing stream");
    stream.play()?;

    println!("Sleeping for 8 seconds...");
    sleep(Duration::from_secs(100));

    println!("Done, pausing stream");
    stream.pause()?;

    Ok(())
}

fn sample_sine(time: f64, frequency: f64) -> f64 {
    f64::sin(2.0 * PI * time * frequency)
}
