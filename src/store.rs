use crate::components::{Store, StoreOptions, StoreReader};
use crate::index::DirectIndex;
use crate::storage::mmap_journal::MmapJournal;
use bytemuck::Pod;
use std::cell::Cell;
use std::path::PathBuf;

pub struct StoreJournal {
    storage: MmapJournal,
}

pub struct StoreJournalReader {
    next_index: Cell<usize>,
    storage: MmapJournal,
}

impl StoreJournal {
    pub fn new(root_path: &'static str, option: StoreOptions, state_size: usize) -> Self {
        let total_size = option.size * state_size;
        let storage = if option.in_memory {
            MmapJournal::new(None, total_size).unwrap()
        } else {
            let path: PathBuf = format!("{}/{}.store", root_path, option.name).into();
            if path.exists() {
                MmapJournal::load(path).unwrap()
            } else {
                MmapJournal::new(Some(path), total_size).unwrap()
            }
        };

        Self { storage }
    }
}

impl<State: Pod + Send> Store<State> for StoreJournal {
    type Reader = StoreJournalReader;

    fn push(&mut self, state: State) {
        let size = size_of::<State>();
        let current_pos = self.storage.get_write_index();
        assert!(
            current_pos + size <= self.storage.len(),
            "Store is full. Capacity: {}, Current position: {}, State size: {}",
            self.storage.len(),
            current_pos,
            size
        );
        self.storage.append(&state);
    }

    fn reader(&self) -> StoreJournalReader {
        StoreJournalReader {
            next_index: Cell::new(0),
            storage: self.storage.reader(),
        }
    }

    fn direct_index<Key: Pod + Ord + Send>(&self) -> DirectIndex<Key, State, StoreJournalReader> {
        DirectIndex {
            map: std::sync::Arc::new(crossbeam_skiplist::SkipMap::new()),
            reader: StoreJournalReader {
                next_index: Cell::new(0),
                storage: self.storage.reader(),
            },
        }
    }
}

impl<State: Pod + Send> StoreReader<State> for StoreJournalReader {
    fn next(&self) -> bool {
        let index_to_read = self.next_index.get();
        let offset = index_to_read * size_of::<State>();
        let write_index = self.storage.get_write_index();

        if offset + size_of::<State>() > write_index {
            return false;
        }

        self.next_index.set(index_to_read + 1);

        true
    }

    fn get_index(&self) -> usize {
        self.next_index.get()
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

    fn get_window<const N: usize>(&self, at: usize) -> Option<&[State]> {
        let offset = at * size_of::<State>();
        let write_index = self.storage.get_write_index();
        if offset + size_of::<State>() * N > write_index {
            return None;
        }

        Some(self.storage.read_window::<State, N>(offset))
    }
}
