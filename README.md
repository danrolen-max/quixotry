# Quixotry

A Rust-based quantum random number generator preserving the original FQRNG quantum profile.

## Overview

Quixotry provides a minimal, production-oriented CLI that combines:

- GHZ-state quantum randomness from real IBM Brisbane hardware data
- Vacuum fluctuation simulation via a beam splitter model
- Von Neumann bias removal
- NIST SP 800-22 statistical validation
- Forward-secure entropy pool management

This project is intentionally focused on preserving the quantum profile rather than forcing full classical NIST compliance.

## Features

- Pure Rust implementation with a small dependency set
- CLI modes for bit, integer, and float generation
- NIST SP 800-22 validation support
- Forward-secure entropy pool and SHA-256-based whitening
- Real quantum source integration via `brisbane_raw.bin`

## Usage

Build the project:

```bash
cargo build --release
```

Generate bits:

```bash
./target/release/quixotry --bits 1024
```

Run validation:

```bash
./target/release/quixotry --bits 10000 --validate
```

Generate a random integer:

```bash
./target/release/quixotry --output int --min 0 --max 100
```

Generate a random float:

```bash
./target/release/quixotry --output float
```

## Command Line Options

| Option | Description | Default |
|--------|-------------|---------|
| `--bits <N>` | Number of bits to generate | `1024` |
| `--validate` | Run NIST SP 800-22 validation | `false` |
| `--output <type>` | Output type: `bits`, `int`, `float` | `bits` |
| `--min <N>` | Minimum integer output | `0` |
| `--max <N>` | Maximum integer output | `100` |
| `--seed <N>` | Seed entropy pool with N bits | `8192` |
| `--entropy` | Show entropy pool status | `false` |

## Test Status

- Local `cargo test` completed successfully: **56 tests passed**.
- The project remains aligned with the expected quantum profile.

## Project Structure

```text
quixotry/
├── Cargo.toml
├── README.md
├── brisbane_raw.bin
├── build_manager
├── extract_brisbane.py
└── src/
    ├── beam_splitter.rs
    ├── cli.rs
    ├── entropy.rs
    ├── ghz_state.rs
    ├── main.rs
    ├── nist_validator.rs
    └── von_neumann.rs
```

## Build

```bash
cargo build --release
cargo test
```

## Notes

- This repository is meant as a focused Rust implementation of the original FQRNG quantum RNG concept.
- It preserves the intended quantum profile rather than applying classical noise correction.
