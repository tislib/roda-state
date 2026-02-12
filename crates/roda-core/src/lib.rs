pub mod components;
mod store;

use crate::components::{RodaStore, RodaStoreReader};
use bytemuck::Pod;
use std::fmt::{Debug, Formatter};
use std::marker::PhantomData;
use std::thread;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RodaError {}

pub struct RodaDirectIndex<Key, Value> {
    _k: PhantomData<Key>,
    _v: PhantomData<Value>,
}

impl<Key, Value> RodaDirectIndex<Key, Value> {
    pub fn get(&self, key: &Key) -> Option<&Value> {
        todo!()
    }
}

impl<Key, Value> RodaDirectIndex<Key, Value> {
    pub fn shallow_clone(&self) -> RodaDirectIndex<Key, Value> {
        todo!()
    }
}

impl<Key, Value> RodaDirectIndex<Key, Value> {
    pub fn compute(&self, key_fn: impl FnOnce(&Value) -> Key) {
        todo!()
    }
}

pub struct CircularRodaStore<State: Pod> {
    _p: PhantomData<State>,
}

pub struct CircularRodaStoreReader<State: Pod> {
    _p: PhantomData<State>,
}

// impl<State> RodaStore<State> {
//     pub fn reader(&self) -> RodaStore<State> {
//         todo!()
//     }
//
//     pub fn collect<const N: usize>(&self) -> [&State; N] {
//         todo!()
//     }
// }
//
// impl<State> RodaStore<State> {
//     pub fn direct_index<Key>(&self) -> RodaDirectIndex<Key, State> {
//         todo!()
//     }
//     pub fn push(&self, value: State) -> Result<(), RodaError> {
//         todo!()
//     }
//
//     pub fn with<R>(&self, handler: impl FnOnce(&State) -> R) -> R {
//         todo!()
//     }
// }

impl<State: Pod> RodaStore<State, CircularRodaStoreReader<State>> for CircularRodaStore<State> {
    fn push(&mut self, state: State) {
        todo!()
    }

    fn reader(&self) -> CircularRodaStoreReader<State> {
        todo!()
    }

    fn direct_index<Key>(&self) -> RodaDirectIndex<Key, State> {
        todo!()
    }
}

impl<State: Pod> RodaStoreReader<State> for CircularRodaStoreReader<State> {
    fn collect<const N: usize>(&self) -> [&State; N] {
        todo!()
    }

    fn next(&self) -> bool {
        todo!()
    }

    fn with<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R> {
        todo!()
    }

    fn at<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R> {
        todo!()
    }
}

pub struct RodaEngine {}

impl RodaEngine {
    pub fn run_worker(&self, mut runnable: impl FnMut() + Send + 'static) {
        thread::spawn(move || loop {
            runnable();
        });
    }
}

impl RodaEngine {
    pub fn store<State: Pod>(&self, size: u32) -> CircularRodaStore<State> {
        todo!()
    }
}

impl RodaEngine {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct Aggregator<InValue: Pod, OutValue: Pod, PartitionKey = ()> {
    _v: PhantomData<InValue>,
    _out_v: PhantomData<OutValue>,
    _partition_key: PhantomData<PartitionKey>,
}

impl<InValue: Pod, OutValue: Pod, PartitionKey> Aggregator<InValue, OutValue, PartitionKey> {
    pub fn to(
        &self,
        p0: &mut CircularRodaStore<OutValue>,
    ) -> Aggregator<InValue, OutValue, PartitionKey> {
        todo!()
    }
}

impl<InValue: Pod, OutValue: Pod, PartitionKey> Aggregator<InValue, OutValue, PartitionKey> {
    pub fn from(
        &self,
        p0: &CircularRodaStoreReader<InValue>,
    ) -> Aggregator<InValue, OutValue, PartitionKey> {
        todo!()
    }
}

impl<InValue: Pod, OutValue: Pod, PartitionKey> Aggregator<InValue, OutValue, PartitionKey> {
    pub fn new() -> Aggregator<InValue, OutValue, PartitionKey> {
        todo!()
    }
}

impl<InValue: Pod, OutValue: Pod, PartitionKey> Aggregator<InValue, OutValue, PartitionKey> {
    pub fn pipe(source: CircularRodaStore<InValue>, target: CircularRodaStore<OutValue>) -> Self {
        Self {
            _v: Default::default(),
            _out_v: Default::default(),
            _partition_key: Default::default(),
        }
    }

    pub fn partition_by(
        &mut self,
        key_fn: impl FnOnce(&InValue) -> PartitionKey,
    ) -> Aggregator<InValue, OutValue, PartitionKey> {
        todo!()
    }

    pub fn reduce(&mut self, update_fn: impl FnOnce(u64, &InValue, &mut OutValue)) {}
}

pub struct Window<InValue, OutValue = ()> {
    _v: PhantomData<InValue>,
    _out_v: PhantomData<OutValue>,
}

impl<InValue: Pod, OutValue: Pod> Window<InValue, OutValue> {
    pub fn from<Reader: RodaStoreReader<InValue>>(
        &self,
        reader: &Reader,
    ) -> Window<InValue, OutValue> {
        todo!()
    }

    pub fn to<Reader: RodaStoreReader<OutValue>, Store: RodaStore<OutValue, Reader>>(
        &self,
        store: &mut Store,
    ) -> Window<InValue, OutValue> {
        todo!()
    }
}

impl<InValue, OutValue> Window<InValue, OutValue> {
    pub fn new() -> Window<InValue, OutValue> {
        todo!()
    }
}

impl<InValue: Pod, OutValue: Pod> Window<InValue, OutValue> {
    pub fn pipe(
        source: impl RodaStoreReader<InValue>,
        target: CircularRodaStore<OutValue>,
    ) -> Self {
        Self {
            _v: Default::default(),
            _out_v: Default::default(),
        }
    }

    pub fn reduce(
        &mut self,
        window_size: u32,
        update_fn: impl FnOnce(&[InValue]) -> Option<OutValue>,
    ) {
    }
}
