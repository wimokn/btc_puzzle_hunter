use btc_puzzle_hunter::{Worker, distribute_range_to_workers, print_worker_distribution, save_worker_distribution_to_file};
use num_bigint::BigUint;
use anyhow::Result;

fn main() -> Result<()> {
    // Example: Your own worker configuration
    let workers = vec![
        Worker {
            name: "Server-1".to_string(),
            hashes_per_second: 500_000, // 500k keys/sec
        },
        Worker {
            name: "Server-2".to_string(),
            hashes_per_second: 200_000, // 200k keys/sec
        },
        Worker {
            name: "GPU-Rig".to_string(),
            hashes_per_second: 2_000_000, // 2M keys/sec
        },
    ];

    // Bitcoin puzzle range (e.g., puzzle 40: 2^39 to 2^40-1)
    let start = BigUint::parse_bytes(b"8000000000", 16).unwrap();
    let end = BigUint::parse_bytes(b"ffffffffff", 16).unwrap();

    println!("Distributing range 0x{:x} to 0x{:x}", start, end);
    println!("Target time: 10 minutes per worker\n");

    // Distribute the work
    let (worker_ranges, remaining) = distribute_range_to_workers(
        workers,
        &start,
        &end,
        10.0, // 10 minutes target
    )?;

    // Print results
    print_worker_distribution(&worker_ranges, &remaining);

    // Save to file
    save_worker_distribution_to_file(&worker_ranges, &remaining, "my_distribution.json")?;

    println!("\nUsage instructions:");
    println!("Each worker should search their assigned range:");
    for (_, range) in worker_ranges.iter().enumerate() {
        println!("  {} → cargo run --release -- --start 0x{} --end 0x{} --targets <address>",
                 range.worker_name, range.start_hex, range.end_hex);
    }

    Ok(())
}