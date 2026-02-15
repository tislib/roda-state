use std::sync::atomic::AtomicU64;
use std::sync::{Arc, Mutex};

pub struct OpCounter {
    counters: Mutex<Vec<Arc<AtomicU64>>>,
}

impl OpCounter {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            counters: Mutex::new(vec![]),
        })
    }

    pub fn total_op_count(&self) -> u64 {
        self.counters
            .lock()
            .unwrap()
            .iter()
            .map(|c| c.load(std::sync::atomic::Ordering::Relaxed))
            .sum()
    }

    pub fn new_counter(&self) -> Arc<AtomicU64> {
        let counter = Arc::new(AtomicU64::new(0));

        self.counters.lock().unwrap().push(counter.clone());

        counter
    }
}
