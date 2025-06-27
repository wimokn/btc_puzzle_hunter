# Bitcoin Puzzle Hunter Examples

## Puzzle Mode (Recommended)

### List All Unsolved Puzzles
```bash
cargo run --release -- --list
```

### Show Easiest Puzzles
```bash
cargo run --release -- --easy 3
```

### Search Specific Puzzle
```bash
# Search puzzle #71 (easiest unsolved - 7.1 BTC reward)
cargo run --release -- --puzzle 71 --threads 8 --batch-size 10000000

# Search puzzle #72 with custom settings
cargo run --release -- --puzzle 72 --threads 16 --batch-size 50000000
```

## Manual Mode

### Test with Known Solutions
```bash
# Bitcoin Puzzle #1
cargo run --release -- --start 0x1 --end 0x10 --targets "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH"

# Bitcoin Puzzle #3  
cargo run --release -- --start 0x7 --end 0x7 --targets "19ZewH8Kk1PDbSNdJ97FP4EiCjTRaZMZQA"
```

### Custom Range Search
```bash
# Large range with multiple threads
cargo run --release -- --start 0x400000000000000000 --end 0x7fffffffffffffffff --targets "1PWo3JeB9jrGwfHDNpdGK54CRas7fsVzXU" --threads 8 --batch-size 1000000

# Multiple target addresses
cargo run --release -- --start 0x1 --end 0x100 --targets "1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH,19ZewH8Kk1PDbSNdJ97FP4EiCjTRaZMZQA"
```

## Command Line Arguments

### Puzzle Mode
- `--puzzle, -p`: Bitcoin puzzle number to solve (auto-loads range/target)
- `--list, -l`: List all available unsolved puzzles
- `--easy`: Show N easiest unsolved puzzles (default: 5)

### Manual Mode  
- `--start`: Start of private key range in hex (required if not using --puzzle)
- `--end`: End of private key range in hex (required if not using --puzzle)  
- `--targets`: Comma-separated list of target Bitcoin addresses (required if not using --puzzle)

### Performance Settings
- `--threads, -t`: Number of threads to use (default: auto-detect)
- `--batch-size, -b`: Batch size for each thread (default: 1,000,000)

## Performance Tips

1. Use `--release` flag for optimized builds
2. Adjust `--batch-size` based on your system (smaller for more frequent progress updates)
3. Set `--threads` to match your CPU cores for optimal performance
4. Use `RUST_LOG=info` for detailed logging

## Address Types Generated

For each private key, the program generates:
- Compressed P2PKH address (1...)
- Uncompressed P2PKH address (1...)  
- Bech32 P2WPKH address (bc1...) when supported

## Known Bitcoin Puzzle Solutions

- Puzzle #1: Private key `1` → Address `1BgGZ9tcN4rm9KBzDn7KprQz87SZ26SAMH`
- Puzzle #2: Private key `3` → Address `1CUTxxqJWs9FMMSqZgJH6jWNKbKZjNMFLP`
- Puzzle #3: Private key `7` → Address `19ZewH8Kk1PDbSNdJ97FP4EiCjTRaZMZQA`