use crate::stage::{OutputCollector, Stage};
use bytemuck::Pod;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Only emits the event if the value associated with the key has changed.
pub struct DedupBy<K, T, F> {
    key_fn: F,
    last_values: HashMap<K, T>,
    _phantom: PhantomData<T>,
}

impl<K, T, F> DedupBy<K, T, F>
where
    K: std::hash::Hash + Eq,
    T: Pod + PartialEq,
    F: FnMut(&T) -> K,
{
    pub fn new(key_fn: F) -> Self {
        Self {
            key_fn,
            last_values: HashMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<K, T, F> Stage<T, T> for DedupBy<K, T, F>
where
    K: std::hash::Hash + Eq + Send,
    T: Pod + PartialEq + Send,
    F: FnMut(&T) -> K + Send,
{
    #[inline(always)]
    fn process<C>(&mut self, curr: &T, collector: &mut C)
    where
        C: OutputCollector<T>,
    {
        let key = (self.key_fn)(curr);
        let prev = self.last_values.get(&key);

        if prev == Some(curr) {
            return;
        }

        self.last_values.insert(key, *curr);
        collector.push(curr);
    }
}

pub fn dedup_by<K, T>(
    key_fn: impl FnMut(&T) -> K + Send,
) -> DedupBy<K, T, impl FnMut(&T) -> K + Send>
where
    K: std::hash::Hash + Eq,
    T: Pod + PartialEq,
{
    DedupBy::new(key_fn)
}

#[cfg(test)]
mod dedup_tests {
    use super::*;

    #[test]
    fn test_dedup_logic() {
        let mut pipe = dedup_by(|_: &i32| 0);
        let mut out = Vec::new();

        pipe.process(&10, &mut |x: &i32| out.push(*x));
        pipe.process(&10, &mut |x: &i32| out.push(*x));
        pipe.process(&20, &mut |x: &i32| out.push(*x));
        pipe.process(&10, &mut |x: &i32| out.push(*x));

        assert_eq!(out, vec![10, 20, 10]);
    }
}
