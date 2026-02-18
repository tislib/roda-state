use crate::stage::{OutputCollector, Stage};
use bytemuck::Pod;
use fxhash::FxHashMap;
use std::marker::PhantomData;

/// Compares the current item with the previous item associated with the same key.
///
/// This stage is useful for calculating changes or deltas between events in a stream.
pub struct Delta<K, T, Out, F, L> {
    key_fn: F,
    logic: L,
    last_values: FxHashMap<K, T>,
    _phantom: PhantomData<(T, Out)>,
}

impl<K, T, Out, F, L> Delta<K, T, Out, F, L>
where
    K: std::hash::Hash + Eq,
    T: Pod,
    Out: Pod,
    F: FnMut(&T) -> K,
    L: FnMut(&T, Option<T>) -> Option<Out>,
{
    pub fn new(key_fn: F, logic: L) -> Self {
        Self {
            key_fn,
            logic,
            last_values: FxHashMap::default(),
            _phantom: PhantomData,
        }
    }
}

impl<K, T, Out, F, L> Stage<T, Out> for Delta<K, T, Out, F, L>
where
    K: std::hash::Hash + Eq + Send,
    T: Pod + Send,
    Out: Pod + Send,
    F: FnMut(&T) -> K + Send,
    L: FnMut(&T, Option<T>) -> Option<Out> + Send,
{
    #[inline(always)]
    fn process<C>(&mut self, curr: &T, collector: &mut C)
    where
        C: OutputCollector<Out>,
    {
        let key = (self.key_fn)(curr);
        let prev = self.last_values.get(&key).copied();
        self.last_values.insert(key, *curr);
        if let Some(out) = (self.logic)(curr, prev) {
            collector.push(&out);
        }
    }
}

#[allow(clippy::type_complexity)]
pub fn delta<K, T, Out>(
    key_fn: impl FnMut(&T) -> K + Send,
    logic: impl FnMut(&T, Option<T>) -> Option<Out> + Send,
) -> Delta<K, T, Out, impl FnMut(&T) -> K + Send, impl FnMut(&T, Option<T>) -> Option<Out> + Send>
where
    K: std::hash::Hash + Eq,
    T: Pod,
    Out: Pod,
{
    Delta::new(key_fn, logic)
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug, PartialEq)]
struct Metric {
    pub id: u64,
    pub val: f64,
}

#[test]
fn test_delta_logic() {
    let mut pipe = delta(
        |m: &Metric| m.id,
        |curr, prev| match prev {
            Some(p) if curr.val >= p.val + 5.0 => Some(1u8),
            _ => Some(0u8),
        },
    );
    let mut out = Vec::new();

    let m1 = Metric { id: 1, val: 10.0 };
    let m2 = Metric { id: 1, val: 17.0 };

    pipe.process(&m1, &mut |x: &u8| out.push(*x));
    pipe.process(&m2, &mut |x: &u8| out.push(*x));

    assert_eq!(out, vec![0u8, 1u8]);
}
