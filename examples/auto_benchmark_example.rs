use btc_puzzle_hunter::{
    create_auto_worker, distribute_range_to_workers, print_worker_distribution, 
    save_worker_distribution_to_file, benchmark_hashes_per_second
};
use num_bigint::BigUint;
use anyhow::Result;

fn main() -> Result<()> {
    println!("🚀 Auto-Benchmark Worker Distribution Example\n");

    // Method 1: Quick benchmark of current machine
    println!("=== Method 1: Quick Auto-Worker Creation ===");
    let auto_worker = create_auto_worker(
        "Auto-Detected Machine".to_string(),
        Some(5), // 5-second benchmark
        None,    // Auto-detect CPU cores
    )?;
    
    println!("Created worker: {} with {} keys/sec\n", 
             auto_worker.name, auto_worker.hashes_per_second);

    // Method 2: Manual benchmark with custom settings
    println!("=== Method 2: Custom Benchmark Settings ===");
    let manual_performance = benchmark_hashes_per_second(
        3,        // 3 seconds
        Some(4),  // Use 4 threads specifically
    )?;
    
    println!("Manual benchmark result: {} keys/sec\n", manual_performance);

    // Method 3: Simulate multiple machines with auto-detection
    println!("=== Method 3: Multi-Machine Distribution ===");
    
    // In real scenario, you'd run this on each machine separately
    // For demo, we'll use the current machine as multiple "workers"
    let workers = vec![
        auto_worker.clone(),
        btc_puzzle_hunter::Worker {
            name: "Server-A (Simulated)".to_string(),
            hashes_per_second: auto_worker.hashes_per_second / 2, // Simulate slower machine
        },
        btc_puzzle_hunter::Worker {
            name: "Server-B (Simulated)".to_string(),  
            hashes_per_second: auto_worker.hashes_per_second * 2, // Simulate faster machine
        },
    ];

    // Bitcoin puzzle range (puzzle 40 range as example)
    let start = BigUint::parse_bytes(b"8000000000", 16).unwrap();
    let end = BigUint::parse_bytes(b"ffffffffff", 16).unwrap();

    println!("Distributing range 0x{:x} to 0x{:x}", start, end);
    println!("Target time: 10 minutes per worker\n");

    // Distribute work based on actual performance
    let (worker_ranges, remaining) = distribute_range_to_workers(
        workers,
        &start,
        &end,
        10.0, // 10 minutes target
    )?;

    // Show results
    print_worker_distribution(&worker_ranges, &remaining);

    // Save configuration
    save_worker_distribution_to_file(&worker_ranges, &remaining, "auto_benchmark_distribution.json")?;

    println!("\n🎯 Next Steps:");
    println!("1. Run '--benchmark' on each machine to get their performance");
    println!("2. Create Worker structs with the results");
    println!("3. Use distribute_range_to_workers() to assign work");
    println!("4. Each machine runs: cargo run --release -- --start 0x<start> --end 0x<end> --targets <address>");

    println!("\n📊 Performance Comparison:");
    for range in &worker_ranges {
        let efficiency = range.range_size as f64 / range.estimated_time_minutes;
        println!("  {}: {:.0} keys/minute efficiency", 
                 range.worker_name, efficiency);
    }

    Ok(())
}