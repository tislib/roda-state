use std::collections::HashMap;

/// Only emits the event if the value associated with the key has changed.
pub fn dedup_by<K, T>(mut key_fn: impl FnMut(&T) -> K) -> impl FnMut(T) -> Option<T>
where
    K: std::hash::Hash + Eq,
    T: bytemuck::Pod + Send + Copy + PartialEq,
{
    let mut last_values: HashMap<K, T> = HashMap::new();
    move |curr| {
        let key = key_fn(&curr);
        let prev = last_values.get(&key);

        if let Some(p) = prev {
            if *p == curr {
                // Value hasn't changed; suppress the event
                return None;
            }
        }

        // Value changed or is new; update cache and emit
        last_values.insert(key, curr);
        Some(curr)
    }
}

#[cfg(test)]
mod dedup_tests {
    use super::*;

    #[test]
    fn test_dedup_logic() {
        let mut pipe = dedup_by(|_: &i32| 0); // Use a constant key for global consecutive dedup

        assert_eq!(pipe(10), Some(10)); // First time: pass
        assert_eq!(pipe(10), None); // Same value: drop
        assert_eq!(pipe(20), Some(20)); // New value: pass
        assert_eq!(pipe(10), Some(10)); // Changed back: pass
    }
}
