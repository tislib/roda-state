use roda_core::{RodaEngine, Window};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct DataPoint {
    pub value: f64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct Analysis {
    pub average: f64,
    pub is_increasing: bool,
}

#[test]
fn test_window_filling_and_sliding() {
    let engine = RodaEngine::new();
    let source = engine.store::<DataPoint>(10);
    let target = engine.store::<Analysis>(10);
    let target_reader = target.reader();
    let mut pipeline = Window::pipe(source.reader(), target);

    // Run window reduce inside worker
    engine.run_worker(move || {
        pipeline.reduce(3, |window| {
            if window.len() < 3 {
                return None;
            }
            let sum: f64 = window.iter().map(|d| d.value).sum();
            let increasing = window[2].value > window[1].value && window[1].value > window[0].value;
            Some(Analysis {
                average: sum / 3.0,
                is_increasing: increasing,
            })
        });
    });

    // Push data points
    for i in 1..=5 {
        source.push(DataPoint { value: i as f64 }).unwrap();
    }

    // Validate by collecting all outputs (5 - 3 + 1 = 3)
    let res = target_reader.collect::<3>();
    assert_eq!(res[0].average, 2.0);
    assert!(res[0].is_increasing);
    assert_eq!(res[1].average, 3.0);
    assert!(res[1].is_increasing);
    assert_eq!(res[2].average, 4.0);
    assert!(res[2].is_increasing);
}

#[test]
fn test_window_size_one() {
    let engine = RodaEngine::new();
    let source = engine.store::<DataPoint>(10);
    let target = engine.store::<Analysis>(10);
    let target_reader = target.reader();
    let mut pipeline = Window::pipe(source.reader(), target);

    engine.run_worker(move || {
        // Window size 1 should process every item individually
        pipeline.reduce(1, |window| {
            assert_eq!(window.len(), 1);
            Some(Analysis {
                average: window[0].value,
                is_increasing: false,
            })
        });
    });

    // Push values
    for v in [10.0, 20.0, 30.0] {
        source.push(DataPoint { value: v }).unwrap();
    }

    let res = target_reader.collect::<3>();
    assert_eq!(res[0].average, 10.0);
    assert!(!res[0].is_increasing);
    assert_eq!(res[1].average, 20.0);
    assert!(!res[1].is_increasing);
    assert_eq!(res[2].average, 30.0);
    assert!(!res[2].is_increasing);
}

#[test]
fn test_window_large_sliding() {
    let engine = RodaEngine::new();
    let source = engine.store::<DataPoint>(100);
    let target = engine.store::<Analysis>(100);
    let target_reader = target.reader();
    let mut pipeline = Window::pipe(source.reader(), target);

    engine.run_worker(move || {
        // Larger window size
        pipeline.reduce(10, |window| {
            if window.len() < 10 {
                return None;
            }
            let sum: f64 = window.iter().map(|d| d.value).sum();
            Some(Analysis {
                average: sum / 10.0,
                is_increasing: window[9].value > window[0].value,
            })
        });
    });

    // Push values 0..11 -> expect 3 outputs
    for i in 0..12 {
        source.push(DataPoint { value: i as f64 }).unwrap();
    }

    let res = target_reader.collect::<3>();
    assert_eq!(res[0].average, 4.5);
    assert!(res[0].is_increasing);
    assert_eq!(res[1].average, 5.5);
    assert!(res[1].is_increasing);
    assert_eq!(res[2].average, 6.5);
    assert!(res[2].is_increasing);
}

#[test]
fn test_window_worker_large() {
    use std::sync::{Arc, Mutex};
    use std::time::Duration;
    use std::thread;

    let engine = RodaEngine::new();
    let source = engine.store::<DataPoint>(2000);
    let target = engine.store::<Analysis>(2000);
    let target_reader = target.reader();
    let mut pipeline = Window::pipe(source.reader(), target);

    engine.run_worker(move || {
        pipeline.reduce(10, |window| {
            if window.len() < 10 {
                return None;
            }
            let sum: f64 = window.iter().map(|d| d.value).sum();
            Some(Analysis {
                average: sum / 10.0,
                is_increasing: window[window.len()-1].value > window[0].value,
            })
        });
    });

    for i in 0..1000 {
        source.push(DataPoint { value: i as f64 }).unwrap();
    }

    let res = target_reader.collect::<991>();
    assert_eq!(res[0].average, 4.5); // (0+1+2+3+4+5+6+7+8+9)/10 = 45/10 = 4.5
    assert!(res[0].is_increasing);
}

#[test]
fn test_window_max_value() {
    let engine = RodaEngine::new();
    let source = engine.store::<DataPoint>(10);
    let target = engine.store::<f64>(10);
    let target_reader = target.reader();
    let mut pipeline = Window::pipe(source.reader(), target);

    engine.run_worker(move || {
        pipeline.reduce(3, |window| {
            window.iter().map(|d| d.value).max_by(|a, b| a.total_cmp(b))
        });
    });

    // Push values: expect maxima per 3-sized window
    for v in [1.0, 3.0, 2.0, 5.0, 4.0] {
        source.push(DataPoint { value: v }).unwrap();
    }

    let res = target_reader.collect::<3>();
    assert_eq!(*res[0], 3.0);
    assert_eq!(*res[1], 5.0);
    assert_eq!(*res[2], 5.0);
}

#[test]
fn test_window_all_none_until_full() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicUsize, Ordering};

    let engine = RodaEngine::new();
    let source = engine.store::<DataPoint>(10);
    let target = engine.store::<bool>(10);
    let target_reader = target.reader();
    let mut pipeline = Window::pipe(source.reader(), target);

    let call_count = Arc::new(AtomicUsize::new(0));
    let cc = call_count.clone();
    engine.run_worker(move || {
        pipeline.reduce(5, |window| {
            cc.fetch_add(1, Ordering::Relaxed);
            if window.len() == 5 {
                Some(true)
            } else {
                None
            }
        });
    });

    for i in 0..5 {
        source.push(DataPoint { value: i as f64 }).unwrap();
    }

    let res = target_reader.collect::<1>();
    assert_eq!(*res[0], true);
}
