use std::collections::HashMap;

/// Compares current item with the previous item of the same key.
pub fn delta<K, T, Out>(
    mut key_fn: impl FnMut(&T) -> K,
    mut logic: impl FnMut(T, Option<T>) -> Option<Out>,
) -> impl FnMut(T) -> Option<Out>
where
    K: std::hash::Hash + Eq,
    T: bytemuck::Pod + Send + Copy,
    Out: bytemuck::Pod + Send,
{
    let mut last_values: HashMap<K, T> = HashMap::new();
    move |curr| {
        let key = key_fn(&curr);
        let prev = last_values.get(&key).copied();
        last_values.insert(key, curr);
        logic(curr, prev)
    }
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable, Debug, PartialEq)]
struct Metric {
    pub id: u64,
    pub val: f64,
}

#[test]
fn test_delta_logic() {
    // Return u8 (1 for alert, 0 for none) to satisfy Pod
    let mut pipe = delta(
        |m: &Metric| m.id,
        |curr, prev| match prev {
            Some(p) if curr.val >= p.val + 5.0 => Some(1u8),
            _ => Some(0u8),
        },
    );

    let m1 = Metric { id: 1, val: 10.0 };
    let m2 = Metric { id: 1, val: 17.0 };

    assert_eq!(pipe(m1), Some(0u8));
    assert_eq!(pipe(m2), Some(1u8)); // Alert triggered
}
