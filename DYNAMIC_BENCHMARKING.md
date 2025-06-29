# Dynamic CPU Benchmarking for Bitcoin Puzzle Hunter

This document explains how to use the dynamic CPU benchmarking feature to automatically calculate `hashes_per_second` for optimal worker distribution.

## Quick Start

### 1. Benchmark Your Machine
```bash
# Quick benchmark (10 seconds, auto-detect cores)
cargo run --release -- --benchmark

# Custom benchmark (5 seconds, 4 threads)  
cargo run --release -- --benchmark --benchmark-duration 5 --threads 4
```

### 2. See Dynamic Demo
```bash
# Demo with auto-benchmarking of current machine
cargo run --release -- --demo-workers
```

### 3. Run Examples
```bash
# Comprehensive auto-benchmark example
cargo run --example auto_benchmark_example

# Original static example
cargo run --example worker_distribution_example
```

## API Usage

### Basic Benchmarking
```rust
use btc_puzzle_hunter::benchmark_hashes_per_second;

// Benchmark for 10 seconds with auto CPU detection
let performance = benchmark_hashes_per_second(10, None)?;
println!("Performance: {} keys/second", performance);
```

### Auto-Worker Creation
```rust
use btc_puzzle_hunter::create_auto_worker;

// Create worker with auto-detected performance
let worker = create_auto_worker(
    "My Machine".to_string(),
    Some(5),    // 5-second benchmark
    None,       // Auto-detect CPU cores
)?;
```

### Multi-Machine Workflow
```rust
use btc_puzzle_hunter::{Worker, distribute_range_to_workers};

// Step 1: Run benchmark on each machine
let machine1_perf = benchmark_hashes_per_second(10, None)?;
let machine2_perf = benchmark_hashes_per_second(10, Some(4))?; // 4 threads

// Step 2: Create worker list
let workers = vec![
    Worker { name: "Server-1".to_string(), hashes_per_second: machine1_perf },
    Worker { name: "Server-2".to_string(), hashes_per_second: machine2_perf },
];

// Step 3: Distribute work
let (ranges, remaining) = distribute_range_to_workers(
    workers, &start_range, &end_range, 10.0
)?;
```

## Benchmark Results

The benchmark function tests actual Bitcoin key generation and address derivation performance, including:
- Private key generation from BigUint
- Elliptic curve operations (secp256k1)
- Address generation (P2PKH, P2SH, Bech32)
- Multi-threading coordination

### Sample Performance Data
- **High-end CPU (16 cores)**: ~500k-2M keys/sec
- **Gaming PC (8 cores)**: ~200k-500k keys/sec  
- **Laptop (4 cores)**: ~50k-200k keys/sec
- **Raspberry Pi**: ~1k-10k keys/sec

## Command Line Options

```bash
# Benchmark commands
--benchmark                    # Run CPU benchmark
--benchmark-duration SECONDS  # Benchmark duration (default: 10)
--threads NUM                  # Number of threads (0 = auto-detect)

# Algorithm commands
--random-walk                  # Use random walk with auto-calculated parameters

# Distribution commands  
--demo-workers                 # Demo with auto-benchmarking
```

## Auto-Calculated Random Walk Parameters

When using `--random-walk`, the application automatically:

1. **Benchmarks your machine** (3-second test) to determine performance
2. **Calculates optimal parameters** based on your `hashes_per_second`:
   - **Walk iterations**: How many keys each walk tests
   - **Walk count**: Number of parallel walks
   - **Adaptation interval**: How often to adjust search strategy
3. **Displays the calculated parameters** before starting the search

### Parameter Scaling Examples:
```
Machine Type              Keys/Sec     Iterations  Walks  Runtime
Raspberry Pi              1,000        60,000      2      120s
Gaming PC                 100,000      1,500,000   6      90s  
GPU Mining Rig            2,000,000    10,000,000  20     100s
```

The algorithm automatically balances:
- **Exploration vs exploitation**: Longer walks for thoroughness vs more walks for coverage
- **Resource utilization**: More parallel walks on faster machines
- **Runtime efficiency**: Targets reasonable execution times (1-2 minutes)

## Implementation Details

### Benchmark Process
1. **Multi-threaded execution**: Uses all CPU cores by default
2. **Real workload simulation**: Actual key generation + address derivation
3. **Accurate timing**: Measures wall-clock time for realistic performance
4. **Thread-safe counting**: Atomic counters for precise key counting
5. **Configurable duration**: Balance between accuracy and speed

### Performance Factors
- **CPU architecture**: Modern CPUs with hardware crypto acceleration
- **Memory bandwidth**: BigUint operations are memory-intensive
- **Thread count**: Optimal thread count may differ from CPU core count
- **System load**: Other processes can affect benchmark results

### Auto-Distribution Benefits
- **Fair work allocation**: Each machine gets work proportional to its capability
- **Time synchronization**: All workers finish approximately simultaneously  
- **No manual tuning**: Eliminates guesswork in performance estimation
- **Real-world accuracy**: Benchmarks actual application workload

## Best Practices

### 1. Benchmark Each Machine Separately
```bash
# On each worker machine:
ssh server1 "cd btc_puzzle_hunter && cargo run --release -- --benchmark"
ssh server2 "cd btc_puzzle_hunter && cargo run --release -- --benchmark" 
```

### 2. Use Longer Benchmarks for Accuracy
```bash
# More accurate results with longer benchmarks
cargo run --release -- --benchmark --benchmark-duration 30
```

### 3. Test Different Thread Counts
```bash
# Find optimal thread count
cargo run --release -- --benchmark --threads 4
cargo run --release -- --benchmark --threads 8
cargo run --release -- --benchmark --threads 16
```

### 4. Save Worker Configurations
```rust
// Save benchmark results for reuse
let workers = vec![
    Worker { name: "Server-1".to_string(), hashes_per_second: 245_000 },
    Worker { name: "Server-2".to_string(), hashes_per_second: 180_000 },
    Worker { name: "GPU-Rig".to_string(), hashes_per_second: 850_000 },
];

save_worker_distribution_to_file(&ranges, &remaining, "production_workers.json")?;
```

## Integration with Existing Code

The benchmarking functionality is fully compatible with the existing Bitcoin Puzzle Hunter. Once you have the `hashes_per_second` values, use them exactly as before:

```bash
# Each worker runs their assigned range
cargo run --release -- --start 0x8000000000 --end 0x8023c345ff --targets "1BvBMSEYstWetqTFn5Au4m4GFg7xJaNVN2"
```

The dynamic benchmarking simply provides more accurate performance values than manual estimation.