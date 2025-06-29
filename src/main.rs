use anyhow::Result;
use clap::{Arg, Command};
use indicatif::{ProgressBar, ProgressStyle};
use log::error;
use num_bigint::BigUint;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::Instant;

mod bitcoin_utils;
mod puzzle_data;
mod random_walk;
use bitcoin_utils::{parse_hex_key, private_key_to_addresses};
use puzzle_data::{get_easiest_puzzles, get_puzzle_by_number, list_available_puzzles};
use random_walk::parallel_adaptive_random_walk_search_with_progress;

use btc_puzzle_hunter::{
    Worker, benchmark_hashes_per_second, distribute_range_to_workers, print_worker_distribution,
    save_worker_distribution_to_file, calculate_and_display_walk_parameters,
};

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
}

pub fn demo_worker_distribution() -> Result<()> {
    println!("=== Bitcoin Puzzle Worker Distribution Demo ===\n");

    println!("🔧 This demo will benchmark your current machine and create example workers...");

    // Benchmark current machine
    let current_machine_perf = benchmark_hashes_per_second(3, None)?; // Quick 3-second benchmark

    let hostname = std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "Current Machine".to_string());

    // Create workers list with current machine + example machines
    let workers = vec![
        Worker {
            name: hostname,
            hashes_per_second: current_machine_perf,
        },
        Worker {
            name: "High-End GPU Rig (Example)".to_string(),
            hashes_per_second: 1_000_000, // 1M keys/sec
        },
        Worker {
            name: "Gaming PC (Example)".to_string(),
            hashes_per_second: 100_000, // 100k keys/sec
        },
        Worker {
            name: "Old Laptop (Example)".to_string(),
            hashes_per_second: 10_000, // 10k keys/sec
        },
    ];

    // Range from 0x8000000000 to 0xffffffffff (example range)
    let start = BigUint::parse_bytes(b"8000000000", 16).unwrap();
    let end = BigUint::parse_bytes(b"ffffffffff", 16).unwrap();

    println!("Range: 0x{:x} to 0x{:x}", start, end);
    println!("Target time per worker: 10 minutes\n");

    // Distribute the range
    let (worker_ranges, remaining) = distribute_range_to_workers(workers, &start, &end, 10.0)?;

    // Print the distribution
    print_worker_distribution(&worker_ranges, &remaining);

    // Save to JSON file
    save_worker_distribution_to_file(&worker_ranges, &remaining, "worker_distribution.json")?;

    Ok(())
}

