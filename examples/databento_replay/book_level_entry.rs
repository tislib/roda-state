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
