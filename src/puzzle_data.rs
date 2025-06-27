use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct PuzzleData {
    pub puzzle: u32,
    pub bits: u32,
    pub range_start: String,
    pub range_end: String,
    pub address: String,
    pub reward_btc: f64,
}

pub fn load_unsolved_puzzles() -> Result<Vec<PuzzleData>> {
    let data = fs::read_to_string("unsolved_puzzles.json")?;
    let puzzles: Vec<PuzzleData> = serde_json::from_str(&data)?;
    Ok(puzzles)
}

pub fn get_puzzle_by_number(puzzle_number: u32) -> Result<Option<PuzzleData>> {
    let puzzles = load_unsolved_puzzles()?;
    Ok(puzzles.into_iter().find(|p| p.puzzle == puzzle_number))
}

pub fn list_available_puzzles() -> Result<()> {
    let puzzles = load_unsolved_puzzles()?;
    
    println!("Available unsolved Bitcoin puzzles:");
    println!("┌────────┬──────┬─────────────┬──────────────────────────────────────┐");
    println!("│ Puzzle │ Bits │ Reward (BTC)│ Address                              │");
    println!("├────────┼──────┼─────────────┼──────────────────────────────────────┤");
    
    for puzzle in puzzles {
        println!(
            "│ {:6} │ {:4} │ {:11.1} │ {} │",
            puzzle.puzzle, puzzle.bits, puzzle.reward_btc, puzzle.address
        );
    }
    
    println!("└────────┴──────┴─────────────┴──────────────────────────────────────┘");
    
    Ok(())
}

pub fn get_easiest_puzzles(count: usize) -> Result<Vec<PuzzleData>> {
    let mut puzzles = load_unsolved_puzzles()?;
    puzzles.sort_by_key(|p| p.bits);
    Ok(puzzles.into_iter().take(count).collect())
}