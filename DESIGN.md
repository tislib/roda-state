# Roda: Architecture & Design Specification

## 1. Core Philosophy: "The Latency Is The Product"

Roda is built for ultra-high-performance streaming applications—trading systems, real-time analytics, and
telemetry—where deterministic performance is paramount. It adheres to **Mechanical Sympathy**, aligning software design
with hardware realities.

1. **Deterministic Latency:** Every operation has a bounded execution time. We prefer O(1) algorithms over O(log n). No
   memory is allocated on the hot path.
2. **Predictable Cycles:** A "Unit of Work" is constant. Processing $N$ events scales linearly with $N$ in terms of CPU
   cycles.
3. **Explicit Control:** The developer defines the memory bounds and data flow. Roda provides the primitives (Stores,
   Indexes), but the developer orchestrates how they are processed.
4. **Zero-Copy by Default:** Data is not moved; ownership is not transferred. Readers get a **View** (borrowed
   reference) into shared memory regions.
5. **Lock-Free Concurrency:** No `Mutex`, `RwLock`, or condition variables on the data path. Synchronization is achieved
   via **Atomic Sequence Counters** (Acquire/Release semantics).

---

## 2. System Architecture

The system follows a **Shared-Nothing** architecture for logic (workers don't share state directly), but a *
*Shared-Memory** architecture for data.

### 2.1 The Engine (Orchestrator)

The `RodaEngine` acts as the system's bootloader, managing resources and thread lifecycle.

**Core Responsibilities:**

* **Memory Management:** Allocates large, contiguous memory blocks via `mmap` and initializes shared structures (ring buffers, headers).
* **Thread Orchestration:** Spawns long-lived worker threads, optionally pinning them to specific CPU cores (`isolcpus`) for deterministic execution.

**Worker Execution Model:**
Workers execute user pipelines in a continuous loop using an **Adaptive Backoff Strategy** to balance latency and efficiency:

1. **Busy Spin (Hot Path):** Continuously polls cursors for nanosecond-level response times.
2. **CPU Relax (Warm Path):** After empty cycles, emits `PAUSE` instructions (`std::hint::spin_loop`) to reduce power usage.
3. **Park/Sleep (Cold Path):** After extended inactivity, yields the thread to the OS scheduler to save resources until new data arrives.

### 2.2 The Store (The Source of Truth)

The `StoreJournal<T>` is a fixed-capacity append-only buffer backed by memory-mapped files.

* **Memory Layout:** `[ Header (Atomics) | Data Region (T...) | Padding ]`.
* **Write Model:** **Single Writer**. Only one thread (the owner of the `Store` handle) can write, eliminating
  write-side contention.
* **Read Model:** **Multiple Readers**. Each reader (or worker) uses an independent `StoreJournalReader<T>` handle
  that maintains its own
  state (cursor).
* **Addressing:** Data is addressed by a monotonic `u64` sequence number (`Cursor`). The physical address is
  `Cursor * sizeof(T)`.
* **Full Buffer Policy:** If the store is full, it will panic on the next `push`. No wrapping or overwriting occurs.

### 2.3 StoreReader & Traits

Roda uses traits to define the behavior of stores and readers, allowing for different implementations (like the default
`StoreJournal`).

* **Store Trait:** Defines `push`, `reader`, and `direct_index`.
* **StoreReader Trait:** Defines `next`, `with`, `with_at`, `with_last`, `get`, `get_at`, `get_last`, and `get_window`.
* **Explicit Advancement:** Each `StoreReader` maintains its own `LocalCursor`.
  The cursor is moved next everytime `next()` is called. So inside a worker for all used store readers `next()` function
  must be
  called.
* **Synchronicity by Design:** Each worker is designed to process a single unit of work in each cycle. Explicit `next()`
  calls give the developer control over when data is consumed relative to other operations (like indexing).
  If there are no more data to read, the cursor will simply stay at the end of the store. No need to handle any special
  case.

---

### 3. The Index (O(1) Access)

The `DirectIndex` is a derivative structure that maps a `Key` to a `Cursor` in a `Store`.

* **Storage:** Also backed by `mmap`.
* **Manual Update:** The index is **not** automatically updated when the store is written. The developer must explicitly
  call the `compute` method (typically inside a worker) to index new data.
* **Consistency:** The developer controls when the index is updated relative to other operations.
* **Safety:** A reader might see data before it is indexed, but will never see an index entry pointing to invalid or
  uninitialized data.

---

## 4. Pipeline Primitives

Roda enables **Declarative Pipelines** by chaining these primitives using a builder pattern:

* **Aggregator:** Maps `Input -> Key -> Output`. Used for partitioned reduction (e.g., Ticks to Candles). State is
  sharded by Key.
    * Pattern: `Aggregator::new().from(&reader).to(&mut store).partition_by(...).reduce(...)`
* **Window:** Maps `Input -> Slice<Input> -> Option<Output>`. Provides a zero-copy "Lookback" mechanism (e.g., Moving
  Averages over the
  last $N$ elements).
    * Pattern: `Window::new().from(&reader).to(&mut store).reduce(size, ...)`
* **Join:** Aligns two independent streams by a common attribute (e.g., Timestamp).

---

## 5. Technical Constraints & Safety

To guarantee performance and zero-copy safety, Roda imposes several constraints:

* **Fixed-Size POD Types:** `T` must be `Copy`, `Sized`, and satisfy `bytemuck` traits. No `String`, `Vec`, or pointers
  allowed inside a `Store`.
* **Memory Pinning:** Uses `mlock` (via `libc`) to prevent shared memory from being swapped to disk.
* **Alignment:** All structures use `#[repr(C)]` and are aligned to machine word boundaries to support zero-copy casting
  and avoid torn reads.

---

## 6. Implementation Notes: The "Magic" of Atomics

Synchronization is achieved without locks using `Acquire/Release` semantics:

* **Writer:** `buffer[cursor] = data; cursor.store(new_val, Release);`
* **Reader:** `while cursor.load(Acquire) > local_cursor { process(); local_cursor++; }`

This ensures that when the reader sees the updated cursor, it is guaranteed to see the data written by the writer.