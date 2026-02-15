use roda_state::{OutputCollector, Stage, StageEngine, pipe};
use std::thread;
use std::time::Duration;

#[test]
fn test_basic_pipeline() {
    let mut engine = StageEngine::<u32, u32>::new()
        .add_stage(|x: u32| Some(x + 1))
        .add_stage(|x: u32| Some(x * 2));

    engine.send(10);
    engine.send(20);

    assert_eq!(engine.receive(), Some(22)); // (10 + 1) * 2
    assert_eq!(engine.receive(), Some(42)); // (20 + 1) * 2
}

#[test]
fn test_none_filtering() {
    let mut engine = StageEngine::<u32, u32>::new()
        .add_stage(|x: u32| if x.is_multiple_of(2) { Some(x) } else { None });

    engine.send(1);
    engine.send(2);
    engine.send(3);
    engine.send(4);

    assert_eq!(engine.receive(), Some(2));
    assert_eq!(engine.receive(), Some(4));
}

#[test]
fn test_multiple_outputs() {
    struct Duplicate;
    impl Stage<u32, u32> for Duplicate {
        fn process<C>(&mut self, data: u32, collector: &mut C)
        where
            C: OutputCollector<u32>,
        {
            collector.push(data);
            collector.push(data);
        }
    }

    let mut engine = StageEngine::<u32, u32>::new().add_stage(Duplicate);

    engine.send(5);
    assert_eq!(engine.receive(), Some(5));
    assert_eq!(engine.receive(), Some(5));
}

#[test]
fn test_load_moderate() {
    let count = 1000;
    let mut engine =
        StageEngine::<u32, u32>::with_capacity(count + 1).add_stage(|x: u32| Some(x + 1));

    for i in 0..count {
        engine.send(i as u32);
    }

    for i in 0..count {
        assert_eq!(engine.receive(), Some(i as u32 + 1));
    }
}

#[test]
fn test_concurrency_stress() {
    let mut engine = StageEngine::<u32, u32>::new()
        .add_stage(|x: u32| {
            // Some artificial delay to force concurrency
            thread::sleep(Duration::from_millis(1));
            Some(x)
        })
        .add_stage(|x: u32| {
            thread::sleep(Duration::from_millis(1));
            Some(x)
        });

    let count = 100;
    for i in 0..count {
        engine.send(i);
    }

    for i in 0..count {
        assert_eq!(engine.receive(), Some(i));
    }
}

#[test]
fn test_complex_pipe_macro() {
    let mut engine = StageEngine::<u32, u32>::new().add_stage(pipe![
        |x: u32| Some(x as u64),
        |x: u64| Some(x * 10),
        |x: u64| Some(x + 5),
    ]);

    engine.send(1);
    assert_eq!(engine.receive(), Some(15));
}

#[test]
fn test_empty_pipeline() {
    let mut engine = StageEngine::<u32, u32>::new();
    engine.send(42);
    assert_eq!(engine.receive(), Some(42));
}

#[test]
fn test_await_idle() {
    let mut engine = StageEngine::<u32, u32>::new().add_stage(|x: u32| {
        // Very short sleep to test await_idle without being too slow
        thread::sleep(Duration::from_millis(1));
        Some(x)
    });

    engine.send(1);
    // Give it a tiny bit of time to start
    thread::sleep(Duration::from_millis(5));
    engine.await_idle(Duration::from_millis(200));
    assert_eq!(engine.output_size(), 1);
    assert_eq!(engine.receive(), Some(1));
}

#[test]
fn test_large_pod_struct() {
    #[repr(C)]
    #[derive(Debug, Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, PartialEq)]
    struct Large {
        data: [f64; 16],
        id: u64,
    }

    let mut engine = StageEngine::<Large, Large>::new().add_stage(|mut l: Large| {
        l.id += 1;
        Some(l)
    });

    let input = Large {
        data: [1.0; 16],
        id: 100,
    };
    engine.send(input);

    let expected = Large {
        data: [1.0; 16],
        id: 101,
    };
    assert_eq!(engine.receive(), Some(expected));
}

#[test]
fn test_nested_pipes() {
    let mut engine = StageEngine::<u32, u32>::new().add_stage(pipe![
        |x: u32| Some(x + 1),
        pipe![|x: u32| Some(x * 2), |x: u32| Some(x + 1),]
    ]);

    engine.send(10);
    // (10 + 1) * 2 + 1 = 23
    assert_eq!(engine.receive(), Some(23));
}

#[test]
fn test_multi_stage_load() {
    let stages = 5;
    let items = 100;

    let mut engine = StageEngine::<u32, u32>::new();
    for _ in 0..stages {
        engine = engine.add_stage(|x: u32| Some(x + 1));
    }

    for i in 0..items {
        engine.send(i);
    }

    for i in 0..items {
        assert_eq!(engine.receive(), Some(i + stages as u32));
    }
}

#[test]
#[should_panic(expected = "Store is full")]
fn test_input_capacity_limit_panic() {
    let mut engine = StageEngine::<u32, u32>::with_capacity(1);
    engine.send(1);
    engine.send(2); // Should panic here
}

#[test]
fn test_stage_producing_none() {
    let mut engine = StageEngine::<u32, u32>::new()
        .add_stage(|x: u32| if x > 10 { Some(x) } else { None })
        .add_stage(|x: u32| Some(x * 2));

    engine.send(5);
    engine.send(15);

    engine.await_idle(Duration::from_millis(100));
    assert_eq!(engine.output_size(), 1);
    assert_eq!(engine.receive(), Some(30));
}

#[test]
fn test_worker_panic_on_drop() {
    // This test ensures that if a worker panics, the engine will panic on drop.
    let result = std::panic::catch_unwind(|| {
        let mut engine = StageEngine::<u32, u32>::new().add_stage(|_| {
            panic!("Stage panic");
            #[allow(unreachable_code)]
            Some(0u32)
        });
        engine.send(1);
        // Wait for worker to panic
        thread::sleep(Duration::from_millis(50));
        // engine is dropped here
    });
    assert!(result.is_err());
}

#[test]
fn test_long_pipeline_heavy_load() {
    let stages = 10;
    let items = 5000;

    let mut engine = StageEngine::<u32, u32>::with_capacity(items + 1);
    for _ in 0..stages {
        engine = engine.add_stage(|x: u32| Some(x + 1));
    }

    for i in 0..items {
        engine.send(i as u32);
    }

    for i in 0..items {
        assert_eq!(engine.receive(), Some(i as u32 + stages as u32));
    }
}
