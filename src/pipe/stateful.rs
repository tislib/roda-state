use crate::stage::{OutputCollector, Stage};
use bytemuck::Pod;
use std::collections::HashMap;
use std::marker::PhantomData;

/// Maintains per-key state for stateful aggregations or processing.
///
/// It uses a `HashMap` to store state for each key and applies a folding function
/// to update the state with each incoming item.
pub struct Stateful<K, In, Out, KF, IF, FF> {
    key_fn: KF,
    init_fn: IF,
    fold_fn: FF,
    storage: HashMap<K, Out>,
    _phantom: PhantomData<In>,
}

impl<K, In, Out, KF, IF, FF> Stateful<K, In, Out, KF, IF, FF>
where
    K: std::hash::Hash + Eq,
    In: Pod,
    Out: Pod,
    KF: FnMut(&In) -> K,
    IF: FnMut(&In) -> Out,
    FF: FnMut(&mut Out, &In),
{
    pub fn new(key_fn: KF, init_fn: IF, fold_fn: FF) -> Self {
        Self {
            key_fn,
            init_fn,
            fold_fn,
            storage: HashMap::new(),
            _phantom: PhantomData,
        }
    }
}

impl<K, In, Out, KF, IF, FF> Stage<In, Out> for Stateful<K, In, Out, KF, IF, FF>
where
    K: std::hash::Hash + Eq + Send,
    In: Pod + Send,
    Out: Pod + Send,
    KF: FnMut(&In) -> K + Send,
    IF: FnMut(&In) -> Out + Send,
    FF: FnMut(&mut Out, &In) + Send,
{
    #[inline(always)]
    fn process<C>(&mut self, item: &In, collector: &mut C)
    where
        C: OutputCollector<Out>,
    {
        let key = (self.key_fn)(item);
        let entry = self
            .storage
            .entry(key)
            .and_modify(|state| (self.fold_fn)(state, item))
            .or_insert_with(|| (self.init_fn)(item));
        collector.push(entry);
    }
}

#[allow(clippy::type_complexity)]
pub fn stateful<K, In, Out>(
    key_fn: impl FnMut(&In) -> K + Send,
    init_fn: impl FnMut(&In) -> Out + Send,
    fold_fn: impl FnMut(&mut Out, &In) + Send,
) -> Stateful<
    K,
    In,
    Out,
    impl FnMut(&In) -> K + Send,
    impl FnMut(&In) -> Out + Send,
    impl FnMut(&mut Out, &In) + Send,
>
where
    K: std::hash::Hash + Eq,
    In: Pod,
    Out: Pod,
{
    Stateful::new(key_fn, init_fn, fold_fn)
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Message {
    pub id: u64,
    pub value: i64,
}

#[cfg(test)]
mod stateful_tests {
    use super::*;

    #[test]
    fn test_stateful_logic() {
        let mut pipe = stateful(
            |item: &Message| item.id,
            |item| item.value,
            |state, item| *state += item.value,
        );
        let mut out = Vec::new();

        let m1 = Message { id: 1, value: 10 };
        let m2 = Message { id: 2, value: 5 };
        let m3 = Message { id: 1, value: 20 };

        pipe.process(&m1, &mut |x: &i64| out.push(*x));
        pipe.process(&m2, &mut |x: &i64| out.push(*x));
        pipe.process(&m3, &mut |x: &i64| out.push(*x));

        assert_eq!(out, vec![10, 5, 30]);
    }
}
