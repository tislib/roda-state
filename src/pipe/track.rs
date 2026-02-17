use crate::stage::{OutputCollector, Stage};
use bytemuck::{Pod, Zeroable};
use std::collections::HashMap;
use std::marker::PhantomData;

/// A struct that holds the current and previous values of a stream.
/// This is used to satisfy the `Pod` constraint while providing tuple-like behavior.
#[repr(C)]
#[derive(Debug, Clone, Copy, Default)]
pub struct Tracked<T: Pod + Zeroable> {
    pub prev: T,
    pub curr: T,
    pub has_prev: u8,
}

unsafe impl<T: Pod + Zeroable> Zeroable for Tracked<T> {}
unsafe impl<T: Pod + Zeroable> Pod for Tracked<T> {}

impl<T: Pod + Zeroable> Tracked<T> {
    /// Returns the previous value as an Option.
    pub fn prev(&self) -> Option<T> {
        if self.has_prev != 0 {
            Some(self.prev)
        } else {
            None
        }
    }
}

pub struct TrackPrevByHashmap<K, T, F> {
    key_fn: F,
    storage: HashMap<K, T>,
    _phantom: PhantomData<T>,
}

impl<K, T, F> TrackPrevByHashmap<K, T, F>
where
    K: std::hash::Hash + Eq,
    T: Pod + Zeroable + Copy,
    F: FnMut(&T) -> K,
{
    pub fn new(key_fn: F) -> Self {
        Self {
            key_fn,
            storage: HashMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<K, T, F> Stage<T, Tracked<T>> for TrackPrevByHashmap<K, T, F>
where
    K: std::hash::Hash + Eq,
    T: Pod + Zeroable + Copy + Send,
    F: FnMut(&T) -> K + Send,
{
    #[inline(always)]
    fn process<C>(&mut self, item: &T, collector: &mut C)
    where
        C: OutputCollector<Tracked<T>>,
    {
        let key = (self.key_fn)(item);
        let prev = self.storage.get(&key).copied();
        self.storage.insert(key, *item);

        collector.push(&Tracked {
            prev: prev.unwrap_or(T::zeroed()),
            curr: *item,
            has_prev: if prev.is_some() { 1 } else { 0 },
        });
    }
}

pub fn track_prev_by_hashmap<K, T>(
    key_fn: impl FnMut(&T) -> K + Send,
) -> TrackPrevByHashmap<K, T, impl FnMut(&T) -> K + Send>
where
    K: std::hash::Hash + Eq,
    T: Pod + Zeroable + Copy + Send,
{
    TrackPrevByHashmap::new(key_fn)
}

pub struct TrackPrev<T> {
    last_value: Option<T>,
}

impl<T: Pod + Zeroable + Copy + Send> Stage<T, Tracked<T>> for TrackPrev<T> {
    #[inline(always)]
    fn process<C>(&mut self, curr: &T, collector: &mut C)
    where
        C: OutputCollector<Tracked<T>>,
    {
        let prev = self.last_value.replace(*curr);
        collector.push(&Tracked {
            prev: prev.unwrap_or(T::zeroed()),
            curr: *curr,
            has_prev: if prev.is_some() { 1 } else { 0 },
        });
    }
}

pub fn track_prev<T: Pod + Zeroable + Copy + Send>() -> TrackPrev<T> {
    TrackPrev { last_value: None }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_track_prev_by_hashmap() {
        let mut pipe = track_prev_by_hashmap(|val: &i32| *val % 2);
        let mut out = Vec::new();

        // Key 0 (even): 2
        pipe.process(&2, &mut |res: &Tracked<i32>| out.push(*res));
        assert_eq!(out.last().unwrap().prev(), None);
        assert_eq!(out.last().unwrap().curr, 2);

        // Key 1 (odd): 3
        pipe.process(&3, &mut |res: &Tracked<i32>| out.push(*res));
        assert_eq!(out.last().unwrap().prev(), None);
        assert_eq!(out.last().unwrap().curr, 3);

        // Key 0 (even): 4, prev was 2
        pipe.process(&4, &mut |res: &Tracked<i32>| out.push(*res));
        assert_eq!(out.last().unwrap().prev(), Some(2));
        assert_eq!(out.last().unwrap().curr, 4);
    }

    #[test]
    fn test_track_prev() {
        let mut pipe = track_prev::<i32>();
        let mut out = Vec::new();

        pipe.process(&10, &mut |res: &Tracked<i32>| out.push(*res));
        assert_eq!(out.last().unwrap().prev(), None);
        assert_eq!(out.last().unwrap().curr, 10);

        pipe.process(&20, &mut |res: &Tracked<i32>| out.push(*res));
        assert_eq!(out.last().unwrap().prev(), Some(10));
        assert_eq!(out.last().unwrap().curr, 20);
    }
}
