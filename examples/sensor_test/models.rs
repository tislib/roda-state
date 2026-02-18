use bytemuck::{Pod, Zeroable};

/// Raw sensor reading
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Reading {
    pub sensor_id: u64,
    pub value: f64,
    pub timestamp: u64,
}

impl Reading {
    pub fn from(sensor_id: u64, value: f64, timestamp: u64) -> Self {
        Self {
            sensor_id,
            value,
            timestamp,
        }
    }
}

/// Statistical summary of readings for a time window
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Summary {
    pub sensor_id: u64,
    pub min: f64,
    pub max: f64,
    pub sum: f64, // Changed from avg
    pub count: u64,
    pub timestamp: u64,
    pub _pad: [u64; 2], // Pad to 64 bytes (1 cache line)
}

/// Key used for partitioning and indexing summaries
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct SensorKey {
    pub sensor_id: u64,
    pub timestamp: u64,
}

/// Alert generated when an anomaly is detected
#[repr(C)]
#[derive(Debug, Clone, Copy, Default, Pod, Zeroable)]
pub struct Alert {
    pub sensor_id: u64,
    pub timestamp: u64,
    pub severity: i32,
    pub _pad0: [i32; 3],
}

impl SensorKey {
    /// Helper to create a key aligned to a 100ms (100,000 unit) window.
    #[inline(always)]
    pub fn from_reading(r: &Reading) -> Self {
        Self {
            sensor_id: r.sensor_id,
            // Aligning timestamp to the floor of the window
            timestamp: (r.timestamp / 100_000) * 100_000,
        }
    }
}

impl Summary {
    /// Initialize a new summary bucket from the first reading encountered.
    #[inline(always)]
    pub fn init(r: &Reading) -> Self {
        Self {
            sensor_id: r.sensor_id,
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
