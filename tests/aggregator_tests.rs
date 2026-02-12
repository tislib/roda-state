use roda_core::{Aggregator, RodaEngine};
use bytemuck::{Pod, Zeroable};
use roda_core::components::{RodaStore, RodaStoreReader};

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Pod, Zeroable)]
pub struct SensorReading {
    pub value: f64,
    pub sensor_id: u16,
    pub _pad: [u8; 6],
}

#[repr(C)]
#[derive(Debug, Clone, Copy, Default, PartialEq, Pod, Zeroable)]
pub struct SensorStats {
    pub sum: f64,
    pub min: f64,
    pub max: f64,
    pub count: u32,
    pub sensor_id: u16,
    pub _pad: [u8; 2],
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GroupKey {
    pub sensor_id: u16,
    pub group_id: u16,
}

#[test]
fn test_aggregator_count_and_sum() {
    let engine = RodaEngine::new();
    let mut source = engine.store::<SensorReading>(1024);
    let mut target = engine.store::<SensorStats>(1024);

    let source_reader = source.reader();
    let target_reader = target.reader();
    let mut aggregator: Aggregator<SensorReading, SensorStats, u16> = Aggregator::new();

    // Run aggregation inside worker
    engine.run_worker(move || {
        source_reader.next();
        aggregator
            .from(&source_reader)
            .to(&mut target)
            .partition_by(|r| r.sensor_id)
            .reduce(|index, reading, stats| {
                stats.sensor_id = reading.sensor_id;
                stats.count = (index + 1) as u32;
                stats.sum += reading.value;
            });
    });

    // Push readings
    source.push(SensorReading { sensor_id: 1, value: 10.0, ..Default::default() });
    source.push(SensorReading { sensor_id: 1, value: 20.0, ..Default::default() });

    // Validate the final aggregated result by collecting from the target
    let res = target_reader.collect::<2>();
    assert_eq!(res[1].sensor_id, 1);
    assert_eq!(res[1].count, 2);
    assert_eq!(res[1].sum, 30.0);
}

#[test]
fn test_aggregator_min_max_tracking() {
    let engine = RodaEngine::new();
    let mut source = engine.store::<SensorReading>(1024);
    let mut target = engine.store::<SensorStats>(1024);

    let source_reader = source.reader();
    let target_reader = target.reader();
    let mut aggregator: Aggregator<SensorReading, SensorStats, u16> = Aggregator::new();

    // Run aggregation inside worker
    engine.run_worker(move || {
        source_reader.next();
        aggregator
            .from(&source_reader)
            .to(&mut target)
            .partition_by(|r| r.sensor_id)
            .reduce(|index, reading, stats| {
                if index == 0 {
                    stats.min = reading.value;
                    stats.max = reading.value;
                } else {
                    stats.min = stats.min.min(reading.value);
                    stats.max = stats.max.max(reading.value);
                }
                stats.sensor_id = reading.sensor_id;
            });
    });

    // Push readings
    source.push(SensorReading { sensor_id: 1, value: 10.0, ..Default::default() });
    source.push(SensorReading { sensor_id: 1, value: 20.0, ..Default::default() });
    source.push(SensorReading { sensor_id: 1, value: 5.0, ..Default::default() });

    // Validate by collecting from the target
    let res = target_reader.collect::<3>();
    assert_eq!(res[2].min, 5.0);
    assert_eq!(res[2].max, 20.0);
}

#[test]
fn test_aggregator_multiple_partitions() {
    let engine = RodaEngine::new();
    let mut source = engine.store::<SensorReading>(1024);
    let mut target = engine.store::<SensorStats>(1024);

    let source_reader = source.reader();
    let target_reader = target.reader();
    let mut aggregator: Aggregator<SensorReading, SensorStats, u16> = Aggregator::new();

    // Run aggregation inside worker
    engine.run_worker(move || {
        source_reader.next();
        aggregator
            .from(&source_reader)
            .to(&mut target)
            .partition_by(|r| r.sensor_id)
            .reduce(|index, reading, stats| {
                stats.sensor_id = reading.sensor_id;
                stats.count = (index + 1) as u32;
            });
    });

    // Push readings across partitions
    source.push(SensorReading { sensor_id: 1, value: 1.0, ..Default::default() });
    source.push(SensorReading { sensor_id: 2, value: 2.0, ..Default::default() });
    source.push(SensorReading { sensor_id: 1, value: 3.0, ..Default::default() });

    // Validate by collecting all results
    let res = target_reader.collect::<3>();
    assert_eq!(res[0].sensor_id, 1);
    assert_eq!(res[0].count, 1);
    assert_eq!(res[1].sensor_id, 2);
    assert_eq!(res[1].count, 1);
    assert_eq!(res[2].sensor_id, 1);
    assert_eq!(res[2].count, 2);
}

#[test]
fn test_aggregator_complex_key() {
    let engine = RodaEngine::new();
    let mut source = engine.store::<SensorReading>(1024);
    let mut target = engine.store::<SensorStats>(1024);

    let source_reader = source.reader();
    let target_reader = target.reader();
    let mut aggregator: Aggregator<SensorReading, SensorStats, GroupKey> = Aggregator::new();

    // Run aggregation with complex key inside worker
    engine.run_worker(move || {
        source_reader.next();
        aggregator
            .from(&source_reader)
            .to(&mut target)
            .partition_by(|r| GroupKey {
                sensor_id: r.sensor_id,
                group_id: (r.value / 10.0) as u16,
            })
            .reduce(|index, reading, stats| {
                stats.sensor_id = reading.sensor_id;
                stats.count = (index + 1) as u32;
            });
    });

    source.push(SensorReading { sensor_id: 1, value: 15.0, ..Default::default() });

    let res = target_reader.collect::<1>();
    assert_eq!(res[0].sensor_id, 1);
    assert_eq!(res[0].count, 1);
}

#[test]
fn test_aggregator_reset_behavior() {
    let engine = RodaEngine::new();
    let mut source = engine.store::<SensorReading>(10);
    let mut target = engine.store::<SensorStats>(10);

    let source_reader = source.reader();
    let target_reader = target.reader();
    let mut aggregator: Aggregator<SensorReading, SensorStats, u16> = Aggregator::new();

    // Run aggregation inside worker
    engine.run_worker(move || {
        source_reader.next();
        aggregator
            .from(&source_reader)
            .to(&mut target)
            .partition_by(|r| r.sensor_id)
            .reduce(|index, reading, stats| {
                stats.sensor_id = reading.sensor_id;
                stats.count = (index + 1) as u32;
            });
    });

    // Push several readings for sensor 1
    for i in 0..5 {
        source.push(SensorReading { sensor_id: 1, value: i as f64, ..Default::default() });
    }

    // Switch to sensor 2
    source.push(SensorReading { sensor_id: 2, value: 100.0, ..Default::default() });

    // Validate collected results: first 5 for sensor 1 with counts 1..5, then sensor 2 with count 1
    let res = target_reader.collect::<6>();
    for i in 0..5 {
        assert_eq!(res[i].sensor_id, 1);
        assert_eq!(res[i].count, (i as u32) + 1);
    }
    assert_eq!(res[5].sensor_id, 2);
    assert_eq!(res[5].count, 1);
}

#[test]
fn test_aggregator_large_index() {
    let engine = RodaEngine::new();
    let mut source = engine.store::<SensorReading>(1024);
    let mut target = engine.store::<SensorStats>(1024);
    let source_reader = source.reader();
    let target_reader = target.reader();
    let mut aggregator: Aggregator<SensorReading, SensorStats, u16> = Aggregator::new();

    // Run aggregation inside worker
    engine.run_worker(move || {
        source_reader.next();
        aggregator
            .from(&source_reader)
            .to(&mut target)
            .partition_by(|r| r.sensor_id)
            .reduce(|index, _reading, stats| {
                stats.count = (index + 1) as u32;
            });
    });

    // Simulate 1000 items in one partition
    for i in 0..1000 {
        source.push(SensorReading { sensor_id: 1, value: i as f64, ..Default::default() });
    }

    // Validate all results
    let res = target_reader.collect::<1000>();
    for i in 0..1000usize {
        assert_eq!(res[i].count, (i as u32) + 1);
    }
}

#[test]
fn test_aggregator_worker_large() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::thread;

    let engine = RodaEngine::new();
    let mut source = engine.store::<SensorReading>(2000);
    let mut target = engine.store::<SensorStats>(2000);
    let source_reader = source.reader();
    let target_reader = target.reader();
    
    let mut aggregator: Aggregator<SensorReading, SensorStats, u16> = Aggregator::new();
    
    engine.run_worker(move || {
        source_reader.next();
        aggregator
            .from(&source_reader)
            .to(&mut target)
            .partition_by(|r| r.sensor_id)
            .reduce(|index, reading, stats| {
                stats.sensor_id = reading.sensor_id;
                stats.count = (index + 1) as u32;
                stats.sum += reading.value;
            });
    });
    
    for _ in 0..1000 {
        source.push(SensorReading { sensor_id: 1, value: 1.0, ..Default::default() });
    }
    
    let res = target_reader.collect::<1000>();
    assert_eq!(res[999].count, 1000);
    assert_eq!(res[999].sum, 1000.0);
}
