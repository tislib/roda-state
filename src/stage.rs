use bytemuck::Pod;
use std::marker::PhantomData;

pub trait Stage<In: Pod + Send, Out: Pod + Send> {
    fn process<C>(&mut self, data: In, collector: &mut C)
    where
        C: OutputCollector<Out>;
}

pub trait OutputCollector<T> {
    fn push(&mut self, item: T);
}

impl<T, F> OutputCollector<T> for F
where
    F: FnMut(T),
{
    #[inline(always)]
    fn push(&mut self, item: T) {
        (self)(item);
    }
}

impl<F, In, Out> Stage<In, Out> for F
where
    F: FnMut(In) -> Option<Out>,
    In: Pod + Send,
    Out: Pod + Send,
{
    #[inline(always)]
    fn process<C>(&mut self, data: In, collector: &mut C)
    where
        C: OutputCollector<Out>,
    {
        // Execute the closure and pass the result downstream
        let out = (self)(data);
        if let Some(out) = out {
            collector.push(out);
        }
    }
}

pub struct Pipeline<S1, S2, In, Mid, Out> {
    s1: S1,
    s2: S2,
    _phantom: PhantomData<(In, Mid, Out)>,
}

impl<In, Mid, Out, S1, S2> Stage<In, Out> for Pipeline<S1, S2, In, Mid, Out>
where
    In: Pod + Send,
    Mid: Pod + Send,
    Out: Pod + Send,
    S1: Stage<In, Mid>,
    S2: Stage<Mid, Out>,
{
    #[inline(always)]
    fn process<C>(&mut self, data: In, collector: &mut C)
    where
        C: OutputCollector<Out>,
    {
        self.s1.process(data, &mut |mid| {
            self.s2.process(mid, collector);
        });
    }
}

pub trait StageExt<In: Pod + Send, Mid: Pod + Send>: Stage<In, Mid> {
    #[inline(always)]
    fn pipe<Out: Pod + Send, S2: Stage<Mid, Out>>(self, s2: S2) -> Pipeline<Self, S2, In, Mid, Out>
    where
        Self: Sized,
    {
        Pipeline {
            s1: self,
            s2,
            _phantom: PhantomData,
        }
    }
}

impl<S, In, Mid> StageExt<In, Mid> for S
where
    In: Pod + Send,
    Mid: Pod + Send,
    S: Stage<In, Mid>,
{
}

#[macro_export]
macro_rules! pipe {
    ($s1:expr) => { $s1 };
    ($s1:expr, $($rest:expr),+ $(,)?) => {
        {
            use $crate::stage::StageExt;
            $s1.pipe($crate::pipe!($($rest),+))
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipe_closures() {
        let mut p = pipe![|x: u32| Some(x as u64), |x: u64| Some(x as u8),];

        let mut out = Vec::new();
        p.process(100u32, &mut |x: u8| out.push(x));
        assert_eq!(out, vec![100u8]);
    }

    #[test]
    fn test_pipe_one_to_many() {
        struct Duplicate;
        impl Stage<u64, u64> for Duplicate {
            fn process<C>(&mut self, data: u64, collector: &mut C)
            where
                C: OutputCollector<u64>,
            {
                collector.push(data);
                collector.push(data);
            }
        }

        let mut p = pipe![|x: u32| Some(x as u64), Duplicate, |x: u64| Some(x as u8),];

        let mut out = Vec::new();
        p.process(10u32, &mut |x: u8| out.push(x));
        assert_eq!(out, vec![10u8, 10u8]);
    }
}
