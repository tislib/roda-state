use spdlog::info;
use std::time::Instant;

/// A pipe that logs progress information.
pub fn progress<T>(name: impl Into<String>, interval: usize) -> impl FnMut(T) -> Option<T>
where
    T: bytemuck::Pod + Send,
{
    assert!(interval > 0, "interval must be greater than 0");
    let name = name.into();
    let mut count: usize = 0;
    let mut last_instant = Instant::now();
    let start_instant = last_instant;

    move |item| {
        count += 1;
        if count.is_multiple_of(interval) {
            let now = Instant::now();
            let elapsed = now.duration_since(last_instant);
            let total_elapsed = now.duration_since(start_instant);

            let mps = interval as f64 / elapsed.as_secs_f64();
            let total_mps = count as f64 / total_elapsed.as_secs_f64();

            info!(
                "[{}] Processed {} messages, Rate: {} msg/s, Avg: {} msg/s",
                name,
                format_count(count as f64),
                format_count(mps),
                format_count(total_mps)
            );
            last_instant = now;
        }
        Some(item)
    }
}

fn format_count(val: f64) -> String {
    if val < 1000.0 {
        if val == val.floor() {
            format!("{:.0}", val)
        } else {
            format!("{:.2}", val)
        }
    } else if val < 1_000_000.0 {
        format!("{:.2}k", val / 1000.0)
    } else if val < 1_000_000_000.0 {
        format!("{:.2}m", val / 1_000_000.0)
    } else if val < 1_000_000_000_000.0 {
        format!("{:.2}b", val / 1_000_000_000.0)
    } else {
        format!("{:.2}t", val / 1_000_000_000_000.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_progress_logic() {
        let mut pipe = progress("test", 2);

        // Process 1st item
        let res = pipe(1u32);
        assert_eq!(res, Some(1));

        // Process 2nd item - should trigger print
        thread::sleep(Duration::from_millis(10));
        let res = pipe(2u32);
        assert_eq!(res, Some(2));

        // Process 3rd item
        let res = pipe(3u32);
        assert_eq!(res, Some(3));

        // Process 4th item - should trigger print
        thread::sleep(Duration::from_millis(10));
        let res = pipe(4u32);
        assert_eq!(res, Some(4));
    }

    #[test]
    fn test_progress_no_delay() {
        let mut pipe = progress("test_fast", 2);
        for i in 0..10 {
            pipe(i);
        }
    }

    #[test]
    fn test_format_count() {
        assert_eq!(format_count(0.0), "0");
        assert_eq!(format_count(123.0), "123");
        assert_eq!(format_count(123.45), "123.45");
        assert_eq!(format_count(1000.0), "1.00k");
        assert_eq!(format_count(1234.0), "1.23k");
        assert_eq!(format_count(1_000_000.0), "1.00m");
        assert_eq!(format_count(1_234_567.0), "1.23m");
        assert_eq!(format_count(1_000_000_000.0), "1.00b");
        assert_eq!(format_count(1_234_567_890.0), "1.23b");
        assert_eq!(format_count(1_000_000_000_000.0), "1.00t");
    }
}
