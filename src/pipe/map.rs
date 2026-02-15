/// Transforms an item from one type to another.
use crate::stage::{OutputCollector, Stage};
use bytemuck::Pod;
use std::marker::PhantomData;

/// Transforms an item from one type to another.
pub struct Map<In, Out, F> {
    f: F,
    _phantom: PhantomData<(In, Out)>,
}

impl<In: Pod + Send, Out: Pod + Send, F: FnMut(&In) -> Out> Map<In, Out, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: PhantomData,
        }
    }
}

impl<In: Pod + Send, Out: Pod + Send, F: FnMut(&In) -> Out> Stage<In, Out> for Map<In, Out, F> {
    #[inline(always)]
    fn process<C>(&mut self, data: &In, collector: &mut C)
    where
        C: OutputCollector<Out>,
    {
        let out = (self.f)(data);
        collector.push(&out);
    }
}

pub fn map<In, Out>(f: impl FnMut(&In) -> Out) -> Map<In, Out, impl FnMut(&In) -> Out>
where
    In: Pod + Send,
    Out: Pod + Send,
{
    Map::new(f)
}

#[cfg(test)]
mod map_tests {
    use super::*;

    #[test]
    fn test_map_logic() {
        // Transform u32 to u64
        let mut pipe = map(|x: &u32| *x as u64 * 2);
        let mut out = Vec::new();

        pipe.process(&21u32, &mut |x: &u64| out.push(*x));

        assert_eq!(out, vec![42u64]);
    }
}
