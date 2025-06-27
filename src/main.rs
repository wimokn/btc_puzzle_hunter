use anyhow::Result;
use clap::{Arg, Command};
use indicatif::{ProgressBar, ProgressStyle};
use log::{error, info};
use num_bigint::BigUint;
use rayon::prelude::*;
use std::collections::HashSet;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Instant;

mod bitcoin_utils;
mod puzzle_data;
use bitcoin_utils::{private_key_to_addresses, parse_hex_key};
use puzzle_data::{get_puzzle_by_number, list_available_puzzles, get_easiest_puzzles};

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
        .get_matches();

    // Handle list command
    if matches.get_flag("list") {
        list_available_puzzles()?;
        return Ok(());
    }

    // Handle easy puzzles command - only if no other specific command
    if !matches.contains_id("puzzle") && !matches.get_flag("list") {
        if let Some(easy_count_str) = matches.get_one::<String>("easy") {
            if let Ok(count) = easy_count_str.parse::<usize>() {
                let easy_puzzles = get_easiest_puzzles(count)?;
                println!("Top {} easiest unsolved puzzles:", count);
                for puzzle in easy_puzzles {
                    println!("Puzzle #{}: {} bits, {} BTC reward", 
                        puzzle.puzzle, puzzle.bits, puzzle.reward_btc);
                    println!("  Range: {} to {}", puzzle.range_start, puzzle.range_end);
                    println!("  Address: {}", puzzle.address);
                    println!();
                }
                return Ok(());
            }
        }
    }

    let (start_hex, end_hex, targets_str) = if let Some(puzzle_num_str) = matches.get_one::<String>("puzzle") {
        let puzzle_num: u32 = puzzle_num_str.parse()?;
        if let Some(puzzle) = get_puzzle_by_number(puzzle_num)? {
            info!("Loading puzzle #{}: {} bits, {} BTC reward", puzzle.puzzle, puzzle.bits, puzzle.reward_btc);
            (puzzle.range_start, puzzle.range_end, puzzle.address)
        } else {
            return Err(anyhow::anyhow!("Puzzle #{} not found in unsolved puzzles", puzzle_num));
        }
    } else {
        // Manual mode - require start, end, targets
        let start = matches.get_one::<String>("start")
            .ok_or_else(|| anyhow::anyhow!("--start is required when not using --puzzle"))?;
        let end = matches.get_one::<String>("end")
            .ok_or_else(|| anyhow::anyhow!("--end is required when not using --puzzle"))?;
        let targets = matches.get_one::<String>("targets")
            .ok_or_else(|| anyhow::anyhow!("--targets is required when not using --puzzle"))?;
        (start.clone(), end.clone(), targets.clone())
    };
    let threads: usize = matches.get_one::<String>("threads").unwrap().parse()?;
    let batch_size: u64 = matches.get_one::<String>("batch-size").unwrap().parse()?;

    let start_key = parse_hex_key(&start_hex)?;
    let end_key = parse_hex_key(&end_hex)?;
    
    let target_addresses: HashSet<String> = if targets_str.contains(',') {
        targets_str.split(',').map(|s| s.trim().to_string()).collect()
    } else {
        [targets_str].into_iter().collect()
    };

    info!("Starting Bitcoin puzzle hunter");
    info!("Range: {} to {}", start_hex, end_hex);
    info!("Target addresses: {:?}", target_addresses);
    info!("Threads: {}", if threads == 0 { rayon::current_num_threads() } else { threads });
    info!("Batch size: {}", batch_size);

    if threads > 0 {
        rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .build_global()
            .unwrap();
    }

    let found = Arc::new(AtomicBool::new(false));
    let keys_checked = Arc::new(AtomicU64::new(0));
    
    let total_keys = &end_key - &start_key + 1u32;
    let progress_bar = ProgressBar::new(total_keys.try_into().unwrap_or(u64::MAX));
    progress_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} ({eta}) {per_sec}")
            .unwrap()
            .progress_chars("#>-"),
    );

    let start_time = Instant::now();
    
    let result = search_range(
        start_key,
        end_key,
        target_addresses,
        batch_size,
        found.clone(),
        keys_checked.clone(),
        progress_bar.clone(),
    );

    progress_bar.finish();
    
    let elapsed = start_time.elapsed();
    let total_checked = keys_checked.load(Ordering::Relaxed);
    let keys_per_sec = total_checked as f64 / elapsed.as_secs_f64();
    
    info!("Search completed in {:?}", elapsed);
    info!("Total keys checked: {}", total_checked);
    info!("Keys per second: {:.2}", keys_per_sec);
    
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
    let batch_count = (range_size.clone() / batch_size + 1u32).try_into().unwrap_or(usize::MAX);
    
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
