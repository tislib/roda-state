use crate::light_mbo_delta::MboDelta;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct BookLevelEntry {
    pub ts: u64,
    pub symbol: u64, // or instrument_id
    pub price: i64,
    pub volume: u64, // "size" is also common
    pub side: u8,    // 0=Bid, 1=Ask
    pub _pad: [u8; 7],
}

impl BookLevelEntry {
    pub fn init(entry: &MboDelta) -> Self {
        let mut delta = 0;
        if entry.delta > 0 {
            delta = entry.delta;
        }
        Self {
            ts: entry.ts,
            symbol: entry.instrument_id as u64,
            price: entry.price,
            volume: delta as u64,
            side: entry.side as u8,
            _pad: [0; 7],
        }
    }
    pub fn update(curr: &mut BookLevelEntry, entry: &MboDelta) {
        if (entry.delta + curr.volume as i32) < 0 {
            return;
        }
        curr.volume = (curr.volume as i32 + entry.delta) as u64;
    }
}
