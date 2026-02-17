use crate::stage::{OutputCollector, Stage};
use bytemuck::Pod;
use std::marker::PhantomData;

/// Filters items based on a predicate.
///
/// Only items for which the predicate returns `true` are passed to the next stage.
pub struct Filter<T, F> {
    predicate: F,
    _phantom: PhantomData<T>,
}

impl<T: Pod + Send, F: FnMut(&T) -> bool> Filter<T, F> {
    pub fn new(predicate: F) -> Self {
        Self {
            predicate,
            _phantom: PhantomData,
        }
    }
}

impl<T: Pod + Send, F: FnMut(&T) -> bool> Stage<T, T> for Filter<T, F> {
    #[inline(always)]
    fn process<C>(&mut self, data: &T, collector: &mut C)
    where
        C: OutputCollector<T>,
    {
        if (self.predicate)(data) {
            collector.push(data);
        }
    }
}

pub fn filter<T: Pod + Send>(
    predicate: impl FnMut(&T) -> bool,
) -> Filter<T, impl FnMut(&T) -> bool> {
    Filter::new(predicate)
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn test_filter_logic() {
        let mut pipe = filter(|x: &i32| *x > 0);
        let mut out = Vec::new();

        pipe.process(&10, &mut |x: &i32| out.push(*x));
        pipe.process(&-5, &mut |x: &i32| out.push(*x));

        assert_eq!(out, vec![10]);
    }
}
