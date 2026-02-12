use crate::components::{Store, StoreOptions, StoreReader};
use crate::index::DirectIndex;
use crate::storage::mmap_journal::MmapRing;
use bytemuck::Pod;
use std::cell::Cell;
use std::path::PathBuf;

pub struct CircularStore {
    storage: MmapRing,
}

pub struct CircularStoreReader {
    next_index: Cell<usize>,
    storage: MmapRing,
}

impl CircularStore {
    pub fn new(root_path: &'static str, option: StoreOptions) -> Self {
        let storage = if option.in_memory {
            MmapRing::new(None, option.size).unwrap()
        } else {
            let path: PathBuf = format!("{}/{}.store", root_path, option.name).into();
            if path.exists() {
                MmapRing::load(path).unwrap()
            } else {
                MmapRing::new(Some(path), option.size).unwrap()
            }
        };

        Self { storage }
    }
}

impl<State: Pod + Send> Store<State> for CircularStore {
    type Reader = CircularStoreReader;

    fn push(&mut self, state: State) {
        assert!(self.storage.len() >= size_of::<State>(), "Store size {} is too small for State size {}", self.storage.len(), size_of::<State>());
        self.storage.append(&state);
    }

    fn reader(&self) -> CircularStoreReader {
        CircularStoreReader {
            next_index: Cell::new(0),
            storage: self.storage.reader(),
        }
    }

    fn direct_index<Key: Pod>(&self) -> DirectIndex<Key, State> {
        DirectIndex {
            _k: std::marker::PhantomData,
            _v: std::marker::PhantomData,
        }
    }
}

impl<State: Pod + Send> StoreReader<State> for CircularStoreReader {
    fn next(&self) -> bool {
        let index_to_read = self.next_index.get();
        let offset = index_to_read * size_of::<State>();
        let write_index = self.storage.get_write_index();
        
        if offset + size_of::<State>() > write_index {
            return false;
        }

        let min_offset = write_index.saturating_sub(self.storage.len());
        if offset < min_offset {
            // Lapped: skip to the oldest available data
            let new_index = min_offset / size_of::<State>();
            self.next_index.set(new_index + 1);
        } else {
            self.next_index.set(index_to_read + 1);
        }

        true
    }

    fn with<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R> {
        let next_index = self.next_index.get();
        if next_index == 0 {
            return None;
        }
        let current_index = next_index - 1;
        let offset = current_index * size_of::<State>();
        Some(handler(self.storage.read(offset)))
    }

    fn with_at<R>(&self, at: usize, handler: impl FnOnce(&State) -> R) -> Option<R> {
        let offset = at * size_of::<State>();
        let write_index = self.storage.get_write_index();
        if offset + size_of::<State>() > write_index {
            return None;
        }
        if offset < write_index.saturating_sub(self.storage.len()) {
            return None; // Data has been overwritten
        }
        Some(handler(self.storage.read(offset)))
    }

    fn with_last<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R> {
        let write_index = self.storage.get_write_index();
        if write_index < size_of::<State>() {
            return None;
        }
        let offset = write_index - size_of::<State>();
        Some(handler(self.storage.read(offset)))
    }

    fn get(&self) -> Option<State> {
        self.with(|s| *s)
    }

    fn get_at(&self, at: usize) -> Option<State> {
        self.with_at(at, |s| *s)
    }

    fn get_last(&self) -> Option<State> {
        self.with_last(|s| *s)
    }

    fn get_window<const N: usize>(&self, at: usize) -> Option<[State; N]> {
        let offset = at * size_of::<State>();
        let write_index = self.storage.get_write_index();
        if offset + size_of::<State>() * N > write_index {
            return None;
        }
        if offset < write_index.saturating_sub(self.storage.len()) {
            return None; // Part of the window has been overwritten
        }

        Some(std::array::from_fn(|i| {
            *self.storage.read::<State>(offset + i * size_of::<State>())
        }))
    }
}
