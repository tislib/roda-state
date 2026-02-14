use bytemuck::{Pod, Zeroable};
use dbn::record::MboMsg;

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct LightMboEntry {
    /// 1. The Event Timestamp (UNIX nanos).
    /// Essential for detecting "Flash Crash" speed or latency.
    pub ts: u64,

    /// 2. The Unique Order ID.
    /// Critical for linking a 'Cancel' message back to the original 'Add'.
    pub order_id: u64,

    /// 3. The Price.
    /// Signed integer (fixed precision, usually 1e-9).
    pub price: i64,

    /// 4. The Size (Quantity).
    pub size: u32,

    // --- PACKING SECTION (32-Bit Alignment) ---
    /// 5. The Instrument ID (from Header).
    /// Needed if your store contains multiple symbols (e.g., MSFT and AAPL).
    pub instrument_id: u32,

    // --- PACKING SECTION (8-Bit Alignment) ---
    /// 6. Action (Add='A', Cancel='C', Modify='M', etc.)
    /// We store as u8 to match the raw byte.
    pub action: u8,

    /// 7. Side (Bid='B', Ask='A').
    pub side: u8,

    /// 8. Explicit Padding.
    /// We have used: 8+8+8+4+4+1+1 = 34 bytes.
    /// The next multiple of 8 (for u64 alignment) is 40.
    /// So we need 6 bytes of padding.
    pub _pad: [u8; 6],
}

impl From<&MboMsg> for LightMboEntry {
    fn from(msg: &MboMsg) -> Self {
        Self {
            ts: msg.hd.ts_event,
            order_id: msg.order_id,
            price: msg.price,
            size: msg.size,
            instrument_id: msg.hd.instrument_id,
            // Cast char (i8) to u8 directly.
            // 'A' is 65, 'B' is 66, etc.
            action: msg.action as u8,
            side: msg.side as u8,
            _pad: [0; 6],
        }
    }
}
