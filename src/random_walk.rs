use crate::bitcoin_utils::private_key_to_addresses;
use anyhow::Result;
use indicatif::ProgressBar;
use num_bigint::BigUint;
use num_traits::{One, Zero};
use rand::Rng;
use std::collections::HashSet;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;


/// Advanced adaptive random walk search with configurable adaptation interval
pub fn adaptive_random_walk_search(
    start_range: &BigUint,
    end_range: &BigUint,
    targets: &HashSet<String>,
    max_iter: usize,
    adapt_interval: usize,
    progress_bar: Option<ProgressBar>,
    keys_checked: Option<Arc<AtomicU64>>,
) -> Result<Option<(String, String)>> {
    let mut rng = rand::thread_rng();
    let mut seen = HashSet::new();

    // Use the range size as our modulus
    let range_size = end_range - start_range + 1u32;
    if range_size.is_zero() {
        return Ok(None);
    }

    // Initialize random walk parameters
    let mut step_size = BigUint::from(rng.gen_range(1u32..100u32));
    let mut position = BigUint::from(rng.r#gen::<u64>()) % &range_size;
    let mut iterations_since_adapt = 0;
    let mut total_adaptations = 0;
    let mut local_count = 0u64;

    // Step size variants for adaptation
    let step_variants = [
        1u32, 2u32, 3u32, 5u32, 8u32, 13u32, 21u32, 34u32, 55u32, 89u32,
        144u32, 233u32, 377u32, 610u32, 987u32, 1597u32
    ];

    for i in 0..max_iter {
        // Random direction with slight forward bias
        let direction = if rng.r#gen::<f64>() < 0.6 { 1i32 } else { -1i32 };
        
        // Apply random step
        if direction > 0 {
            position = (&position + &step_size) % &range_size;
        } else {
            // Handle negative direction properly
            if step_size <= position {
                position -= &step_size;
            } else {
                let step_mod = &step_size % &range_size;
                if step_mod <= position {
                    position -= &step_mod;
                } else {
                    position = &range_size - (&step_mod - &position);
                }
            }
        }

        // Calculate actual private key
        let private_key = start_range + &position;

        // Check if we've seen this key before (cycle detection)
        if !seen.insert(position.clone()) {
            // Cycle detected, jump to new random position and change step
            position = BigUint::from(rng.r#gen::<u64>()) % &range_size;
            step_size = BigUint::from(step_variants[rng.gen_range(0..step_variants.len())]);
            seen.clear();
            total_adaptations += 1;
            iterations_since_adapt = 0;
            continue;
        }

        // Test the private key against target addresses
        if let Ok(addresses) = private_key_to_addresses(&private_key) {
            for address in addresses {
                if targets.contains(&address) {
                    let hex_key = format!("{:064x}", private_key);
                    return Ok(Some((hex_key, address)));
                }
            }
        }

        iterations_since_adapt += 1;
        local_count += 1;

        // Update progress bar and counter periodically
        if local_count % 1000 == 0 {
            if let Some(ref counter) = keys_checked {
                counter.fetch_add(local_count, Ordering::Relaxed);
            }
            if let Some(ref pb) = progress_bar {
                pb.inc(local_count);
            }
            local_count = 0;
        }

        // Adaptive step changing: change step size periodically
        if iterations_since_adapt >= adapt_interval {
            adapt_random_walk(&mut step_size, &step_variants, &mut rng);
            
            // Occasionally clear seen positions to explore previously visited areas
            if total_adaptations % 4 == 0 {
                seen.clear();
            }
            
            // Slightly randomize position to avoid getting stuck
            if rng.r#gen::<f64>() < 0.3 {
                position = (position + BigUint::from(rng.r#gen::<u32>())) % &range_size;
            }
            
            iterations_since_adapt = 0;
            total_adaptations += 1;
        }

        // Progressive exploration: occasional random jumps
        if i > 0 && i % (max_iter / 8) == 0 {
            if rng.r#gen::<f64>() < 0.25 {
                position = BigUint::from(rng.r#gen::<u64>()) % &range_size;
                step_size = BigUint::from(step_variants[rng.gen_range(0..step_variants.len())]);
                seen.clear();
            }
        }

        // Dynamic step size adjustment based on progress
        if i % 5000 == 0 && i > 0 {
            // Increase step size if we haven't found anything in a while
            if rng.r#gen::<f64>() < 0.4 {
                let multiplier = rng.gen_range(2u32..10u32);
                step_size = (&step_size * multiplier) % &range_size;
                if step_size.is_zero() {
                    step_size = BigUint::one();
                }
            }
        }
    }

    // Final progress update for remaining local count
    if local_count > 0 {
        if let Some(ref counter) = keys_checked {
            counter.fetch_add(local_count, Ordering::Relaxed);
        }
        if let Some(ref pb) = progress_bar {
            pb.inc(local_count);
        }
    }

    Ok(None)
}

/// Adapts the random walk step size to a new variant
fn adapt_random_walk(
    step_size: &mut BigUint,
    step_variants: &[u32],
    rng: &mut impl Rng,
) {
    // Choose new step size
    let base_step = step_variants[rng.gen_range(0..step_variants.len())];
    
    // Add some randomness to the step
    let random_factor = rng.gen_range(1u32..20u32);
    *step_size = BigUint::from(base_step * random_factor);
    
    // Occasionally use very large steps for long jumps
    if rng.r#gen::<f64>() < 0.1 {
        *step_size *= BigUint::from(rng.gen_range(100u32..1000u32));
    }
}


/// Multi-threaded Adaptive Random Walk search with progress tracking
pub fn parallel_adaptive_random_walk_search_with_progress(
    start_range: &BigUint,
    end_range: &BigUint,
    targets: &HashSet<String>,
    max_iter_per_thread: usize,
    num_walks: usize,
    adapt_interval: usize,
    progress_bar: Option<ProgressBar>,
    keys_checked: Option<Arc<AtomicU64>>,
) -> Result<Option<(String, String)>> {
    use rayon::prelude::*;

    // Run multiple independent adaptive random walks in parallel
    // Each walk uses a different adaptation interval and starting position
    let result = (0..num_walks)
        .into_par_iter()
        .map(|walk_id| {
            // Vary adaptation interval per walk for better exploration
            let varied_adapt_interval = adapt_interval + (walk_id * 200);
            adaptive_random_walk_search(
                start_range, 
                end_range, 
                targets, 
                max_iter_per_thread, 
                varied_adapt_interval,
                progress_bar.clone(),
                keys_checked.clone()
            )
        })
        .find_map_first(|result| match result {
            Ok(Some(found)) => Some(Ok(Some(found))),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        });

    match result {
        Some(r) => r,
        None => Ok(None),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
        
    #[test]
    fn test_adaptive_random_walk_search_small_range() {
        let start = BigUint::from(1u32);
        let end = BigUint::from(100u32);
        let targets = HashSet::new(); // Empty targets for test
        
        let result = adaptive_random_walk_search(&start, &end, &targets, 1000, 100, None, None);
        assert!(result.is_ok());
        assert!(result.unwrap().is_none()); // Should not find anything with empty targets
    }
    
    #[test]
    fn test_adapt_random_walk() {
        let mut rng = rand::thread_rng();
        let step_variants = [1u32, 5u32, 10u32];
        let mut step_size = BigUint::from(1u32);
        
        adapt_random_walk(&mut step_size, &step_variants, &mut rng);
        
        // Step size should have changed
        assert!(step_size > BigUint::zero());
    }
}