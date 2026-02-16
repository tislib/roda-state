# Real-Time Sensor Data Aggregation & Anomaly Detection

This example demonstrates a high-performance multistage pipeline for processing streaming sensor data using the **Roda Engine**. It showcases statistical windowing (Aggregation) and stateful delta analysis (Anomaly Detection) in a thread-per-stage architecture.

## Key Features

- **Multistage Pipeline**: Decouples data ingestion, statistical aggregation, and anomaly detection into separate CPU-bound stages.
- **Stateful Windowing**: Maintains running statistics (min, max, average) for sensors using the `stateful` pipe component.
- **Low-Latency Alerting**: Detects anomalies (e.g., sudden spikes in average value) using the `delta` component to compare current window state with the previous one.
- **Performance Metrics**: 
  - **Execution Latency**: Measures time spent within each stage using the `latency` pipe component.
  - **End-to-End Latency**: Tracks "Tick-to-Alert" latency from raw reading to signal generation.
  - **Throughput**: Capable of processing millions of sensor readings per second.

## Pipeline Architecture

```mermaid
graph LR
    A[Raw Reading] --> B(Stage 1: Aggregation)
    B -->|Summary| C(Stage 2: Anomaly Detection)
    C -->|Alert| D[Alert Journal]

    subgraph "Worker Thread 1"
    B
    end
    subgraph "Worker Thread 2"
    C
    end
```

## Data Models

1.  **Reading**: Raw sensor data with `sensor_id`, `value`, and receive timestamp.
2.  **Summary**: Statistical window containing min, max, average, and observation count.
3.  **Alert**: Signal generated when a sensor's average value jumps by more than 50% compared to the previous window.

## Usage

```bash
# Run the example with optimizations
cargo run --release --example sensor_test
```

## Performance

On a modern CPU, this example typically achieves:
- **Throughput**: > 5 MEPS (Million Events Per Second).
- **End-to-End Latency**: < 500ns (median) for alert generation.
- **Stage Latency**: ~50ns per record for aggregation logic.
