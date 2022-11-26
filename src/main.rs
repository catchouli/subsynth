#![feature(inherent_associated_types)]
pub mod audio_device;
pub mod midi_device;
pub mod synth;
pub mod signal;
pub mod types;
pub mod oscillators;

use std::cell::RefCell;
use std::fmt::Debug;
use std::marker::PhantomData;
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

pub trait EventTrait<TIME, T>
where
    TIME: Clone + Debug + PartialOrd,
    T: Clone + Debug
{
    /// Call the provided callback for all events that occurred between the last update (exclusive)
    /// and current time (inclusive), or all events up to `cur_time` if `start_time` is None.
    fn process_new_events(&self, last_update: &Option<TIME>, cur_time: &TIME) -> Box<dyn Iterator<Item = (TIME, T)>>;
}

/// An event that is always firing at every new timestep.
#[derive(Clone)]
pub struct AlwaysEvent<TIME>
where
    TIME: Clone + Debug + PartialOrd
{
    phantom: PhantomData<TIME>,
}

impl<TIME> AlwaysEvent<TIME>
where
    TIME: Clone + Debug + PartialOrd
{
    pub fn new() -> Self {
        Self {
            phantom: PhantomData,
        }
    }
}

impl<TIME> EventTrait<TIME, ()> for AlwaysEvent<TIME>
where
    TIME: Clone + Debug + PartialOrd
{
    /// Call the provided callback for all events that occurred between the last update (exclusive)
    /// and current time (inclusive), or all events up to `cur_time` if `start_time` is None.
    fn process_new_events(&self, _: &Option<TIME>, cur_time: &TIME) -> Box<dyn Iterator<Item = (TIME, ())>>
    {
        Box::new(std::iter::once((cur_time.clone(), ())))
    }
}

/// An event of a given type.
#[derive(Clone)]
pub struct Event<TIME, T>
where
    TIME: Clone + Debug + PartialOrd,
    T: Clone + Debug
{
    // TODO: this list is currently unbounded.
    events: Arc<RefCell<Vec<(TIME, T)>>>,
}

impl<TIME, T> Event<TIME, T>
where
    TIME: Clone + Debug + PartialOrd,
    T: Clone + Debug
{
    /// Create a new Event of the given type, with the given initial time and value.
    pub fn new(initial_time: TIME, initial_value: T) -> Self {
        let events = vec![(initial_time, initial_value)];
        Self {
            events: Arc::new(RefCell::new(events)),
        }
    }

    /// Push a new value for the event at a given time.
    pub fn push(&mut self, time: TIME, value: T) -> Result<(), Box<dyn Error>> {
        let mut events = self.events.borrow_mut();

        if let Some((last_event_time, _)) = events.last() {
            if time < *last_event_time {
                return Err("New event time was before previous event time".into());
            }
        }

        // TODO: we might want to make sure new events pushed aren't before a previous event,
        // otherwise we'd need to re-evaluate the history.
        events.push((time, value));

        Ok(())
    }
}

impl<TIME, T> EventTrait<TIME, T> for Event<TIME, T>
where
    TIME: Clone + Debug + PartialOrd,
    T: Clone + Debug
{
    fn process_new_events(&self, last_update: &Option<TIME>, cur_time: &TIME) -> Box<dyn Iterator<Item = (TIME, T)>>
    {
        let events = self.events.borrow();

        // If a time is provided return from the next event following it, otherwise return all
        // events.
        // TODO: this is a bit inefficient since we're iterating all events every time, we could
        // probably accelerate it using a hash map to find the start index.
        if let Some(last_update) = last_update {
            Box::new(std::iter::empty())
            //for (event_time, event_value) in events.iter() {
            //    if event_time > &last_update && event_time <= &cur_time {
            //        f(event_time, event_value);
            //    }
            //}
        }
        else {
            let iter = std::iter::empty();
            Box::new(iter)
            //for (event_time, event_value) in events.iter() {
            //    if event_time > &cur_time {
            //        break;
            //    }

            //    f(event_time, event_value);
            //}
        }
    }
}

/// A signal defined by an update closure and an event. When the event has a new value, the update
/// function is called, updating the signal's value.
pub struct OnEventSignal<F, TIME, IN, OUT>
where
    F: Fn(TIME, OUT, IN) -> OUT,
    TIME: Clone + Debug + PartialOrd,
    IN: Clone + Debug,
    OUT: Clone + Debug
{
    update: F,
    cur_value: OUT,
    event: Box<dyn EventTrait<TIME, IN>>,
    last_update_time: Option<TIME>,
}

impl<F, TIME, IN, OUT> OnEventSignal<F, TIME, IN, OUT>
where
    F: Fn(TIME, OUT, IN) -> OUT,
    TIME: Clone + Debug + PartialOrd,
    IN: Clone + Debug,
    OUT: Clone + Debug
{
    /// Create a new OnEventSignal for the given event and with the given initial value and update
    /// closure.
    pub fn new(initial_value: OUT, event: Box<dyn EventTrait<TIME, IN>>, update: F) -> Self {
        Self {
            update,
            cur_value: initial_value,
            event,
            last_update_time: None,
        }
    }

    /// Evaluate the closure at the given time, updating its value if there were any new events.
    pub fn evaluate(&mut self, time: TIME) -> &OUT {
        let mut cur_value = self.cur_value.clone();
        for (time, value) in self.event.process_new_events(&self.last_update_time, &time) {
            cur_value = (self.update)(time.clone(), cur_value, value.clone());
        }
        self.cur_value = cur_value;
        self.last_update_time = Some(time);

        &self.cur_value
    }
}

pub fn never<TIME>() -> Event<TIME, ()>
where
    TIME: Clone + Debug + PartialOrd + Default
{
    Event::new(TIME::default(), ())
}

pub fn always<TIME>() -> AlwaysEvent<TIME>
where
    TIME: Clone + Debug + PartialOrd + Default
{
    AlwaysEvent::new()
}

/// Start standalone command-line synth application.
fn main() -> Result<(), Box<dyn Error>> {
    // Initialise logging.
    env_logger::init();

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

    let mut sine_signal = OnEventSignal::new(0.0, Box::new(always()), |time: f64, _: f64, _: ()| {
        let new_value = f64::sin(time);
        println!("new value for sine signal at time {}: {}", time, new_value);
        new_value
    });

    for i in 0..1000 {
        let time = i as f64 / 100.0;
        println!("Evaluating sine signal at time {time}");
        println!("Sine signal value at time {}: {}", i, sine_signal.evaluate(time));
    }

    panic!("Done");

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