fn main() -> Result<()> {
    env_logger::init();

    let matches = Command::new("Bitcoin Puzzle Hunter")
        .version("1.0")
        .about("High-performance Bitcoin private key searcher")
        .arg(
            Arg::new("start")
                .long("start")
                .value_name("HEX")
                .help("Start of private key range (hex)"),
        )
        .arg(
            Arg::new("end")
                .long("end")
                .value_name("HEX")
                .help("End of private key range (hex)"),
        )
        .arg(
            Arg::new("targets")
                .long("targets")
                .value_name("ADDRESSES")
                .help("Target Bitcoin addresses (comma-separated)"),
        )
        .arg(
            Arg::new("puzzle")
                .long("puzzle")
                .short('p')
                .value_name("NUMBER")
                .help("Bitcoin puzzle number to solve (loads range/target automatically)"),
        )
        .arg(
            Arg::new("list")
                .long("list")
                .short('l')
                .help("List all available unsolved puzzles")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("easy")
                .long("easy")
                .value_name("COUNT")
                .help("Show the N easiest unsolved puzzles")
                .default_value("5"),
        )
        .arg(
            Arg::new("threads")
                .long("threads")
                .short('t')
                .value_name("NUM")
                .help("Number of threads to use")
                .default_value("0"),
        )
        .arg(
            Arg::new("batch-size")
                .long("batch-size")
                .short('b')
                .value_name("SIZE")
                .help("Batch size for each thread")
                .default_value("1000000"),
        )
        .arg(
            Arg::new("random-walk")
                .long("random-walk")
                .short('r')
                .help("Use Random Walk algorithm instead of sequential search")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("demo-workers")
                .long("demo-workers")
                .help("Demo worker range distribution functionality")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("benchmark")
                .long("benchmark")
                .help("Benchmark current machine's hashing performance")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("benchmark-duration")
                .long("benchmark-duration")
                .value_name("SECONDS")
                .help("Duration for benchmark test in seconds")
                .default_value("10"),
        )
        .get_matches();

    // Handle benchmark command
    if matches.get_flag("benchmark") {
        let duration: u64 = matches
            .get_one::<String>("benchmark-duration")
            .unwrap()
            .parse()?;
        let threads: usize = matches.get_one::<String>("threads").unwrap().parse()?;
        let threads_opt = if threads == 0 { None } else { Some(threads) };

        println!("🚀 Starting CPU benchmark...");
        let performance = benchmark_hashes_per_second(duration, threads_opt)?;
        println!("\n📋 Benchmark Results Summary:");
        println!("   Your machine: {} keys/second", performance);
        println!(
            "   Estimated range for 10 minutes: {} keys",
            performance * 600
        );

        // Create worker definition for this machine
        let hostname = std::env::var("HOSTNAME")
            .or_else(|_| std::env::var("COMPUTERNAME"))
            .unwrap_or_else(|_| "localhost".to_string());

        let _worker = Worker {
            name: hostname.clone(),
            hashes_per_second: performance,
        };

        println!("\n💾 Worker configuration for this machine:");
        println!("Worker {{");
        println!("    name: \"{}\".to_string(),", hostname);
        println!("    hashes_per_second: {},", performance);
        println!("}}");

        return Ok(());
    }

    // Handle demo command
    if matches.get_flag("demo-workers") {
        demo_worker_distribution()?;
        return Ok(());
    }

    // Handle list command
    if matches.get_flag("list") {
        list_available_puzzles()?;
        return Ok(());
    }

    // Handle easy puzzles command - only if no other specific command and no manual range
    if !matches.contains_id("puzzle")
        && !matches.get_flag("list")
        && !matches.contains_id("start")
        && !matches.contains_id("end")
        && !matches.contains_id("targets")
    {
        if let Some(easy_count_str) = matches.get_one::<String>("easy") {
            if let Ok(count) = easy_count_str.parse::<usize>() {
                let easy_puzzles = get_easiest_puzzles(count)?;
                println!("Top {} easiest unsolved puzzles:", count);
                for puzzle in easy_puzzles {
                    println!(
                        "Puzzle #{}: {} bits, {} BTC reward",
                        puzzle.puzzle, puzzle.bits, puzzle.reward_btc
                    );
                    println!("  Range: {} to {}", puzzle.range_start, puzzle.range_end);
                    println!("  Address: {}", puzzle.address);
                    println!();
                }
                return Ok(());
            }
        }
    }

    let (start_hex, end_hex, targets_str) =
        if let Some(puzzle_num_str) = matches.get_one::<String>("puzzle") {
            let puzzle_num: u32 = puzzle_num_str.parse()?;
            if let Some(puzzle) = get_puzzle_by_number(puzzle_num)? {
                println!(
                    "Loading puzzle #{}: {} bits, {} BTC reward",
                    puzzle.puzzle, puzzle.bits, puzzle.reward_btc
                );
                (puzzle.range_start, puzzle.range_end, puzzle.address)
            } else {
                return Err(anyhow::anyhow!(
                    "Puzzle #{} not found in unsolved puzzles",
                    puzzle_num
                ));
            }
        } else {
            // Manual mode - require start, end, targets
            let start = matches
                .get_one::<String>("start")
                .ok_or_else(|| anyhow::anyhow!("--start is required when not using --puzzle"))?;
            let end = matches
                .get_one::<String>("end")
                .ok_or_else(|| anyhow::anyhow!("--end is required when not using --puzzle"))?;
            let targets = matches
                .get_one::<String>("targets")
                .ok_or_else(|| anyhow::anyhow!("--targets is required when not using --puzzle"))?;
            (start.clone(), end.clone(), targets.clone())
        };
    let threads: usize = matches.get_one::<String>("threads").unwrap().parse()?;
    let batch_size: u64 = matches.get_one::<String>("batch-size").unwrap().parse()?;
    let use_random_walk = matches.get_flag("random-walk");

    let start_key = parse_hex_key(&start_hex)?;
    let end_key = parse_hex_key(&end_hex)?;

    let target_addresses: HashSet<String> = if targets_str.contains(',') {
        targets_str
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    } else {
        [targets_str].into_iter().collect()
    };

    println!("Starting Bitcoin puzzle hunter");
    println!("Range: {} to {}", start_hex, end_hex);
    println!("Target addresses: {:?}", target_addresses);
    println!(
        "Algorithm: {}",
        if use_random_walk {
            "Random Walk"
        } else {
            "Sequential Search"
        }
    );
    if use_random_walk {
        println!("Parameters will be auto-calculated based on machine performance");
    } else {
        println!(
            "Threads: {}",
            if threads == 0 {
                rayon::current_num_threads()
            } else {
                threads
            }
        );
        println!("Batch size: {}", batch_size);
    }

    if threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap();
    }

    let found = Arc::new(AtomicBool::new(false));
    let keys_checked = Arc::new(AtomicU64::new(0));

    // Create progress bar based on algorithm type
    let total_keys = &end_key - &start_key + 1u32;
    let progress_bar = if use_random_walk {
        // For random walk, we use an indeterminate progress bar since parameters are calculated dynamically
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::default_spinner()
                .template("{spinner:.green} [{elapsed_precise}] {pos} keys tested ({per_sec})")
                .unwrap(),
        );
        pb
    } else {
        let pb = ProgressBar::new(total_keys.try_into().unwrap_or(u64::MAX));
        pb.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {per_sec}")
                .unwrap()
                .progress_chars("#>-"),
        );
        pb
    };

    let start_time = Instant::now();

    let result = if use_random_walk {
        println!("Using Adaptive Random Walk algorithm...");
        
        // Auto-benchmark machine to determine optimal parameters
        println!("🔧 Auto-detecting optimal random walk parameters...");
        let threads_for_benchmark = if threads == 0 { None } else { Some(threads) };
        let machine_performance = benchmark_hashes_per_second(3, threads_for_benchmark)?; // Quick 3-second benchmark
        
        // Calculate optimal parameters based on machine performance
        let (walk_iterations, walk_count, adapt_interval) = 
            calculate_and_display_walk_parameters(machine_performance);
        
        parallel_adaptive_random_walk_search_with_progress(
            &start_key,
            &end_key,
            &target_addresses,
            walk_iterations,
            walk_count,
            adapt_interval,
            Some(progress_bar.clone()),
            Some(keys_checked.clone()),
        )
    } else {
        search_range(
            start_key,
            end_key,
            target_addresses,
            batch_size,
            found.clone(),
            keys_checked.clone(),
            progress_bar.clone(),
        )
    };

    progress_bar.finish();

    let elapsed = start_time.elapsed();
    println!("Search completed in {:?}", elapsed);

    let total_checked = keys_checked.load(Ordering::Relaxed);
    let keys_per_sec = total_checked as f64 / elapsed.as_secs_f64();
    println!("Total keys checked: {}", total_checked);
    println!("Keys per second: {:.2}", keys_per_sec);

    if use_random_walk {
        println!("Adaptive Random Walk algorithm completed with auto-calculated parameters");
    }

    match result {
        Ok(Some((private_key, address))) => {
            println!("🎉 MATCH FOUND!");
            println!("Private Key: {}", private_key);
            println!("Address: {}", address);
        }
        Ok(None) => {
            println!("No match found in the specified range.");
        }
        Err(e) => {
            error!("Error during search: {}", e);
        }
    }

    Ok(())
}

