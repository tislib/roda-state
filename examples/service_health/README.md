# Service Health Monitoring Pipeline

This example demonstrates a robust, low-latency service health monitoring system built with the **Roda Engine**. It includes noise filtering (deduplication), stateful aggregation, and anomaly detection with alert deduplication.

## Key Features

- **Noise Filtering**: Uses the `dedup_by` pipe component to drop redundant raw readings with identical values, reducing downstream load.
- **Hierarchical Pipeline**: Combines multiple processing steps (dedup -> stateful -> inspect) into logical stages.
- **Intelligent Alerting**: 
  - Detects spikes in average values using `delta`.
  - Suppresses duplicate alerts for the same sensor using `dedup_by`, ensuring the monitoring system only notifies on state changes.
- **Performance Observability**:
  - Uses the `latency` pipe to monitor the execution time of each composite stage.
  - Reports end-to-end "Tick-to-Alert" latency for detected anomalies.

## Pipeline Architecture

```mermaid
graph LR
    A[Raw Reading] --> B(Stage 1: Aggregation & Filtering)
    B -->|Summary| C(Stage 2: Alerting & Suppression)
    C -->|Alert| D[Main Thread / Dashboard]

    subgraph "Stage 1 (Pinned Thread)"
    B1[Deduplicator] --> B2[Stateful Aggregator]
    end

    subgraph "Stage 2 (Pinned Thread)"
    C1[Delta Detector] --> C2[Alert Dedup]
    end
```

## Data Models

1.  **Reading**: Raw metric from a service/sensor.
2.  **Summary**: Rolling window of metrics (min, max, avg).
3.  **Alert**: Notifies on significant health degradation (>50% jump in average).

## Usage

```bash
# Run the example with optimizations
cargo run --release --example service_health
```

## Performance Metrics

- **Throughput**: ~4.5 MEPS (due to additional deduplication steps).
- **Stage Execution**: ~70-100ns per record.
- **End-to-End Latency**: Measured in nanoseconds from ingestion to alert receipt.
