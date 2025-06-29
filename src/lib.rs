pub mod bitcoin_utils;
pub mod puzzle_data;
pub mod random_walk;

use anyhow::Result;
use num_bigint::BigUint;
use serde::{Deserialize, Serialize};
use std::fs;
use std::time::{Duration, Instant};
use std::thread;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Worker {
    pub name: String,
    pub hashes_per_second: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkerRange {
    pub worker_name: String,
    pub hashes_per_second: u64,
    pub start_hex: String,
    pub end_hex: String,
    pub range_size: u64,
    pub estimated_time_minutes: f64,
}

pub fn distribute_range_to_workers(
    workers: Vec<Worker>,
    range_start: &BigUint,
    range_end: &BigUint,
    target_time_minutes: f64,
) -> Result<(Vec<WorkerRange>, Option<(String, String)>)> {
    if workers.is_empty() {
        return Err(anyhow::anyhow!("No workers provided"));
    }

    let _total_range = range_end - range_start + 1u32;
    let mut current_position = range_start.clone();
    let mut worker_ranges = Vec::new();

    for worker in workers {
        if current_position > *range_end {
            break;
        }

        // Calculate how many keys this worker can process in target time
        let keys_in_target_time = (worker.hashes_per_second as f64 * target_time_minutes * 60.0) as u64;
        let keys_in_target_time_biguint = BigUint::from(keys_in_target_time);

        // Calculate the end position for this worker
        let worker_end = std::cmp::min(
            current_position.clone() + keys_in_target_time_biguint - 1u32,
            range_end.clone(),
        );

        let actual_range_size = &worker_end - &current_position + 1u32;
        let range_size_u64 = if actual_range_size.bits() <= 64 {
            actual_range_size.iter_u64_digits().next().unwrap_or(0)
        } else {
            u64::MAX // Cap at u64::MAX for very large ranges
        };
        let actual_time_minutes = range_size_u64 as f64 / (worker.hashes_per_second as f64 * 60.0);

        let worker_range = WorkerRange {
            worker_name: worker.name,
            hashes_per_second: worker.hashes_per_second,
            start_hex: format!("{:x}", current_position),
            end_hex: format!("{:x}", worker_end),
            range_size: range_size_u64,
            estimated_time_minutes: actual_time_minutes,
        };

        worker_ranges.push(worker_range);

        // Move to next position
        current_position = &worker_end + 1u32;
    }

    // Check if there's any remaining unassigned range
    let remaining_range = if current_position <= *range_end {
        Some((
            format!("{:x}", current_position),
            format!("{:x}", range_end),
        ))
    } else {
        None
    };

    Ok((worker_ranges, remaining_range))
}

pub fn print_worker_distribution(
    worker_ranges: &[WorkerRange],
    remaining_range: &Option<(String, String)>,
) {
    println!("Worker Range Distribution:");
    println!("{:-^80}", "");
    
    for (i, range) in worker_ranges.iter().enumerate() {
        println!("Worker #{}: {}", i + 1, range.worker_name);
        println!("  Hash Rate: {} keys/sec", range.hashes_per_second);
        println!("  Range: 0x{} – 0x{}", range.start_hex, range.end_hex);
        println!("  Range Size: {} keys", range.range_size);
        println!("  Estimated Time: {:.2} minutes", range.estimated_time_minutes);
        println!();
    }

    if let Some((start, end)) = remaining_range {
        println!("⚠️  Unassigned Range:");
        println!("  Range: 0x{} – 0x{}", start, end);
        println!("  Note: No workers available to process this range");
    }
}

pub fn save_worker_distribution_to_file(
    worker_ranges: &[WorkerRange],
    remaining_range: &Option<(String, String)>,
    filename: &str,
) -> Result<()> {
    #[derive(Serialize)]
    struct DistributionReport {
        worker_ranges: Vec<WorkerRange>,
        remaining_range: Option<(String, String)>,
        total_workers: usize,
        total_estimated_time_minutes: f64,
    }

    let total_time = worker_ranges.iter()
        .map(|r| r.estimated_time_minutes)
        .fold(0.0, f64::max); // Max time since workers work in parallel

    let report = DistributionReport {
        worker_ranges: worker_ranges.to_vec(),
        remaining_range: remaining_range.clone(),
        total_workers: worker_ranges.len(),
        total_estimated_time_minutes: total_time,
    };

    let json = serde_json::to_string_pretty(&report)?;
    fs::write(filename, json)?;
    println!("Worker distribution saved to: {}", filename);
    
    Ok(())
}

/// Benchmarks the current machine's key generation and testing speed
pub fn benchmark_hashes_per_second(
    benchmark_duration_seconds: u64,
    num_threads: Option<usize>,
) -> Result<u64> {
    use crate::bitcoin_utils::private_key_to_addresses;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    
    let benchmark_duration = Duration::from_secs(benchmark_duration_seconds);
    let thread_count = num_threads.unwrap_or_else(|| num_cpus::get());
    
    println!("🔧 Benchmarking CPU performance...");
    println!("   Duration: {} seconds", benchmark_duration_seconds);
    println!("   Threads: {}", thread_count);
    println!("   Testing key generation and address derivation speed...");
    
    let total_keys_tested = Arc::new(AtomicU64::new(0));
    let start_time = Instant::now();
    
    // Create thread handles
    let handles: Vec<_> = (0..thread_count)
        .map(|thread_id| {
            let keys_tested = Arc::clone(&total_keys_tested);
            let start_time = start_time;
            
            thread::spawn(move || {
                let mut local_count = 0u64;
                let mut current_key = BigUint::from(0x8000000000u64 + thread_id as u64 * 1000000);
                
                loop {
                    // Check if benchmark time is up
                    if start_time.elapsed() >= benchmark_duration {
                        break;
                    }
                    
                    // Perform actual key testing work (same as the real application)
                    if let Ok(_addresses) = private_key_to_addresses(&current_key) {
                        // In real usage, we'd check addresses against targets
                        // For benchmark, we just count successful key generations
                        local_count += 1;
                    }
                    
                    current_key += 1u32;
                    
                    // Update global counter every 1000 iterations for performance
                    if local_count % 1000 == 0 {
                        keys_tested.fetch_add(1000, Ordering::Relaxed);
                        local_count = 0;
                    }
                }
                
                // Add any remaining count
                if local_count > 0 {
                    keys_tested.fetch_add(local_count, Ordering::Relaxed);
                }
            })
        })
        .collect();
    
    // Wait for all threads to complete
    for handle in handles {
        handle.join().map_err(|_| anyhow::anyhow!("Thread panicked during benchmark"))?;
    }
    
    let actual_duration = start_time.elapsed();
    let total_keys = total_keys_tested.load(Ordering::Relaxed);
    let hashes_per_second = (total_keys as f64 / actual_duration.as_secs_f64()) as u64;
    
    println!("✅ Benchmark completed!");
    println!("   Total keys tested: {}", total_keys);
    println!("   Actual duration: {:.2} seconds", actual_duration.as_secs_f64());
    println!("   Performance: {} keys/second", hashes_per_second);
    
    Ok(hashes_per_second)
}

/// Creates a Worker struct for the current machine with auto-detected performance
pub fn create_auto_worker(
    name: String,
    benchmark_duration_seconds: Option<u64>,
    num_threads: Option<usize>,
) -> Result<Worker> {
    let duration = benchmark_duration_seconds.unwrap_or(5); // Default 5 seconds
    let hashes_per_second = benchmark_hashes_per_second(duration, num_threads)?;
    
    Ok(Worker {
        name,
        hashes_per_second,
    })
}

/// Benchmarks multiple machines and creates a worker list
pub fn benchmark_and_create_workers(
    worker_configs: Vec<(String, Option<u64>, Option<usize>)>, // (name, benchmark_duration, threads)
) -> Result<Vec<Worker>> {
    let mut workers = Vec::new();
    
    for (name, duration, threads) in worker_configs {
        println!("\n📊 Benchmarking worker: {}", name);
        let worker = create_auto_worker(name, duration, threads)?;
        println!("   Result: {} keys/second", worker.hashes_per_second);
        workers.push(worker);
    }
    
    Ok(workers)
}

/// Calculate optimal random walk parameters based on machine performance
pub fn calculate_walk_parameters(hashes_per_second: u64) -> (usize, usize, usize) {
    // Base calculations on performance tiers
    let (walk_iterations, walk_count, adapt_interval) = match hashes_per_second {
        // Very slow machines (< 10k keys/sec) - fewer, longer walks
        0..=10_000 => {
            let iterations = (hashes_per_second as f64 * 60.0) as usize; // 1 minute worth
            let walks = 2.max((hashes_per_second / 5_000) as usize); // 2-4 walks
            let adapt = iterations / 10; // Adapt every 10% of iterations
            (iterations, walks, adapt)
        },
        // Slow machines (10k-50k keys/sec) - moderate settings
        10_001..=50_000 => {
            let iterations = (hashes_per_second as f64 * 30.0) as usize; // 30 seconds worth
            let walks = 4.max((hashes_per_second / 10_000) as usize); // 4-8 walks
            let adapt = iterations / 20; // Adapt every 5% of iterations
            (iterations, walks, adapt)
        },
        // Medium machines (50k-200k keys/sec) - balanced settings
        50_001..=200_000 => {
            let iterations = (hashes_per_second as f64 * 15.0) as usize; // 15 seconds worth
            let walks = 6.max((hashes_per_second / 25_000) as usize); // 6-12 walks
            let adapt = iterations / 30; // Adapt every 3.3% of iterations
            (iterations, walks, adapt)
        },
        // Fast machines (200k-500k keys/sec) - more walks, shorter iterations
        200_001..=500_000 => {
            let iterations = (hashes_per_second as f64 * 10.0) as usize; // 10 seconds worth
            let walks = 8.max((hashes_per_second / 50_000) as usize); // 8-16 walks
            let adapt = iterations / 50; // Adapt every 2% of iterations
            (iterations, walks, adapt)
        },
        // Very fast machines (> 500k keys/sec) - many short walks
        _ => {
            let iterations = (hashes_per_second as f64 * 5.0) as usize; // 5 seconds worth
            let walks = 12.max((hashes_per_second / 100_000) as usize); // 12+ walks
            let adapt = iterations / 100; // Adapt every 1% of iterations
            (iterations, walks, adapt)
        }
    };

    // Ensure minimum values
    let walk_iterations = walk_iterations.max(1000);
    let walk_count = walk_count.max(2).min(32); // Cap at 32 walks
    let adapt_interval = adapt_interval.max(100).min(walk_iterations / 2);

    (walk_iterations, walk_count, adapt_interval)
}

/// Calculate optimal random walk parameters for a machine and display them
pub fn calculate_and_display_walk_parameters(hashes_per_second: u64) -> (usize, usize, usize) {
    let (iterations, walks, adapt) = calculate_walk_parameters(hashes_per_second);
    
    println!("🎯 Auto-calculated random walk parameters:");
    println!("   Iterations per walk: {}", iterations);
    println!("   Number of walks: {}", walks);
    println!("   Adaptation interval: {}", adapt);
    println!("   Total keys to test: {}", iterations * walks);
    println!("   Estimated runtime: {:.1} seconds", 
             (iterations * walks) as f64 / hashes_per_second as f64);
    
    (iterations, walks, adapt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_worker_distribution() {
        let workers = vec![
            Worker {
                name: "Fast Worker".to_string(),
                hashes_per_second: 100_000,
            },
            Worker {
                name: "Medium Worker".to_string(),
                hashes_per_second: 50_000,
            },
            Worker {
                name: "Slow Worker".to_string(),
                hashes_per_second: 25_000,
            },
        ];

        let start = BigUint::parse_bytes(b"8000000000", 16).unwrap();
        let end = BigUint::parse_bytes(b"ffffffffff", 16).unwrap();

        let result = distribute_range_to_workers(workers, &start, &end, 10.0);
        assert!(result.is_ok());

        let (worker_ranges, _remaining) = result.unwrap();
        assert_eq!(worker_ranges.len(), 3);
        
        // First worker should get 60M keys (100k/sec * 10min * 60sec)
        assert_eq!(worker_ranges[0].range_size, 60_000_000);
    }

    #[test]
    fn test_worker_distribution_exact_example() {
        let workers = vec![
            Worker {
                name: "Worker A".to_string(),
                hashes_per_second: 100_000, // 100k keys/sec
            },
        ];

        let start = BigUint::from(1u32);
        let end = BigUint::from(60_000_000u32);

        let result = distribute_range_to_workers(workers, &start, &end, 10.0);
        assert!(result.is_ok());

        let (worker_ranges, remaining) = result.unwrap();
        assert_eq!(worker_ranges.len(), 1);
        assert_eq!(worker_ranges[0].range_size, 60_000_000);
        assert_eq!(worker_ranges[0].estimated_time_minutes, 10.0);
        assert!(remaining.is_none());
    }

    #[test]
    fn test_benchmark_functionality() {
        // Test that benchmark runs without crashing (very short duration)
        let result = benchmark_hashes_per_second(1, Some(1)); // 1 second, 1 thread
        assert!(result.is_ok());
        let performance = result.unwrap();
        assert!(performance > 0);
        assert!(performance < 10_000_000); // Reasonable upper bound
    }

    #[test]
    fn test_create_auto_worker() {
        let result = create_auto_worker(
            "Test Worker".to_string(),
            Some(1), // Very short benchmark
            Some(1), // Single thread
        );
        assert!(result.is_ok());
        let worker = result.unwrap();
        assert_eq!(worker.name, "Test Worker");
        assert!(worker.hashes_per_second > 0);
    }

    #[test]
    fn test_calculate_walk_parameters() {
        // Test different performance tiers
        
        // Very slow machine
        let (iter, walks, adapt) = calculate_walk_parameters(5_000);
        assert!(iter >= 1000);
        assert!(walks >= 2);
        assert!(adapt >= 100);
        
        // Medium machine
        let (iter, walks, adapt) = calculate_walk_parameters(100_000);
        assert!(iter >= 1000);
        assert!(walks >= 4);
        assert!(adapt >= 100);
        
        // Fast machine
        let (iter, walks, adapt) = calculate_walk_parameters(500_000);
        assert!(iter >= 1000);
        assert!(walks >= 8);
        assert!(adapt >= 100);
        assert!(walks <= 32); // Should be capped
        
        // Very fast machine
        let (iter, walks, adapt) = calculate_walk_parameters(2_000_000);
        assert!(iter >= 1000);
        assert!(walks >= 12);
        assert!(adapt >= 100);
        assert!(walks <= 32); // Should be capped
    }
}