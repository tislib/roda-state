use std::collections::HashMap;

/// Manages a per-key state for aggregations.
pub fn stateful<K, In, Out>(
    mut key_fn: impl FnMut(&In) -> K,
    mut init_fn: impl FnMut(&In) -> Out,
    mut fold_fn: impl FnMut(&mut Out, In),
) -> impl FnMut(In) -> Option<Out>
where
    K: std::hash::Hash + Eq,
    In: bytemuck::Pod + Send,
    Out: bytemuck::Pod + Send + Copy,
{
    let mut storage: HashMap<K, Out> = HashMap::new();
    move |item| {
        let key = key_fn(&item);
        let entry = storage
            .entry(key)
            .and_modify(|state| fold_fn(state, item))
            .or_insert_with(|| init_fn(&item));
        Some(*entry)
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Message {
    pub id: u64,
    pub value: i64,
}

#[cfg(test)]
mod stateful_tests {
    use super::*;

    #[test]
    fn test_stateful_logic() {
        // Now using our Pod-compliant struct instead of a tuple
        let mut pipe = stateful(
            |item: &Message| item.id,           // Key: ID
            |item| item.value,                  // Init: First value
            |state, item| *state += item.value, // Fold: Add new value
        );

        let m1 = Message { id: 1, value: 10 };
        let m2 = Message { id: 2, value: 5 };
        let m3 = Message { id: 1, value: 20 };

        assert_eq!(pipe(m1), Some(10));
        assert_eq!(pipe(m2), Some(5));
        assert_eq!(pipe(m3), Some(30));
    }
}
