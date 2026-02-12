use bytemuck::Pod;
use crate::RodaDirectIndex;

pub trait RodaStore<State: Pod, Reader: RodaStoreReader<State>> {
    fn push(&mut self, state: State);
    fn reader(&self) -> Reader;
    fn direct_index<Key>(&self) -> RodaDirectIndex<Key, State>;
}

pub trait RodaStoreReader<State: Pod> {
    fn collect<const N: usize>(&self) -> [&State; N];
    fn next(&self) -> bool;
    fn with<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R>;
    fn at<R>(&self, handler: impl FnOnce(&State) -> R) -> Option<R>;
}
