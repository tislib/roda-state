use crate::components::Settable;
use crate::op_counter::OpCounter;
use crate::storage::slot_mmap::SlotMmap;
// Using the new SlotMmap logic
use bytemuck::Pod;
use std::path::PathBuf;
use std::sync::atomic::AtomicU64;
use std::sync::Arc;

pub struct SlotStore<State: Pod + Send> {
    storage: SlotMmap<State>,
    pub op_counter: Arc<OpCounter>,
    num_slots: usize,
}

pub struct SlotStoreReader<State: Pod + Send> {
    storage: SlotMmap<State>,
    op_count: Arc<AtomicU64>,
}

pub struct SlotStoreOptions {
    pub name: &'static str,
    pub size: usize,
    pub in_memory: bool,
}

impl<State: Pod + Send> SlotStore<State> {
    pub fn new(
        root_path: &'static str,
        op_counter: Arc<OpCounter>,
        option: SlotStoreOptions,
    ) -> Self {
        let storage = if option.in_memory {
            SlotMmap::new(None, option.size).unwrap()
        } else {
            let path: PathBuf = format!("{}/{}.store", root_path, option.name).into();
            if path.exists() {
                SlotMmap::load(path).unwrap()
            } else {
                SlotMmap::new(Some(path), option.size).unwrap()
            }
        };

        Self {
            num_slots: option.size,
            op_counter,
            storage,
        }
    }

    /// Unique to SlotStore: Write to a specific slot instead of appending
    pub fn update_at(&mut self, index: usize, state: State) {
        self.storage.write(index, &state);
    }

    pub fn reader(&self) -> SlotStoreReader<State> {
        SlotStoreReader {
            op_count: self.op_counter.new_counter(),
            storage: self.storage.reader(),
        }
    }

    pub fn size(&self) -> usize {
        self.num_slots
    }
}

impl<State: Pod + Send> Settable<State> for SlotStore<State> {
    fn set(&mut self, at: usize, state: State) {
        self.update_at(at, state);
    }
}

impl<State: Pod + Send> SlotStoreReader<State> {
    /// Performs a consistent snapshot read with retry logic
    pub fn with_at<R>(&self, at: usize, handler: impl FnOnce(&State) -> R) -> Option<R> {
        // Using 100 retries to ensure we get a consistent L5 snapshot
        self.storage
            .read_snapshot_with_retry(at, 100)
            .map(|state| handler(&state))
    }

    pub fn get_at(&self, at: usize) -> Option<State> {
        self.with_at(at, |s| *s)
    }

    pub fn size(&self) -> usize {
        self.storage.num_slots()
    }
}
