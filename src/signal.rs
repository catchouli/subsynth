//! A simple push-frp-like signals implementation, implemented with callbacks.

use std::sync::{Arc, Mutex};

/// A callback for signals to notify their subscribers that their value has been updated.
type CallbackClosure = Box<dyn FnMut() -> () + Send + Sync + 'static>;

/// The "base" component of Discrete and Continous signals, which is basically a thread-safe value
/// holder, which also holds references to the update callbacks of those dependent on it.
///
/// For example, the signal `input_time: Discrete<f64>` which holds the input time will have one of
/// these, and will also have references to the callback closures for all derived signals.
///
/// This allows the Discrete<T> and Continuous<T> signals themselves to be cloneable and threadsafe
/// without the user having to be aware of Arcs or Mutexes etc.
///
/// We have two separate mutexes for the value and subscribers list, as subscribers will need to
/// access our value while we're notifying them of changes.
#[derive(Clone)]
struct SignalBase<T>
{
    value: Arc<Mutex<Option<T>>>,
    subscribers: Arc<Mutex<Vec<CallbackClosure>>>,
}

impl<T: Clone + PartialEq> SignalBase<T> {
    /// Create a new SignalBase component with a current value of None and an empty list of
    /// subscribers.
    fn new() -> Self {
        Self {
            value: Arc::new(Mutex::new(None)),
            subscribers: Arc::new(Mutex::new(Vec::new())),
        }
    }

    /// Attach a new dependent signal to this one so it will be notified whenever there's a change
    /// in value.
    /// TODO: we should probably have anything that attaches also detach automatically when it's dropped.
    fn attach<F>(&mut self, closure: F)
        where F: FnMut() -> () + Send + Sync + 'static
    {
        self.subscribers.lock()
            .expect("Failed to acquire lock to attach to signal")
            .push(Box::new(closure));
    }

    /// Update the value of this signal, and if the value is different, notify all subscribers that
    /// it's changed.
    fn set(&mut self, value: T) {
        let mut cur_value = self.value.lock()
            .expect("Failed to acquire lock to set signal value");
        if cur_value.as_ref() != Some(&value) {
            cur_value.replace(value);

            // Drop the lock so it can be locked in callbacks.
            std::mem::drop(cur_value);

            let mut subscribers = self.subscribers.lock()
                .expect("Failed to lock subscribers list to notify of updates");
            for notify_dependent in subscribers.iter_mut() {
                notify_dependent();
            }
        }
    }

    /// Get the current value of the signal.
    fn get(&self) -> Option<T> {
        self.value.lock()
            .expect("Failed to lock mutex to acquire signal value")
            .clone()
    }
}

/// A discrete signal that can have its value set directly to introduce external input.
#[derive(Clone)]
pub struct Discrete<T> {
    base: SignalBase<T>,
}

impl<T> Discrete<T>
where
    T: Clone + PartialEq + Send + Sync + 'static
{
    /// Create a new discrete signal of the given type, with the initial value of None.
    pub fn new() -> Self {
        Self {
            base: SignalBase::new(),
        }
    }

    /// Push a new value for the signal.
    pub fn push(&mut self, value: T) {
        self.base.set(value);
    }

    /// Lift the discrete signal into a continous signal that holds the current value whenever it
    /// changes.
    pub fn hold(&mut self) -> Continuous<T> {
        Continuous::new1(&mut self.base, |a| a)
    }
}

impl<T> AsMut<Discrete<T>> for Discrete<T> {
    fn as_mut(&mut self) -> &mut Discrete<T> {
        self
    }
}

/// A continuous signal that can be composed from other discrete and continuous signals and
/// a closure.
#[derive(Clone)]
pub struct Continuous<T> {
    base: SignalBase<T>,
}

