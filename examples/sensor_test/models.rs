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
    pub avg: f64,
    pub count: u64,
    pub timestamp: u64,
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
    pub _pad0: i32,
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
            avg: r.value,
            count: 1,
            timestamp: (r.timestamp / 100_000) * 100_000,
        }
    }

    /// Update the existing summary with a new reading.
    #[inline(always)]
    pub fn update(&mut self, r: &Reading) {
        // Update Min/Max
        if r.value < self.min {
            self.min = r.value;
        }
        if r.value > self.max {
            self.max = r.value;
        }

        // Online Average Calculation:
        // new_avg = ((old_avg * count) + new_val) / (count + 1)
        self.avg = (self.avg * self.count as f64 + r.value) / (self.count + 1) as f64;
        self.count += 1;
    }
}
