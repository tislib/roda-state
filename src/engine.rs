use crate::store::CircularRodaStore;
use bytemuck::Pod;
use std::thread;

pub struct RodaEngine {}

impl RodaEngine {
    pub fn run_worker(&self, mut runnable: impl FnMut() + Send + 'static) {
        thread::spawn(move || {
            loop {
                runnable();
            }
        });
    }
}

impl RodaEngine {
    pub fn store<State: Pod>(&self, _size: u32) -> CircularRodaStore<State> {
        todo!()
    }
}

impl RodaEngine {
    pub fn new() -> Self {
        Self {}
    }
}

impl Default for RodaEngine {
    fn default() -> Self {
        Self::new()
    }
}
