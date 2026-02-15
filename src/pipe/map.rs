/// Transforms an item from one type to another.
pub fn map<In, Out>(mut f: impl FnMut(In) -> Out) -> impl FnMut(In) -> Option<Out>
where
    In: bytemuck::Pod + Send,
    Out: bytemuck::Pod + Send,
{
    move |item| Some(f(item))
}

#[cfg(test)]
mod map_tests {
    use super::*;

    #[test]
    fn test_map_logic() {
        // Transform u32 to u64
        let mut pipe = map(|x: u32| x as u64 * 2);

        assert_eq!(pipe(21), Some(42u64));
    }
}