impl<T> Continuous<T>
where
    T: Clone + PartialEq + Send + Sync + 'static,
{
    /// The internal definition of lift1 ,which is used to provide hold for discrete signals, and
    /// map/lift1 for continuous signals. Produces a new signal from the given input signal and a
    /// closure.
    fn new1<A, F>(parent: &mut SignalBase<A>, update: F) -> Self
    where
        A: Clone + PartialEq + Send + Sync + 'static,
        F: Fn(A) -> T + Send + Sync + 'static,
    {
        let signal = Continuous {
            base: SignalBase::new(),
        };

        let mut signal_base = signal.base.clone();
        let parent_clone = parent.clone();
        parent.attach(move || {
            if let Some(value) = parent_clone.get() {
                signal_base.set(update(value));
            }
        });

        signal
    }

    /// The internal definition of lift2 ,which is used to provide lift2 for continuous signals.
    /// Produces a new signal from the given input signals and a closure.
    /// TODO: work out how to make these for any arity.
    fn new2<A, B, F>(parent_a: &mut SignalBase<A>, parent_b: &mut SignalBase<B>, update: F) -> Self
    where
        A: Clone + PartialEq + Send + Sync + 'static,
        B: Clone + PartialEq + Send + Sync + 'static,
        F: Fn(A, B) -> T + Clone + Send + Sync + 'static,
    {
        let signal = Continuous {
            base: SignalBase::new(),
        };

        let parent_a_base = parent_a.clone();
        let parent_b_base = parent_b.clone();
        let mut signal_base = signal.base.clone();

        let update_closure = move || {
            match (parent_a_base.get(), parent_b_base.get()) {
                (Some(a), Some(b)) => {
                    signal_base.set(update(a, b));                
                },
                _ => {}
            }
        };

        parent_a.attach(update_closure.clone());
        parent_b.attach(update_closure);

        signal
    }

    /// Sample the current value of the signal.
    pub fn sample(&self) -> Option<T> {
        self.base.get()
    }

    /// Apply a closure to the signal, producing a new signal.
    pub fn map<F, B>(&mut self, closure: F) -> Continuous<B>
    where
        B: Clone + PartialEq + Send + Sync + 'static,
        F: Fn(T) -> B + Send + Sync + 'static,
    {
        lift1(self, closure)
    }
}

impl<T> AsMut<Continuous<T>> for Continuous<T> {
    fn as_mut(&mut self) -> &mut Continuous<T> {
        self
    }
}

/// Apply a function to the given signal, producing a new signal.
pub fn lift1<F, A, B>(signal: &mut Continuous<A>, closure: F) -> Continuous<B>
where
    A: Clone + PartialEq + Send + Sync + 'static,
    B: Clone + PartialEq + Send + Sync + 'static,
    F: Fn(A) -> B + Send + Sync + 'static,
{
    Continuous::new1(&mut signal.base, closure)
}

/// Apply a function to the given signals, producing a new signal.
pub fn lift2<F, A, B, C>(signal_a: &mut Continuous<A>, signal_b: &mut Continuous<B>, closure: F) -> Continuous<C>
where
    A: Clone + PartialEq + Send + Sync + 'static,
    B: Clone + PartialEq + Send + Sync + 'static,
    C: Clone + PartialEq + Send + Sync + 'static,
    F: Fn(A, B) -> C + Clone + Send + Sync + 'static,
{
    Continuous::new2(&mut signal_a.base, &mut signal_b.base, closure)
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_unsubscribe() {
        let mut event = Discrete::<f64>::new();
        let signal = event.hold().map(|value| value * 2.0);

        // Initial value should be None.
        assert_eq!(signal.sample(), None);

        // It should be possible to update the value by pushing event occurences.
        event.push(0.0);
        //assert_eq!(signal.sample(), Some(0.0));
        //event.push(50.0);
        //assert_eq!(signal.sample(), Some(100.0));

        // Dropping the signal should cause it to unsubscribe automatically, and pushing new event
        // occurrences should be safe.
        //std::mem::drop(signal);
        //event.push(0.0);
    }
}
