use bytemuck::{Pod, Zeroable};

/// Raw sensor reading
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable, PartialEq)]
pub struct Reading {
    pub service_id: u64,
    pub value: f64,
    pub timestamp: u64,
}

impl Reading {
    pub fn from(service_id: u64, value: f64, timestamp: u64) -> Self {
        Self {
            service_id,
            value,
            timestamp,
        }
    }
}

/// Key used for partitioning and indexing summaries (100ms buckets)
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ServiceKey {
    pub service_id: u64,
    pub timestamp: u64,
}

impl ServiceKey {
    #[inline(always)]
    pub fn from_reading(r: &Reading) -> Self {
        Self {
            service_id: r.service_id,
            // Aligns to 100,000 unit (100ms) windows
            timestamp: (r.timestamp / 100_000) * 100_000,
        }
    }
}

/// Statistical summary of readings for a time window
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable, PartialEq)]
pub struct Summary {
    pub service_id: u64,
    pub min: f64,
    pub max: f64,
    pub sum: f64,
    pub count: u64,
    pub timestamp: u64,
    pub _pad: [u64; 2],
}

impl Summary {
    #[inline(always)]
    pub fn init(r: &Reading) -> Self {
        Self {
            service_id: r.service_id,
            min: r.value,
            max: r.value,
            sum: 0.0,
            count: 1,
            timestamp: (r.timestamp / 100_000) * 100_000,
            _pad: [0; 2],
        }
    }

    #[inline(always)]
    pub fn update(&mut self, r: &Reading) {
        self.min = self.min.min(r.value);
        self.max = self.max.max(r.value);
        self.sum += r.value;
        self.count += 1;
    }

    #[inline(always)]
    pub fn avg(&self) -> f64 {
        self.sum / self.count as f64
    }
}

/// Alert generated when an anomaly is detected
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable, PartialEq)]
pub struct Alert {
    pub service_id: u64,
    pub timestamp: u64,
    pub severity: i32,
    pub _pad0: [i32; 3],
}
