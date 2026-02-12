use crate::index::DirectIndex;
use bytemuck::Pod;

pub trait Store<State: Pod> {
    type Reader: StoreReader<State>;
    fn push(&mut self, state: State);
    fn reader(&self) -> Self::Reader;
    fn direct_index<Key: Pod>(&self) -> DirectIndex<Key, State>;
}

pub trait StoreReader<State: Pod> {
    fn next(&self) -> bool;

    fn with<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R>;
    fn with_at<R>(&self, at: usize, handler: impl FnOnce(&State) -> R) -> Option<R>;
    fn with_last<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R>;

    fn get(&self) -> Option<State>;
    fn get_at(&self, at: usize) -> Option<State>;
    fn get_last(&self) -> Option<State>;
    fn get_window<const N: usize>(&self, at: usize) -> Option<[State; N]>;
}

pub trait Index<Key: Pod, State: Pod> {
    type Reader: IndexReader<Key, State>;
    fn compute(&self, key_fn: impl FnOnce(&State) -> Key);
    fn reader(&self) -> Self::Reader;
}

pub trait IndexReader<Key: Pod, State: Pod> {
    fn with<R>(&self, key: &Key, handler: impl FnOnce(&State) -> R) -> Option<R>;
    fn get(&self, key: &Key) -> Option<State>;
}
