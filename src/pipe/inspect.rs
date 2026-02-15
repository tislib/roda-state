/// Passes the item through while performing a side effect.
pub fn inspect<T>(mut f: impl FnMut(&T)) -> impl FnMut(T) -> Option<T>
where
    T: bytemuck::Pod + Send,
{
    move |item| {
        f(&item);
        Some(item)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    #[test]
    fn test_inspect_logic() {
        let count = Arc::new(AtomicUsize::new(0));
        let mut pipe = inspect(|_x: &u32| {
            count.fetch_add(1, Ordering::Relaxed);
        });

        let res = pipe(42);

        assert_eq!(res, Some(42));
        assert_eq!(count.load(Ordering::Relaxed), 1);
    }
}
