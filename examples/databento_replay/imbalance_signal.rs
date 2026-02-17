use bytemuck::{Pod, Zeroable};

#[repr(C, align(64))]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct ImbalanceSignal {
    pub ts: u64,
    pub ts_recv: u64,
    pub symbol: u64,
    pub imbalance: f64,
    pub bid_vol: f64,
    pub ask_vol: f64,
    pub _pad: [u64; 2],
}
