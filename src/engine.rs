use crate::journal_store::{JournalStore, JournalStoreOptions};
use crate::measure::latency_measurer::LatencyMeasurer;
use crate::op_counter::OpCounter;
use bytemuck::Pod;
use spdlog::info;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};

pub struct RodaEngine {
    root_path: &'static str,
    running: Arc<AtomicBool>,
    enable_latency_stats: bool,
    worker_handlers: Vec<thread::JoinHandle<()>>,
    op_counter: Arc<OpCounter>,
}

impl RodaEngine {
    pub fn new() -> Self {
        Self {
            root_path: "data",
            running: Arc::new(AtomicBool::new(true)),
            enable_latency_stats: false,
            worker_handlers: vec![],
            op_counter: OpCounter::new(),
        }
    }

    pub fn new_with_root_path(root_path: &'static str) -> Self {
        Self {
            root_path,
            running: Arc::new(AtomicBool::new(true)),
            enable_latency_stats: false,
            worker_handlers: vec![],
            op_counter: OpCounter::new(),
        }
    }

    pub fn enable_latency_stats(&mut self, enable: bool) {
        self.enable_latency_stats = enable;
    }

    pub fn run_worker(&mut self, mut runnable: impl FnMut() + Send + 'static) {
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
                info!("[Latency/Worker:{}]{}", worker_id, measurer.format_stats());
            } else {
                while running.load(std::sync::atomic::Ordering::Relaxed) {
                    runnable();
                }
            }
        });
        self.worker_handlers.push(handler);
    }

    pub fn new_journal_store<State: Pod + Send>(
        &self,
        options: JournalStoreOptions,
    ) -> JournalStore<State> {
        JournalStore::new(
            self.root_path,
            self.op_counter.clone(),
            options,
            size_of::<State>(),
        )
    }

    pub fn await_idle(&self, timeout: Duration) {
        let start = Instant::now();
        let mut last_op_count = self.op_counter.total_op_count();
        loop {
            sleep(Duration::from_millis(100));
            let new_op_count = self.op_counter.total_op_count();
            if new_op_count == last_op_count {
                break;
            }
            if start.elapsed() > timeout {
                break;
            }
            println!("[OPC]{}", new_op_count);
            last_op_count = new_op_count;
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
        for handler in self.worker_handlers.drain(..) {
            handler.join().unwrap();
        }
    }
}
