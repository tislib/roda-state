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
        thread::spawn(runnable);
    }
}

impl RodaEngine {
    pub fn store<State>(&self) -> RodaStore<State> {
        todo!()
    }
}

impl RodaEngine {
    pub fn new() -> Self {
        Self {}
    }
}
