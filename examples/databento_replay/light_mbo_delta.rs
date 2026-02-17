use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct MboDelta {
    /// 1. The Event Timestamp (UNIX nanos).
    pub ts: u64,

    /// 2. The Local Receive Timestamp.
    pub ts_recv: u64,

    /// 3. The Price.
    pub price: i64,

    /// 4. The Size (Quantity) change.
    pub delta: i32,

    // --- PACKING SECTION (32-Bit Alignment) ---
    /// 5. The Instrument ID.
    pub instrument_id: u32,

    /// 6. Side (b'A' or b'B').
    pub side: u8,

    /// 7. Clear flag.
    pub is_clear: u8,

    /// 8. Padding.
    pub _pad: [u8; 6],
}
