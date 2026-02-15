use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct ImbalanceSignal {
    pub ts: u64,
    pub symbol: u64,
    pub imbalance: f64,
    pub bid_vol: f64,
    pub ask_vol: f64,
}
