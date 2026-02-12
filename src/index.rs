use crate::components::{Index, IndexReader};
use bytemuck::Pod;
use std::marker::PhantomData;

pub struct DirectIndex<Key: Pod, Value: Pod> {
    pub(crate) _k: PhantomData<Key>,
    pub(crate) _v: PhantomData<Value>,
}

pub struct RodaDirectIndexReader<Key: Pod, Value: Pod> {
    pub(crate) _k: PhantomData<Key>,
    pub(crate) _v: PhantomData<Value>,
}

impl<Key: Pod, Value: Pod> Index<Key, Value> for DirectIndex<Key, Value> {
    type Reader = RodaDirectIndexReader<Key, Value>;
    fn compute(&self, _key_fn: impl FnOnce(&Value) -> Key) {
        todo!()
    }

    fn reader(&self) -> RodaDirectIndexReader<Key, Value> {
        todo!()
    }
}

impl<Key: Pod, Value: Pod> IndexReader<Key, Value> for RodaDirectIndexReader<Key, Value> {
    fn with<R>(&self, _key: &Key, _handler: impl FnOnce(&Value) -> R) -> Option<R> {
        todo!()
    }

    fn get(&self, _key: &Key) -> Option<Value> {
        todo!()
    }
}
