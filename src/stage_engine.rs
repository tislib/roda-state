use crate::components::Appendable;
use crate::stage::Stage;
use crate::{JournalStore, JournalStoreOptions, RodaEngine, StoreJournalReader};
use bytemuck::Pod;
use std::thread;
use std::time::Duration;

/// A threaded pipeline engine that grows by adding stages.
/// Each stage runs in its own thread and communicates via JournalStore.
pub struct StageEngine<In: Pod + Send + 'static, Out: Pod + Send + 'static> {
    engine: RodaEngine,
    input_store: JournalStore<In>,
    output_reader: StoreJournalReader<Out>,
    stage_count: usize,
    default_capacity: usize,
}

impl<In: Pod + Send + 'static, Out: Pod + Send + 'static> StageEngine<In, Out> {
    /// Adds a new stage to the pipeline.
    /// This method consumes the current engine and returns a new one with the updated output type.
    /// A new thread is spawned to run the provided stage.
    pub fn add_stage<NextOut: Pod + Send + 'static, S: Stage<Out, NextOut> + Send + 'static>(
        self,
        stage: S,
    ) -> StageEngine<In, NextOut> {
        let capacity = self.default_capacity;
        self.add_stage_with_capacity(capacity, stage)
    }

    /// Adds a new stage to the pipeline with a specific capacity for the output store.
    pub fn add_stage_with_capacity<
        's,
        NextOut: Pod + Send + 'static,
        S: Stage<Out, NextOut> + Send + 'static,
    >(
        mut self,
        capacity: usize,
        mut stage: S,
    ) -> StageEngine<In, NextOut> {
        let stage_idx = self.stage_count;
        self.stage_count += 1;

        // Use a leaked string for the store name as JournalStoreOptions requires &'static str.
        // In a production long-running system, we would use a more robust name management,
        // but for a pipeline that lasts the lifetime of the process, this is acceptable.
        let name = Box::leak(format!("stage_{}", stage_idx).into_boxed_str());

        let mut next_store = self
            .engine
            .new_journal_store::<NextOut>(JournalStoreOptions {
                name,
                size: capacity,
                in_memory: true,
            });

        let reader = self.output_reader;
        let next_reader = next_store.reader();

        self.engine.run_worker(move || {
            let mut did_work = false;
            while reader.next() {
                did_work = true;
                reader.with(|data| {
                    stage.process(data, &mut |out: &NextOut| next_store.append(out));
                });
            }
            if !did_work {
                thread::yield_now();
            }
        });

        StageEngine {
            engine: self.engine,
            input_store: self.input_store,
            output_reader: next_reader,
            stage_count: self.stage_count,
            default_capacity: self.default_capacity,
        }
    }

    /// Sends data into the start of the pipeline.
    /// Requires &mut self because JournalStore::append requires it (Single-Writer).
    pub fn send(&mut self, data: &In) {
        self.input_store.append(data);
    }

    /// Receives data from the end of the pipeline.
    /// This will block/poll until data is available.
    pub fn receive(&self) -> Option<Out> {
        loop {
            if let Some(data) = self.try_receive() {
                return Some(data);
            }
            if self.engine.is_any_worker_panicked() {
                panic!("Worker panicked, pipeline is broken");
            }
            thread::yield_now();
        }
    }

    /// Tries to receive data from the end of the pipeline without blocking.
    pub fn try_receive(&self) -> Option<Out> {
        if self.output_reader.next() {
            return self.output_reader.get();
        }
        None
    }

    /// Returns the number of items in the output store.
    pub fn output_size(&self) -> usize {
        self.output_reader.size()
    }

    pub fn enable_latency_stats(&mut self, enabled: bool) {
        self.engine.enable_latency_stats(enabled);
    }

    /// Waits for all workers to finish processing.
    pub fn await_idle(&self, timeout: Duration) {
        self.engine.await_idle(timeout);
    }
}

impl<In: Pod + Send + 'static, Out: Pod + Send + 'static> Appendable<In> for StageEngine<In, Out> {
    fn append(&mut self, state: &In) {
        self.send(state);
    }
}

impl<T: Pod + Send + 'static> Default for StageEngine<T, T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T: Pod + Send + 'static> StageEngine<T, T> {
    /// Creates a new engine with no stages.
    /// Acts as a passthrough until stages are added.
    pub fn new() -> Self {
        Self::with_capacity(1024)
    }

    /// Creates a new engine with a specific capacity for the input store.
    pub fn with_capacity(capacity: usize) -> Self {
        let engine = RodaEngine::new();
        let input_store = engine.new_journal_store(JournalStoreOptions {
            name: "input",
            size: capacity,
            in_memory: true,
        });
        let output_reader = input_store.reader();

        Self {
            engine,
            input_store,
            output_reader,
            stage_count: 0,
            default_capacity: capacity,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_new_engine_threaded_pipeline() {
        let mut engine = StageEngine::<u32, u32>::new()
            .add_stage(|x: &u32| Some(*x as u64))
            .add_stage(|x: &u64| Some(*x as u8));

        engine.send(&100u32);

        let result = engine.receive();
        assert_eq!(result, Some(100u8));
    }

    #[test]
    fn test_new_engine_multiple_outputs() {
        struct Duplicate;
        impl Stage<u32, u32> for Duplicate {
            fn process<C>(&mut self, data: &u32, collector: &mut C)
            where
                C: crate::stage::OutputCollector<u32>,
            {
                collector.push(data);
                collector.push(&(data + 1));
            }
        }

        let mut engine = StageEngine::<u32, u32>::new()
            .add_stage(Duplicate)
            .add_stage(|x: &u32| Some(*x as u64));

        engine.send(&10u32);

        assert_eq!(engine.receive(), Some(10u64));
        assert_eq!(engine.receive(), Some(11u64));
    }

    #[test]
    fn test_engine_concurrency() {
        let mut engine = StageEngine::<u32, u32>::new().add_stage(|x: &u32| {
            // Simulate some work
            thread::sleep(Duration::from_millis(10));
            Some(*x * 2)
        });

        engine.send(&1);
        engine.send(&2);
        engine.send(&3);

        assert_eq!(engine.receive(), Some(2));
        assert_eq!(engine.receive(), Some(4));
        assert_eq!(engine.receive(), Some(6));
    }
}
