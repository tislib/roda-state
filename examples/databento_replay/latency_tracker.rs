use std::sync::LazyLock;
use std::time::Instant;

pub static START_TIME: LazyLock<Instant> = LazyLock::new(Instant::now);

#[inline(always)]
pub fn get_relative_nanos() -> u64 {
    START_TIME.elapsed().as_nanos() as u64
}