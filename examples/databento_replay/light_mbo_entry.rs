use bytemuck::{Pod, Zeroable};
use dbn::record::MboMsg;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct LightMboEntry {
    /// 1. The Event Timestamp (UNIX nanos).
    ///    Essential for detecting "Flash Crash" speed or latency.
    pub ts: u64,

    /// 2. The Unique Order ID.
    ///    Critical for linking a 'Cancel' message back to the original 'Add'.
    pub order_id: u64,

    /// 3. The Price.
    ///    Signed integer (fixed precision, usually 1e-9).
    pub price: i64,

    /// 4. The Size (Quantity).
    pub size: u32,

    // --- PACKING SECTION (32-Bit Alignment) ---
    /// 5. The Instrument ID (from Header).
    ///    Needed if your store contains multiple symbols (e.g., MSFT and AAPL).
    pub instrument_id: u32,

    /// 6. The Local Receive Timestamp (nanos since UNX EPOCH or just relative).
    pub ts_recv: u64,

    // --- PACKING SECTION (8-Bit Alignment) ---
    /// 7. Action (Add='A', Cancel='C', Modify='M', etc.)
    ///    We store as u8 to match the raw byte.
    pub action: u8,

    /// 8. Side (Bid='B', Ask='A').
    pub side: u8,

    /// 9. Explicit Padding.
    pub _pad: [u8; 6],
}

impl LightMboEntry {
    pub fn from_msg(msg: &MboMsg, ts_recv: u64) -> Self {
        Self {
            ts: msg.hd.ts_event,
            order_id: msg.order_id,
            price: msg.price,
            size: msg.size,
            instrument_id: msg.hd.instrument_id,
            ts_recv,
            action: msg.action as u8,
            side: msg.side as u8,
            _pad: [0; 6],
        }
    }
}
