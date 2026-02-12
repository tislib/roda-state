use crate::components::{Engine, Store, StoreOptions};
use crate::store::CircularStore;
use bytemuck::Pod;
use std::thread;

pub struct RodaEngine {
    root_path: &'static str,
}

impl Engine for RodaEngine {
    fn run_worker(&self, mut runnable: impl FnMut() + Send + 'static) {
        thread::spawn(move || {
            loop {
                runnable();
            }
        });
    }

    fn store<State: Pod + Send>(&self, options: StoreOptions) -> impl Store<State> + 'static {
        CircularStore::new(self.root_path, options)
    }
}

impl RodaEngine {
    pub fn new() -> Self {
        Self { root_path: "data" }
    }

    pub fn new_with_root_path(root_path: &'static str) -> Self {
        Self { root_path }
    }
}

impl Default for RodaEngine {
    fn default() -> Self {
        Self::new()
    }
}
