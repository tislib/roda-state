use std::sync::mpsc::{channel, Sender, Receiver};
use std::thread;
use bytemuck::Pod;
use crate::stage::Stage;

/// A threaded pipeline engine that grows by adding stages.
/// Each stage runs in its own thread.
pub struct NewEngine<In: Pod + Send + 'static, Out: Pod + Send + 'static> {
    input_tx: Sender<In>,
    output_rx: Receiver<Out>,
}

impl<In: Pod + Send + 'static, Out: Pod + Send + 'static> NewEngine<In, Out> {
    /// Adds a new stage to the pipeline.
    /// This method consumes the current engine and returns a new one with the updated output type.
    /// A new thread is spawned to run the provided stage.
    pub fn add_stage<NextOut: Pod + Send + 'static, S: Stage<Out, NextOut> + Send + 'static>(
        self,
        mut stage: S,
    ) -> NewEngine<In, NextOut> {
        let (next_tx, next_rx) = channel();
        let current_rx = self.output_rx;

        thread::spawn(move || {
            while let Ok(data) = current_rx.recv() {
                stage.process(data, &mut |out: NextOut| {
                    let _ = next_tx.send(out);
                });
            }
        });

        NewEngine {
            input_tx: self.input_tx,
            output_rx: next_rx,
        }
    }

    /// Sends data into the start of the pipeline.
    pub fn send(&self, data: In) {
        let _ = self.input_tx.send(data);
    }

    /// Receives data from the end of the pipeline.
    /// This will block until data is available or the pipeline is broken.
    pub fn receive(&self) -> Option<Out> {
        self.output_rx.recv().ok()
    }
}

impl<T: Pod + Send + 'static> NewEngine<T, T> {
    /// Creates a new engine with no stages.
    /// Acts as a passthrough until stages are added.
    pub fn new() -> Self {
        let (tx, rx) = channel();
        Self {
            input_tx: tx,
            output_rx: rx,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_new_engine_threaded_pipeline() {
        let engine = NewEngine::<u32, u32>::new()
            .add_stage(|x: u32| Some(x as u64))
            .add_stage(|x: u64| Some(x as u8));

        engine.send(100u32);
        
        let result = engine.receive();
        assert_eq!(result, Some(100u8));
    }

    #[test]
    fn test_new_engine_multiple_outputs() {
        struct Duplicate;
        impl Stage<u32, u32> for Duplicate {
            fn process<C>(&mut self, data: u32, collector: &mut C)
            where
                C: crate::stage::OutputCollector<u32>,
            {
                collector.push(data);
                collector.push(data + 1);
            }
        }

        let engine = NewEngine::<u32, u32>::new()
            .add_stage(Duplicate)
            .add_stage(|x: u32| Some(x as u64));

        engine.send(10u32);

        assert_eq!(engine.receive(), Some(10u64));
        assert_eq!(engine.receive(), Some(11u64));
    }

    #[test]
    fn test_engine_concurrency() {
        let engine = NewEngine::<u32, u32>::new()
            .add_stage(|x: u32| {
                // Simulate some work
                thread::sleep(Duration::from_millis(10));
                Some(x * 2)
            });

        engine.send(1);
        engine.send(2);
        engine.send(3);

        assert_eq!(engine.receive(), Some(2));
        assert_eq!(engine.receive(), Some(4));
        assert_eq!(engine.receive(), Some(6));
    }
}
