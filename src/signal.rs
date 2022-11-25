pub trait Signal<IN, OUT>: Send {
    fn evaluate(&mut self, input: IN) -> OUT;
}
