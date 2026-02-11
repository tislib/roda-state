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

pub struct RodaStore<State> {
    _p: PhantomData<State>,
}

impl<State> RodaStore<State> {
    pub fn reader(&self) -> RodaStore<State> {
        todo!()
    }
}

impl<State> RodaStore<State> {
    pub fn direct_index<Key>(&self) -> RodaDirectIndex<Key, State> {
        todo!()
    }
    pub fn push(&self, value: State) -> Result<(), RodaError> {
        todo!()
    }

    pub fn with<R>(&self, handler: impl FnOnce(&State) -> R) -> R {
        todo!()
    }
}

impl<State> RodaStore<State> {}

pub struct RodaEngine {}

impl RodaEngine {
    pub fn run_worker(&self, runnable: impl FnOnce() + Send + 'static) {
        thread::spawn(move || {
            runnable();
        });
    }
}

impl RodaEngine {
    pub fn store<State>(&self, size: u32) -> RodaStore<State> {
        todo!()
    }
}

impl RodaEngine {
    pub fn new() -> Self {
        Self {}
    }
}

pub struct Aggregator<InValue, OutValue, PartitionKey = ()> {
    _v: PhantomData<InValue>,
    _out_v: PhantomData<OutValue>,
    _partition_key: PhantomData<PartitionKey>,
}

impl<InValue, OutValue, PartitionKey> Aggregator<InValue, OutValue, PartitionKey> {
    pub fn pipe(source: RodaStore<InValue>, target: RodaStore<OutValue>) -> Self {
        Self {
            _v: Default::default(),
            _out_v: Default::default(),
            _partition_key: Default::default(),
        }
    }

    pub fn partition_by(&mut self, key_fn: impl FnOnce(&InValue) -> PartitionKey) {}

    pub fn reduce(&mut self, update_fn: impl FnOnce(u64, &InValue, &mut OutValue)) {}
}

pub struct Window<InValue, OutValue = ()> {
    _v: PhantomData<InValue>,
    _out_v: PhantomData<OutValue>,
}

impl<InValue, OutValue> Window<InValue, OutValue> {
    pub fn pipe(source: RodaStore<InValue>, target: RodaStore<OutValue>) -> Self {
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
