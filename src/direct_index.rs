use crate::components::IterativeReadable;
use bytemuck::Pod;
use crossbeam_skiplist::SkipMap;
use std::ops::Bound;
use std::sync::Arc;

pub struct DirectIndex<
    Key: Clone + Ord + Send,
    State: Pod + Send,
    StoreReader: IterativeReadable<State> + 'static,
> {
    pub(crate) map: Arc<SkipMap<Key, State>>,
    pub reader: StoreReader,
}

pub struct DirectIndexReader<Key: Clone + Ord + Send, State: Pod + Send> {
    pub(crate) map: Arc<SkipMap<Key, State>>,
}

impl<Key, State, StoreReader> DirectIndex<Key, State, StoreReader>
where
    Key: Clone + Ord + Send + 'static,
    State: Pod + Send,
    StoreReader: IterativeReadable<State> + 'static,
{
    pub fn compute(&self, key_fn: impl FnOnce(&State) -> Key) {
        if self.reader.next()
            && let Some(state) = self.reader.get()
        {
            let key = key_fn(&state);
            self.map.insert(key.clone(), state);
        }
    }
    pub fn delete(&self, key: &Key) {
        self.map.remove(key);
    }

    pub fn reader(&self) -> DirectIndexReader<Key, State> {
        DirectIndexReader {
            map: self.map.clone(),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, State)> + '_ {
        self.map
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
    }

    pub fn size(&self) -> usize {
        self.map.len()
    }
}

impl<Key, State> DirectIndexReader<Key, State>
where
    Key: Clone + Ord + Send + 'static, // 'static or appropriate lifetime for the Map
    State: Pod + Send,
{
    pub fn with<R>(&self, key: &Key, handler: impl FnOnce(&State) -> R) -> Option<R> {
        self.map.get(key).map(|entry| handler(entry.value()))
    }

    pub fn get(&self, key: &Key) -> Option<State> {
        self.map.get(key).map(|entry| *entry.value())
    }

    pub fn iter(&self) -> impl Iterator<Item = (Key, State)> + '_ {
        self.map
            .iter()
            .map(|entry| (entry.key().clone(), *entry.value()))
    }

    // --- New Navigation Implementations ---

    /// Replicates lower_bound: starts at the first key >= provided key.
    pub fn find_ge<'a>(
        &'a self,
        key: &'a Key,
    ) -> impl DoubleEndedIterator<Item = (Key, State)> + 'a {
        self.map
            .range((Bound::Included(key), Bound::Unbounded))
            .map(move |entry| (entry.key().clone(), *entry.value()))
    }

    /// Replicates upper_bound: starts at the first key <= provided key,
    /// but usually used with .rev() to get the Best Bid.
    pub fn find_le<'a>(
        &'a self,
        key: &'a Key,
    ) -> impl DoubleEndedIterator<Item = (Key, State)> + 'a {
        self.map
            .range((Bound::Unbounded, Bound::Included(key)))
            .map(move |entry| (entry.key().clone(), *entry.value()))
    }

    /// Standard range scan (e.g., for getting a specific slice of the book).
    pub fn range<'a, R>(&'a self, range: R) -> impl DoubleEndedIterator<Item = (Key, State)> + 'a
    where
        R: std::ops::RangeBounds<Key> + 'a,
    {
        self.map
            .range(range)
            .map(move |entry| (entry.key().clone(), *entry.value()))
    }

    /// Efficiency helper to jump straight to the Best Bid or Best Ask.
    pub fn first_after(&self, key: &Key) -> Option<(Key, State)> {
        self.map
            .lower_bound(Bound::Included(key))
            .map(|e| (e.key().clone(), *e.value()))
    }

    pub fn last_before(&self, key: &Key) -> Option<(Key, State)> {
        // upper_bound finds first > key, then prev() finds highest <= key.
        let entry = self.map.upper_bound(Bound::Included(key))?;
        entry.prev().map(|e| (e.key().clone(), *e.value()))
    }

    pub fn size(&self) -> usize {
        self.map.len()
    }
}
