use crate::components::{Engine, Store, StoreOptions};
use crate::measure::latency_measurer::LatencyMeasurer;
use crate::store::StoreJournal;
use bytemuck::Pod;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Instant;

pub struct RodaEngine {
    root_path: &'static str,
    running: Arc<AtomicBool>,
    enable_latency_stats: bool,
    worker_handlers: Vec<thread::JoinHandle<()>>,
}

impl Engine for RodaEngine {
    fn run_worker(&mut self, mut runnable: impl FnMut() + Send + 'static) {
        let worker_id = self.worker_handlers.len();
        let running = self.running.clone();
        let enable_latency_stats = self.enable_latency_stats;
        let handler = thread::spawn(move || {
            if enable_latency_stats {
                let mut measurer = LatencyMeasurer::new(1000);
                while running.load(std::sync::atomic::Ordering::Relaxed) {
                    let instant = Instant::now();
                    runnable();
                    measurer.measure(instant.elapsed());
                }
                println!("[Worker:{}]{}", worker_id, measurer.format_stats());
            } else {
                while running.load(std::sync::atomic::Ordering::Relaxed) {
                    runnable();
                }
            }
        });
        self.worker_handlers.push(handler);
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
            enable_latency_stats: false,
            worker_handlers: vec![],
        }
    }

    pub fn new_with_root_path(root_path: &'static str) -> Self {
        Self {
            root_path,
            running: Arc::new(AtomicBool::new(true)),
            enable_latency_stats: false,
            worker_handlers: vec![],
        }
    }

    pub fn enable_latency_stats(&mut self, enable: bool) {
        self.enable_latency_stats = enable;
    }
}

impl Default for RodaEngine {
    fn default() -> Self {
        Self::new()
    }
}
