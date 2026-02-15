use bytemuck::{Pod, Zeroable};
use dbn::record::MboMsg;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct MboDelta {
    /// 1. The Event Timestamp (UNIX nanos).
    ///    Essential for detecting "Flash Crash" speed or latency.
    pub ts: u64,

    /// 3. The Price.
    ///    Signed integer (fixed precision, usually 1e-9).
    pub price: i64,

    /// 4. The Size (Quantity).
    pub delta: i32,

    // --- PACKING SECTION (32-Bit Alignment) ---
    /// 5. The Instrument ID (from Header).
    ///    Needed if your store contains multiple symbols (e.g., MSFT and AAPL).
    pub instrument_id: u32,
    pub side: u64,
}
