use bytemuck::Pod;

/// For structures where we append data to the end (Journals, Logs).
pub trait Appendable<State: Pod> {
    fn append(&mut self, state: &State);
}

/// For structures where we update a specific "address" or "slot" (State Maps, Arrays).
pub trait Settable<State: Pod> {
    fn set(&mut self, at: usize, state: State);
}

/// The base for anything that can be read.
pub trait IterativeReadable<State: Pod> {
    fn next(&self) -> bool;
    fn get(&self) -> Option<State>;
    fn get_index(&self) -> usize;
}
