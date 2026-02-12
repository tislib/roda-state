use crate::components::{Store, StoreReader};
use crate::index::DirectIndex;
use bytemuck::Pod;
use std::marker::PhantomData;

pub struct CircularRodaStore<State: Pod> {
    pub(crate) _p: PhantomData<State>,
}

pub struct CircularRodaStoreReader<State: Pod> {
    pub(crate) _p: PhantomData<State>,
}

impl<State: Pod> Store<State> for CircularRodaStore<State> {
    type Reader = CircularRodaStoreReader<State>;

    fn push(&mut self, _state: State) {
        todo!()
    }

    fn reader(&self) -> CircularRodaStoreReader<State> {
        todo!()
    }

    fn direct_index<Key: Pod>(&self) -> DirectIndex<Key, State> {
        todo!()
    }
}

impl<State: Pod> StoreReader<State> for CircularRodaStoreReader<State> {
    fn next(&self) -> bool {
        todo!()
    }

    fn with<R>(&self, _handler: impl FnOnce(&State) -> R) -> Option<R> {
        todo!()
    }

    fn with_at<R>(&self, _at: usize, _handler: impl FnOnce(&State) -> R) -> Option<R> {
        todo!()
    }

    fn with_last<R>(&self, _handler: impl FnOnce(&State) -> R) -> Option<R> {
        todo!()
    }

    fn get(&self) -> Option<State> {
        todo!()
    }

    fn get_at(&self, _at: usize) -> Option<State> {
        todo!()
    }

    fn get_last(&self) -> Option<State> {
        todo!()
    }

    fn get_window<const N: usize>(&self, _at: usize) -> Option<[State; N]> {
        todo!()
    }
}
