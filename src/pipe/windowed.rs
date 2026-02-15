/// Aligns a timestamp to the start of a fixed-duration window.
#[inline(always)]
pub fn windowed(timestamp: u64, window_size: u64) -> u64 {
    if window_size == 0 {
        return timestamp;
    }
    (timestamp / window_size) * window_size
}

#[cfg(test)]
mod window_tests {
    use super::*;

    #[test]
    fn test_window_alignment() {
        let t1 = 150_200;
        let t2 = 199_999;
        let window = 100_000;

        // Both should fall into the 100,000 bucket
        assert_eq!(windowed(t1, window), 100_000);
        assert_eq!(windowed(t2, window), 100_000);

        // Next bucket
        assert_eq!(windowed(200_001, window), 200_000);
    }
}
