use crate::components::{Store, StoreReader};
use bytemuck::Pod;
use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::hash::Hash;
use std::marker::PhantomData;

pub struct Aggregator<InValue: Pod, OutValue: Pod, PartitionKey = ()> {
    pub(crate) _v: PhantomData<InValue>,
    pub(crate) _out_v: PhantomData<OutValue>,
    pub(crate) _partition_key: PhantomData<PartitionKey>,
    pub(crate) last_index: Cell<usize>,
    pub(crate) states: RefCell<HashMap<PartitionKey, (u64, OutValue)>>,
}

impl<InValue: Pod, OutValue: Pod, PartitionKey> Aggregator<InValue, OutValue, PartitionKey> {
    pub fn new() -> Aggregator<InValue, OutValue, PartitionKey> {
        Self {
            _v: PhantomData,
            _out_v: PhantomData,
            _partition_key: PhantomData,
            last_index: Cell::new(0),
            states: RefCell::new(HashMap::new()),
        }
    }
}

impl<InValue: Pod, OutValue: Pod, PartitionKey> Default
    for Aggregator<InValue, OutValue, PartitionKey>
{
    fn default() -> Self {
        Self::new()
    }
}

impl<InValue: Pod + Send, OutValue: Pod + Send, PartitionKey>
    Aggregator<InValue, OutValue, PartitionKey>
{
    pub fn from<'a, R: StoreReader<InValue>>(
        &'a self,
        reader: &'a R,
    ) -> AggregatorFrom<'a, InValue, OutValue, PartitionKey, R> {
        AggregatorFrom {
            aggregator: self,
            reader,
            _in: PhantomData,
            _out_v: PhantomData,
            _partition_key: PhantomData,
        }
    }

    pub fn pipe(_source: impl Store<InValue>, _target: impl Store<OutValue>) -> Self {
        Self::new()
    }
}

pub struct AggregatorFrom<
    'a,
    InValue: Pod + Send,
    OutValue: Pod + Send,
    PartitionKey,
    R: StoreReader<InValue>,
> {
    aggregator: &'a Aggregator<InValue, OutValue, PartitionKey>,
    reader: &'a R,
    _in: PhantomData<InValue>,
    _out_v: PhantomData<OutValue>,
    _partition_key: PhantomData<PartitionKey>,
}

impl<'a, InValue: Pod + Send, OutValue: Pod + Send, PartitionKey, R: StoreReader<InValue>>
    AggregatorFrom<'a, InValue, OutValue, PartitionKey, R>
{
    pub fn to<'b, S: Store<OutValue>>(
        self,
        store: &'b mut S,
    ) -> AggregatorTo<'a, 'b, InValue, OutValue, PartitionKey, R, S> {
        AggregatorTo {
            aggregator: self.aggregator,
            reader: self.reader,
            store,
            _in: PhantomData,
            _out: PhantomData,
            _partition_key: PhantomData,
        }
    }
}

pub struct AggregatorTo<
    'a,
    'b,
    InValue: Pod + Send,
    OutValue: Pod + Send,
    PartitionKey,
    R: StoreReader<InValue>,
    S: Store<OutValue>,
> {
    aggregator: &'a Aggregator<InValue, OutValue, PartitionKey>,
    reader: &'a R,
    store: &'b mut S,
    _in: PhantomData<InValue>,
    _out: PhantomData<OutValue>,
    _partition_key: PhantomData<PartitionKey>,
}

impl<
    'a,
    'b,
    InValue: Pod + Send,
    OutValue: Pod + Send,
    PartitionKey,
    R: StoreReader<InValue>,
    S: Store<OutValue>,
> AggregatorTo<'a, 'b, InValue, OutValue, PartitionKey, R, S>
{
    pub fn partition_by<F>(
        self,
        key_fn: F,
    ) -> AggregatorPartition<'a, 'b, InValue, OutValue, PartitionKey, R, S, F>
    where
        F: Fn(&InValue) -> PartitionKey,
    {
        AggregatorPartition {
            aggregator: self.aggregator,
            reader: self.reader,
            store: self.store,
            key_fn,
            _in: PhantomData,
            _out: PhantomData,
            _key: PhantomData,
        }
    }
}

pub struct AggregatorPartition<
    'a,
    'b,
    InValue: Pod + Send,
    OutValue: Pod + Send,
    PartitionKey,
    R,
    S,
    F,
> {
    aggregator: &'a Aggregator<InValue, OutValue, PartitionKey>,
    reader: &'a R,
    store: &'b mut S,
    key_fn: F,
    _in: PhantomData<InValue>,
    _out: PhantomData<OutValue>,
    _key: PhantomData<PartitionKey>,
}

impl<'a, 'b, InValue, OutValue, PartitionKey, R, S, F>
    AggregatorPartition<'a, 'b, InValue, OutValue, PartitionKey, R, S, F>
where
    InValue: Pod + Send,
    OutValue: Pod + Send,
    PartitionKey: Hash + Eq + Send,
    R: StoreReader<InValue>,
    S: Store<OutValue>,
    F: Fn(&InValue) -> PartitionKey,
{
    pub fn reduce(self, mut update_fn: impl FnMut(u64, &InValue, &mut OutValue)) {
        let mut states = self.aggregator.states.borrow_mut();
        let mut last_index = self.aggregator.last_index.get();

        let current_index = self.reader.get_index();
        if current_index > last_index {
            if let Some(val) = self.reader.get() {
                let key = (self.key_fn)(&val);
                let (index, mut state) =
                    states.get(&key).cloned().unwrap_or((0, OutValue::zeroed()));

                update_fn(index, &val, &mut state);
                self.store.push(state);

                states.insert(key, (index + 1, state));
            }
            last_index = current_index;
            self.aggregator.last_index.set(last_index);
        }
    }
}
