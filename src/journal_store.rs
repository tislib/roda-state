use crate::components::{Appendable, IterativeReadable};
use crate::op_counter::OpCounter;
use crate::storage::journal_mmap::JournalMmap;
use bytemuck::Pod;
use std::cell::Cell;
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering::Relaxed;

/// Configuration options for a `JournalStore`.
pub struct JournalStoreOptions {
    /// The name of the store, used for the filename.
    pub name: &'static str,
    /// The maximum number of items the store can hold.
    pub size: usize,
    /// Whether to keep the store only in memory.
    pub in_memory: bool,
}

// Work in Progress, not used currently.
pub struct JournalStore<State: Pod + Send> {
    storage: JournalMmap,
    op_counter: Arc<OpCounter>,
    _marker: std::marker::PhantomData<State>,
}

/// A reader for a `JournalStore` that maintains its own read index.
pub struct StoreJournalReader<State: Pod + Send> {
    next_index: Cell<usize>,
    storage: JournalMmap,
    op_count: Arc<AtomicU64>,
    _marker: std::marker::PhantomData<State>,
}

impl<State: Pod + Send> JournalStore<State> {
    pub fn new(
        root_path: &'static str,
        op_counter: Arc<OpCounter>,
        option: JournalStoreOptions,
    ) -> Self {
        let total_size = option.size * size_of::<State>();
        let storage = if option.in_memory {
            JournalMmap::new(None, total_size).unwrap()
        } else {
            let path: PathBuf = format!("{}/{}.store", root_path, option.name).into();
            if path.exists() {
                JournalMmap::load(path).unwrap()
            } else {
                JournalMmap::new(Some(path), total_size).unwrap()
            }
        };

        Self {
            op_counter,
            storage,
            _marker: Default::default(),
        }
    }

    /// Appends an item to the store.
    pub fn append(&mut self, state: &State) {
        let size = size_of::<State>();
        let current_pos = self.storage.get_write_index();
        assert!(
            current_pos + size <= self.storage.len(),
            "Store is full. Capacity: {}, Current position: {}, State size: {}",
            self.storage.len(),
            current_pos,
            size
        );
        self.storage.append(state);
    }

    pub fn reader(&self) -> StoreJournalReader<State> {
        StoreJournalReader {
            op_count: self.op_counter.new_counter(),
            next_index: Cell::new(0),
            storage: self.storage.reader(),
            _marker: Default::default(),
        }
    }

    pub fn size(&self) -> usize {
        self.storage.get_write_index() / size_of::<State>()
    }
}

impl<State: Pod + Send> Appendable<State> for JournalStore<State> {
    fn append(&mut self, state: &State) {
        self.append(state);
    }
}

impl<State: Pod + Send> StoreJournalReader<State> {
    #[inline(always)]
    pub fn next(&self) -> bool {
        let index_to_read = self.next_index.get();
        let offset = index_to_read * size_of::<State>();
        let write_index = self.storage.get_write_index();

        if offset + size_of::<State>() > write_index {
            return false;
        }

        self.next_index.set(index_to_read + 1);
        self.op_count.fetch_add(1, Relaxed);

        true
    }

    #[inline(always)]
    pub fn get_index(&self) -> usize {
        self.next_index.get()
    }

    #[inline(always)]
    pub fn with<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R> {
        let next_index = self.next_index.get();
        if next_index == 0 {
            return None;
        }
        let current_index = next_index - 1;
        let offset = current_index * size_of::<State>();
        Some(handler(self.storage.read(offset)))
    }

    /// Processes all remaining items in the store using the provided handler.
    ///
    /// This is highly optimized using batch reading (read_window).
    #[inline(always)]
    pub fn handle_remaining(&self, mut handler: impl FnMut(&State)) -> usize {
        let index_to_read = self.next_index.get();
        let offset = index_to_read * size_of::<State>();
        let write_index = self.storage.get_write_index();

        // If there is no new data, exit immediately (Hot path)
        if offset + size_of::<State>() > write_index {
            return 0;
        }

        let processed_items = (write_index - offset) / size_of::<State>();

        let window = self.storage.read_window::<State>(offset, processed_items);

        for item in window {
            handler(item);
        }

        // 3. Commit state exactly once at the end
        self.next_index.set(index_to_read + processed_items);
        self.op_count.fetch_add(processed_items as u64, Relaxed);

        processed_items
    }

    #[inline(always)]
    pub fn with_at<R>(&self, at: usize, handler: impl FnOnce(&State) -> R) -> Option<R> {
        let offset = at * size_of::<State>();
        let write_index = self.storage.get_write_index();
        if offset + size_of::<State>() > write_index {
            return None;
        }
        Some(handler(self.storage.read(offset)))
    }

    #[inline(always)]
    pub fn with_last<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R> {
        let write_index = self.storage.get_write_index();
        if write_index < size_of::<State>() {
            return None;
        }
        let offset = write_index - size_of::<State>();
        Some(handler(self.storage.read(offset)))
    }

    #[inline(always)]
    pub fn get(&self) -> Option<State> {
        self.with(|s| *s)
    }

    #[inline(always)]
    pub fn get_at(&self, at: usize) -> Option<State> {
        self.with_at(at, |s| *s)
    }

    #[inline(always)]
    pub fn get_last(&self) -> Option<State> {
        self.with_last(|s| *s)
    }

    #[inline(always)]
    pub fn get_window<const N: usize>(&self, at: usize) -> Option<&[State]> {
        let offset = at * size_of::<State>();
        let write_index = self.storage.get_write_index();
        if offset + size_of::<State>() * N > write_index {
            return None;
        }

        Some(self.storage.read_window_const::<State, N>(offset))
    }

    #[inline(always)]
    pub fn size(&self) -> usize {
        self.storage.get_write_index() / size_of::<State>()
    }
}

impl<State: Pod + Send> IterativeReadable<State> for StoreJournalReader<State> {
    fn next(&self) -> bool {
        self.next()
    }

    fn get(&self) -> Option<State> {
        self.get()
    }

    fn get_index(&self) -> usize {
        self.get_index()
    }
}
