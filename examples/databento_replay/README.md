# Liquidity Monitor

This example demonstrates a market data replay system using the Roda engine. It processes raw Market-By-Order (MBO) data to perform real-time liquidity analysis.

## Overview

The "Liquidity Monitor" goes beyond simple price tracking. It focuses on three main objectives:

### 1. Reconstruct the Aggregate Book (Level 2)
Convert the raw stream of individual orders (MBO) into a consolidated map of **Price → Total Volume**.
*   **Why useful?** This is what exchanges actually sell as "Level 2 Data." You are building it from scratch from the most granular data available.

### 2. Calculate "Order Book Imbalance"
Measure the ratio of buy vs. sell pressure in the book.

**Formula:**
$$Imbalance = \frac{Bid\ Vol - Ask\ Vol}{Bid\ Vol + Ask\ Vol}$$

*   **Why useful?** This is a primary signal for predicting short-term price movement. A positive value indicates buy pressure.

### 3. Detect "Liquidity Voids"
Monitor the book for sudden drops in available volume.
*   **Condition:** If the total volume at the Top 5 levels drops by 50% in < 1ms, trigger an alert.
*   **Why useful?** This predicts "Flash Crashes" and high-volatility events where price might slip significantly.

## Usage

To run the replay, provide the path to a Databento MBO file:

```bash
cargo run --example databento_replay -- --file path/to/your/data.dbn
```

## Architecture

- `main.rs`: Sets up the Roda engine, market data store, and the processing pipeline.
- `importer.rs`: Handles reading and decoding the Databento MBO file.
- `light_mbo_entry.rs`: Defines the compact data structure for storing MBO records in the Roda store.
