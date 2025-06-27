# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

This is a Rust-based Bitcoin puzzle hunter application designed to solve Bitcoin "puzzle transactions" through multithreaded brute force key generation and testing. The application generates candidate private keys and tests them against known public addresses or script hashes from the Bitcoin Puzzle Challenge.

## Development Setup

Since this is a new Rust project, you'll need to initialize it first:

```bash
cargo init
```

## Common Commands

- **Build**: `cargo build`
- **Build optimized**: `cargo build --release`
- **List puzzles**: `cargo run --release -- --list`
- **Show easy puzzles**: `cargo run --release -- --easy 5`
- **Run specific puzzle**: `cargo run --release -- --puzzle 71 --threads 8`
- **Manual range**: `cargo run --release -- --start 0x1 --end 0x100 --targets "address"`
- **Test**: `cargo test`
- **Lint**: `cargo clippy`
- **Format**: `cargo fmt`

## Architecture Considerations

This application will likely need:

1. **Multithreading**: Use Rust's threading capabilities (std::thread, rayon, tokio) for parallel key generation
2. **Bitcoin cryptography**: Dependencies like `secp256k1`, `bitcoin`, or `k256` for elliptic curve operations
3. **Performance optimization**: Consider using release builds and CPU-specific optimizations
4. **Memory management**: Efficient handling of large key ranges and address lists
5. **Progress tracking**: Threading-safe progress reporting and statistics
6. **Configuration**: Command-line arguments or config files for puzzle parameters

## Key Dependencies to Consider

- `secp256k1` - Bitcoin elliptic curve cryptography
- `bitcoin` - Bitcoin protocol implementations
- `rayon` - Data parallelism
- `clap` - Command line argument parsing
- `serde` - Serialization/deserialization
- `tokio` - Async runtime (if needed)

## Security Notes

This is a legitimate cryptocurrency research/educational tool. Ensure proper key generation randomness and secure handling of any discovered private keys.