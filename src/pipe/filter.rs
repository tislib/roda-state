/// Only passes items that satisfy the predicate.
pub fn filter<T>(mut predicate: impl FnMut(&T) -> bool) -> impl FnMut(T) -> Option<T>
where
    T: bytemuck::Pod + Send,
{
    move |item| {
        if predicate(&item) { Some(item) } else { None }
    }
}

#[cfg(test)]
mod filter_tests {
    use super::*;

    #[test]
    fn test_filter_logic() {
        let mut pipe = filter(|x: &i32| *x > 0);

        assert_eq!(pipe(10), Some(10));
        assert_eq!(pipe(-5), None);
    }
}
