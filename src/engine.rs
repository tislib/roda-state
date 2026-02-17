use crate::journal_store::{JournalStore, JournalStoreOptions};
use crate::op_counter::OpCounter;
use crate::slot_store::{SlotStore, SlotStoreOptions};
use bytemuck::Pod;
use std::hint::spin_loop;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::thread::sleep;
use std::time::{Duration, Instant};

/// The core execution engine for Roda.
///
/// It manages worker threads, storage lifecycle, and shared operation counters.
pub struct RodaEngine {
    root_path: &'static str,
    running: Arc<AtomicBool>,
    worker_handlers: Vec<thread::JoinHandle<()>>,
    op_counter: Arc<OpCounter>,
    pin_cores: bool,
}

impl RodaEngine {
    /// Creates a new `RodaEngine` with the default "data" root path.
    pub fn new() -> Self {
        Self {
            root_path: "data",
            running: Arc::new(AtomicBool::new(true)),
            worker_handlers: vec![],
            op_counter: OpCounter::new(),
            pin_cores: false,
        }
    }

    pub(crate) fn set_pin_cores(&mut self, pin_cores: bool) {
        self.pin_cores = pin_cores;
    }

    /// Creates a new `RodaEngine` with a custom root path for storage.
    pub fn new_with_root_path(root_path: &'static str) -> Self {
        Self {
            root_path,
            running: Arc::new(AtomicBool::new(true)),
            worker_handlers: vec![],
            op_counter: OpCounter::new(),
            pin_cores: false,
        }
    }

    /// Spawns a worker thread that executes the provided runnable in a loop.
    ///
    /// The worker will spin and yield if there is no work to do, minimizing latency.
    pub fn run_worker(&mut self, mut runnable: impl FnMut() -> bool + Send + 'static) {
        let worker_id = self.worker_handlers.len();
        let running = self.running.clone();
        let pin_cores = self.pin_cores;
        let handler = thread::spawn(move || {
            if pin_cores
                && let Some(core_ids) = core_affinity::get_core_ids()
                && let Some(core_id) = core_ids.get(worker_id % core_ids.len())
            {
                core_affinity::set_for_current(*core_id);
            }

            let mut step_without_work_count = 0;
            while running.load(std::sync::atomic::Ordering::Relaxed) {
                let did_work = runnable();
                if did_work {
                    step_without_work_count = 0;
                } else {
                    step_without_work_count += 1;
                }
                if step_without_work_count > 1000 {
                    thread::yield_now();
                } else if step_without_work_count > 10 {
                    spin_loop();
                }
            }
        });
        self.worker_handlers.push(handler);
    }

    /// Creates a new `JournalStore` for sequential, append-only data storage.
    pub fn new_journal_store<State: Pod + Send>(
        &self,
        options: JournalStoreOptions,
    ) -> JournalStore<State> {
        JournalStore::new(self.root_path, self.op_counter.clone(), options)
    }

    /// Creates a new `SlotStore` for random-access, slot-based data storage.
    pub fn new_slot_store<State: Pod + Send>(&self, options: SlotStoreOptions) -> SlotStore<State> {
        SlotStore::new(self.root_path, self.op_counter.clone(), options)
    }

    /// Blocks until the engine is idle (i.e., no operations have occurred for a short period).
    pub fn await_idle(&self, timeout: Duration) {
        let start = Instant::now();
        let mut last_op_count = self.op_counter.total_op_count();
        loop {
            sleep(Duration::from_millis(1));
            let new_op_count = self.op_counter.total_op_count();
            if new_op_count == last_op_count {
                break;
            }
            if start.elapsed() > timeout {
                break;
            }
            last_op_count = new_op_count;
        }
    }

    pub fn is_any_worker_panicked(&self) -> bool {
        for handler in &self.worker_handlers {
            if handler.is_finished() && self.running.load(std::sync::atomic::Ordering::Relaxed) {
                return true;
            }
        }
        false
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
