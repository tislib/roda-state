use crate::components::{Store, StoreReader};
use crate::store::CircularRodaStore;
use bytemuck::Pod;
use std::marker::PhantomData;

pub struct Window<InValue, OutValue = ()> {
    pub(crate) _v: PhantomData<InValue>,
    pub(crate) _out_v: PhantomData<OutValue>,
}

impl<InValue: Pod, OutValue: Pod> Window<InValue, OutValue> {
    pub fn from<Reader: StoreReader<InValue>>(
        &self,
        _reader: &Reader,
    ) -> Window<InValue, OutValue> {
        todo!()
    }

    pub fn to<S: Store<OutValue>>(&self, _store: &mut S) -> Window<InValue, OutValue> {
        todo!()
    }
}

impl<InValue, OutValue> Window<InValue, OutValue> {
    pub fn new() -> Window<InValue, OutValue> {
        todo!()
    }
}

impl<InValue: Pod, OutValue: Pod> Window<InValue, OutValue> {
    pub fn pipe(source: impl StoreReader<InValue>, target: CircularRodaStore<OutValue>) -> Self {
        let _ = source;
        let _ = target;
        Self {
            _v: Default::default(),
            _out_v: Default::default(),
        }
    }

    pub fn reduce(
        &mut self,
        _window_size: u32,
        _update_fn: impl FnOnce(&[InValue]) -> Option<OutValue>,
    ) {
    }
}
