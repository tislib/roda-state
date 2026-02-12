use crate::RodaDirectIndex;
use bytemuck::Pod;
use std::io::Read;

pub trait RodaStore<State: Pod, Reader: RodaStoreReader<State>> {
    fn push(&mut self, state: State);
    fn reader(&self) -> Reader;
    fn direct_index<Key: Pod>(&self) -> RodaDirectIndex<Key, State>;
}

pub trait RodaStoreReader<State: Pod> {
    fn collect<const N: usize>(&self) -> [&State; N];
    fn next(&self) -> bool;
    fn with<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R>;
    fn with_at<R>(&self, index: usize, handler: impl FnOnce(&State) -> R) -> Option<R>;
    fn get(&self) -> Option<State>
    where
        State: Clone;
    fn get_at(&self, index: usize) -> Option<State>
    where
        State: Clone;
}

pub trait RodaIndex<Key: Pod, State: Pod, Reader: RodaIndexReader<Key, State>> {
    fn compute(&self, key_fn: impl FnOnce(&State) -> Key);
    fn reader(&self) -> Reader;
}

pub trait RodaIndexReader<Key: Pod, State: Pod> {
    fn with<R>(&self, key: &Key, handler: impl FnOnce(&State) -> R) -> Option<R>;
    fn get(&self, key: &Key) -> Option<&State>
    where
        Key: Clone;
}
