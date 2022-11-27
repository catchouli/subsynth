//! A simple frp-like signals implementation.

use std::{f64::consts::PI, sync::{Arc, Mutex}};

type CallbackClosure = Box<dyn FnMut() -> () + Send + Sync + 'static>;

/// A trait for "signals" which have an input and an output type, and can be evaluated for the
/// given input.
pub trait OldSignal<In, Out>: Send {
    /// Evaluate this signal for the given input, yielding an output.
    fn evaluate(&mut self, input: In) -> Out;
}

#[derive(Clone)]
struct SignalBase<T> {
    value: Arc<Mutex<Option<T>>>,
    dependents: Arc<Mutex<Vec<CallbackClosure>>>,
}

impl<T: Clone + PartialEq> SignalBase<T> {
    fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(None)),
            dependents: Arc::new(Mutex::new(Vec::new())),
        }
    }

    fn attach<F>(&mut self, closure: F)
        where F: FnMut() -> () + Send + Sync + 'static
    {
        self.dependents.lock().unwrap().push(Box::new(closure));
    }

    fn set(&mut self, value: T) {
        let mut cur_value = self.value.lock().unwrap();
        if cur_value.as_ref() != Some(&value) {
            cur_value.replace(value);

            // Drop the borrow explicitly so it can be reborrowed in callbacks
            drop(cur_value);

            let mut dependents = self.dependents.lock().unwrap();
            for notify_dependent in dependents.iter_mut() {
                notify_dependent();
            }
        }
    }

    fn get(&self) -> Option<T> {
        self.value.lock().unwrap().clone()
    }
}

#[derive(Clone)]
pub struct Discrete<T> {
    base: SignalBase<T>,
}

impl<T> Discrete<T>
where
    T: Clone + PartialEq + Send + Sync + 'static
{
    pub fn new() -> Self {
        Self {
            base: SignalBase::new(),
        }
    }

    pub fn push(&mut self, value: T) {
        self.base.set(value);
    }

    pub fn hold(&mut self) -> Continuous<T> {
        Continuous::new1(&mut self.base, |a| a)
    }
}

#[derive(Clone)]
pub struct Continuous<T> {
    base: SignalBase<T>,
}

impl<T> Continuous<T>
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    fn new1<A, F>(parent: &mut SignalBase<A>, update: F) -> Self
    where
        A: Clone + PartialEq + Send + Sync + 'static,
        F: Fn(A) -> T + Send + Sync + 'static,
    {
        let signal = Continuous {
            base: SignalBase::new(),
        };

        let mut signal_clone = signal.clone();
        let parent_clone = parent.clone();
        parent.attach(move || {
            if let Some(value) = parent_clone.get() {
                signal_clone.base.set(update(value));
            }
        });

        signal
    }

    fn new2<A, B, F>(parent_a: &mut SignalBase<A>, parent_b: &mut SignalBase<B>, update: F) -> Self
    where
        A: Clone + PartialEq + Send + Sync + 'static,
        B: Clone + PartialEq + Send + Sync + 'static,
        F: Fn(A, B) -> T + Clone + Send + Sync + 'static,
    {
        let signal = Continuous {
            base: SignalBase::new(),
        };

        let parent_a_clone = parent_a.clone();
        let parent_b_clone = parent_b.clone();
        let mut new_signal = signal.clone();

        let update_closure = move || {
            match (parent_a_clone.get(), parent_b_clone.get()) {
                (Some(a), Some(b)) => {
                    new_signal.base.set(update(a, b));                
                },
                _ => {}
            }
        };

        parent_a.attach(update_closure.clone());
        parent_b.attach(update_closure);

        signal
    }

    pub fn sample(&self) -> Option<T> {
        self.base.get()
    }

    pub fn map<F, B>(&mut self, closure: F) -> Continuous<B>
    where
        B: Clone + PartialEq + Send + Sync + 'static,
        F: Fn(T) -> B + Send + Sync + 'static,
    {
        lift1(self, closure)
    }
}

pub fn lift1<F, A, B>(signal: &mut Continuous<A>, closure: F) -> Continuous<B>
where
    A: Clone + PartialEq + Send + Sync + 'static,
    B: Clone + PartialEq + Send + Sync + 'static,
    F: Fn(A) -> B + Send + Sync + 'static,
{
    Continuous::new1(&mut signal.base, closure)
}

pub fn lift2<F, A, B, C>(signal_a: &mut Continuous<A>, signal_b: &mut Continuous<B>, closure: F) -> Continuous<C>
where
    A: Clone + PartialEq + Send + Sync + 'static,
    B: Clone + PartialEq + Send + Sync + 'static,
    C: Clone + PartialEq + Send + Sync + 'static,
    F: Fn(A, B) -> C + Clone + Send + Sync + 'static,
{
    Continuous::new2(&mut signal_a.base, &mut signal_b.base, closure)
}

// How to use
pub fn test() {
    let mut time: Discrete<f64> = Discrete::new();
    let mut frequency: Discrete<f64> = Discrete::new();

    let time_hold: Continuous<f64> = time.hold();

    let sine_oscillator: Continuous<f64> = lift2(&mut time.hold(), &mut frequency.hold(), |time: f64, frequency: f64| {
        f64::sin(2.0 * PI * time * frequency)
    });

    // For each time step
    for i in 0..100 {
        // Update time
        let new_time = i as f64 / 100.0;
        time.push(new_time);

        // Push new frequency if it's changed, comes from external midi input
        frequency.push(261.0);

        // Sample oscillator
        let sample = sine_oscillator.sample();
        let time_sample = time_hold.sample();
        log::info!("Time = {time_sample:?}, sample = {sample:?}");
    }
}
