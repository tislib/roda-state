use crate::store::{CircularRodaStore, CircularRodaStoreReader};
use bytemuck::Pod;
use std::marker::PhantomData;

pub struct Aggregator<InValue: Pod, OutValue: Pod, PartitionKey = ()> {
    pub(crate) _v: PhantomData<InValue>,
    pub(crate) _out_v: PhantomData<OutValue>,
    pub(crate) _partition_key: PhantomData<PartitionKey>,
}

impl<InValue: Pod, OutValue: Pod, PartitionKey> Aggregator<InValue, OutValue, PartitionKey> {
    pub fn to(
        &self,
        _p0: &mut CircularRodaStore<OutValue>,
    ) -> Aggregator<InValue, OutValue, PartitionKey> {
        todo!()
    }
}

impl<InValue: Pod, OutValue: Pod, PartitionKey> Aggregator<InValue, OutValue, PartitionKey> {
    pub fn from(
        &self,
        _p0: &CircularRodaStoreReader<InValue>,
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
    pub fn pipe(_source: CircularRodaStore<InValue>, _target: CircularRodaStore<OutValue>) -> Self {
        Self {
            _v: Default::default(),
            _out_v: Default::default(),
            _partition_key: Default::default(),
        }
    }

    pub fn partition_by(
        &mut self,
        _key_fn: impl FnOnce(&InValue) -> PartitionKey,
    ) -> Aggregator<InValue, OutValue, PartitionKey> {
        todo!()
    }

    pub fn reduce(&mut self, _update_fn: impl FnOnce(u64, &InValue, &mut OutValue)) {}
}
