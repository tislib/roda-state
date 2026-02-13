use crate::components::{Engine, Store, StoreOptions};
use crate::store::StoreJournal;
use bytemuck::Pod;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;

pub struct RodaEngine {
    root_path: &'static str,
    running: Arc<AtomicBool>,
}

impl Engine for RodaEngine {
    fn run_worker(&self, mut runnable: impl FnMut() + Send + 'static) {
        let running = self.running.clone();
        thread::spawn(move || {
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                runnable();
            }
        });
    }

    fn store<State: Pod + Send>(&self, options: StoreOptions) -> impl Store<State> + 'static {
        StoreJournal::new(self.root_path, options, size_of::<State>())
    }
}

impl RodaEngine {
    pub fn new() -> Self {
        Self {
            root_path: "data",
            running: Arc::new(AtomicBool::new(true)),
        }
    }

    pub fn new_with_root_path(root_path: &'static str) -> Self {
        Self {
            root_path,
            running: Arc::new(AtomicBool::new(true)),
        }
    }
}

impl Default for RodaEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl Drop for RodaEngine {
    fn drop(&mut self) {
        self.running
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }
}
