//! A simple frp-like signals implementation.

/// A trait for "signals" which have an input and an output type, and can be evaluated for the
/// given input.
pub trait Signal<IN, OUT>: Send {
    /// Evaluate this signal for the given input, yielding an output.
    fn evaluate(&mut self, input: IN) -> OUT;
}

/// A trait for "signals" which have an input and an output type, and can be evaluated for the
/// given input.
pub trait OldSignal<IN, OUT>: Send {
    /// Evaluate this signal for the given input, yielding an output.
    fn evaluate(&mut self, input: IN) -> OUT;
}
