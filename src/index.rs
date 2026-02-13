use crate::components::{Index, IndexReader, StoreReader};
use bytemuck::Pod;
use crossbeam_skiplist::SkipMap;
use std::sync::Arc;

pub struct DirectIndex<Key: Pod + Ord + Send, Value: Pod + Send, Reader: StoreReader<Value>> {
    pub(crate) map: Arc<SkipMap<Key, Value>>,
    pub reader: Reader,
}

pub struct DirectIndexReader<Key: Pod + Ord + Send, Value: Pod + Send> {
    pub(crate) map: Arc<SkipMap<Key, Value>>,
}

impl<Key, Value, Reader: StoreReader<Value>> Index<Key, Value> for DirectIndex<Key, Value, Reader>
where
    Key: Pod + Ord + Send,
    Value: Pod + Send,
{
    type Reader = DirectIndexReader<Key, Value>;
    fn compute(&self, key_fn: impl FnOnce(&Value) -> Key) {
        if self.reader.next()
            && let Some(value) = self.reader.get()
        {
            let key = key_fn(&value);
            self.map.insert(key, value);
        }
    }

    fn reader(&self) -> DirectIndexReader<Key, Value> {
        DirectIndexReader {
            map: self.map.clone(),
        }
    }

    fn iter(&self) -> impl Iterator<Item = (Key, Value)> + '_ {
        self.map.iter().map(|entry| (*entry.key(), *entry.value()))
    }
}

impl<Key, Value> IndexReader<Key, Value> for DirectIndexReader<Key, Value>
where
    Key: Pod + Ord + Send,
    Value: Pod + Send,
{
    fn with<R>(&self, key: &Key, handler: impl FnOnce(&Value) -> R) -> Option<R> {
        self.map.get(key).map(|entry| handler(entry.value()))
    }

    fn get(&self, key: &Key) -> Option<Value> {
        self.map.get(key).map(|entry| *entry.value())
    }

    fn iter(&self) -> impl Iterator<Item = (Key, Value)> + '_ {
        self.map.iter().map(|entry| (*entry.key(), *entry.value()))
    }
}
