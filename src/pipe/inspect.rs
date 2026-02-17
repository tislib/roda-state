/// Passes the item through while performing a side effect.
use crate::stage::{OutputCollector, Stage};
use bytemuck::Pod;
use std::marker::PhantomData;

/// Passes the item through while performing a side effect.
pub struct Inspect<T, F> {
    f: F,
    _phantom: PhantomData<T>,
}

impl<T: Pod + Send, F: FnMut(&T)> Inspect<T, F> {
    pub fn new(f: F) -> Self {
        Self {
            f,
            _phantom: PhantomData,
        }
    }
}

impl<T: Pod + Send, F: FnMut(&T)> Stage<T, T> for Inspect<T, F> {
    #[inline(always)]
    fn process<C>(&mut self, data: &T, collector: &mut C)
    where
        C: OutputCollector<T>,
    {
        (self.f)(data);
        collector.push(data);
    }
}

pub fn inspect<T: Pod + Send>(f: impl FnMut(&T)) -> Inspect<T, impl FnMut(&T)> {
    Inspect::new(f)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_inspect_logic() {
        let count = Arc::new(AtomicUsize::new(0));
        let count_inner = count.clone();
        let mut pipe = inspect(move |_x: &u32| {
            count_inner.fetch_add(1, Ordering::Relaxed);
        });

        let mut out = Vec::new();
        pipe.process(&42u32, &mut |x: &u32| out.push(*x));

        assert_eq!(out, vec![42]);
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }
}