fn search_range(
    start: BigUint,
    end: BigUint,
    targets: HashSet<String>,
    batch_size: u64,
    found: Arc<AtomicBool>,
    keys_checked: Arc<AtomicU64>,
    progress_bar: ProgressBar,
) -> Result<Option<(String, String)>> {
    let range_size = &end - &start + 1u32;
    let batch_count = (range_size.clone() / batch_size + 1u32)
        .try_into()
        .unwrap_or(usize::MAX);

    let result = (0..batch_count)
        .into_par_iter()
        .map(|batch_idx| {
            if found.load(Ordering::Relaxed) {
                return Ok(None);
            }

            let batch_start = &start + (batch_idx as u64) * batch_size;
            let batch_end = std::cmp::min(batch_start.clone() + batch_size - 1u32, end.clone());

            search_batch(
                batch_start,
                batch_end,
                &targets,
                found.clone(),
                keys_checked.clone(),
                progress_bar.clone(),
            )
        })
        .find_map_first(|result| match result {
            Ok(Some(found_key)) => {
                found.store(true, Ordering::Relaxed);
                Some(Ok(Some(found_key)))
            }
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        });

    match result {
        Some(r) => r,
        None => Ok(None),
    }
}

fn search_batch(
    start: BigUint,
    end: BigUint,
    targets: &HashSet<String>,
    found: Arc<AtomicBool>,
    keys_checked: Arc<AtomicU64>,
    progress_bar: ProgressBar,
) -> Result<Option<(String, String)>> {
    let mut current = start;
    let mut local_count = 0u64;

    while current <= end && !found.load(Ordering::Relaxed) {
        if let Ok(addresses) = private_key_to_addresses(&current) {
            for address in addresses {
                if targets.contains(&address) {
                    let hex_key = format!("{:064x}", current);
                    return Ok(Some((hex_key, address)));
                }
            }
        }

        current += 1u32;
        local_count += 1;

        if local_count % 10000 == 0 {
            keys_checked.fetch_add(local_count, Ordering::Relaxed);
            progress_bar.inc(local_count);
            local_count = 0;
        }
    }

    if local_count > 0 {
        keys_checked.fetch_add(local_count, Ordering::Relaxed);
        progress_bar.inc(local_count);
    }

    Ok(None)
}
