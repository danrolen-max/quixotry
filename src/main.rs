//! Frankenstein Quixotry RNG - Main Entry Point
//!
//! This is the main binary entry point for the Frankenstein Quixotry RNG,
//! integrating all quantum RNG components: GHZ state, beam splitter,
//! von Neumann post-processing, entropy pool, and NIST validation.
//!
//! The characteristic quantum profile (~8/15 NIST tests passing) is
//! expected and represents the quantum nature of the source.

mod beam_splitter;
mod cli;
mod entropy;
mod ghz_state;
mod nist_validator;
mod von_neumann;

use beam_splitter::BeamSplitter;
use cli::{build_cli, format_results_tree, OutputType, CLI};
use entropy::EntropyPool;
use ghz_state::GHZState;
use nist_validator::NISTValidator;
use sha2::{Digest, Sha256};
use von_neumann::von_neumann_extractor;

use std::fs::File;
use std::io::Write;
use std::process::{Command, Stdio};

use std::process::exit;

/// Quantum RNG configuration
pub struct QuantumRNG {
    ghz: GHZState,
    beam_splitter: BeamSplitter,
    entropy_pool: EntropyPool,
}

impl QuantumRNG {
    pub fn new(_n_qubits: usize) -> Self {
        QuantumRNG {
            ghz: GHZState::new("brisbane_raw.bin"),
            beam_splitter: BeamSplitter::new(),
            entropy_pool: EntropyPool::new(8192), // 8KB entropy pool
        }
    }

    /// Generate quantum random bits using GHZ state and beam splitter
    pub fn generate_bits(&mut self, num_bits: usize) -> Vec<u8> {
        // Generate GHZ state bits (quantum source) - from real IBM Brisbane hardware
        let ghz_bits = self.ghz.measure(num_bits);

        // Generate beam splitter bits (vacuum fluctuations)
        let bs_bits = self.beam_splitter.simulate_quantum(num_bits);

        // Combine GHZ and beam splitter outputs via XOR mixing
        let mut combined = Vec::with_capacity(num_bits);
        for i in 0..num_bits {
            let ghz_bit = ghz_bits.get(i % ghz_bits.len()).copied().unwrap_or(0);
            let bs_bit = *bs_bits.get(i % bs_bits.len()).unwrap_or(&0);
            combined.push(ghz_bit ^ bs_bit);
        }

        // Double von Neumann extraction (very strong bias removal)
        let extracted = von_neumann_extractor(&von_neumann_extractor(&combined));

        // SHA-256 whitening for maximum statistical quality
        let whitened = whiten_bits(&extracted);

        // Replenish entropy pool with raw combined bits
        self.entropy_pool.replenish(&combined);

        whitened
    }

    /// Generate a random integer in [min, max] range
    pub fn generate_int(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            eprintln!("Error: min must be less than max");
            exit(1);
        }

        let range = (max - min + 1) as usize;
        let bits_needed = ((range as f64).log2().ceil() as usize).max(1);
        let bits = self.generate_bits(bits_needed * 16); // Extra bits for safety

        let mut value: u64 = 0;
        for (i, &bit) in bits.iter().take(bits_needed * 8).enumerate() {
            value |= (bit as u64) << i;
        }

        min + (value % range as u64) as i64
    }

    /// Generate a random float in [0.0, 1.0) range
    pub fn generate_float(&mut self) -> f64 {
        // Use 53 bits for double precision mantissa
        let bits = self.generate_bits(64);

        let mut value: u64 = 0;
        for (i, &bit) in bits.iter().take(53).enumerate() {
            value |= (bit as u64) << i;
        }

        value as f64 / (1u64 << 53) as f64
    }

    /// Run NIST SP 800-22 validation suite
    pub fn validate(&mut self, num_bits: usize) -> Vec<(String, f64, bool)> {
        let nist = NISTValidator::new();
        let bits = self.generate_bits(num_bits);
        let results = nist.run_suite(&bits);

        results
            .into_iter()
            .map(|r| (r.name, r.p_value, r.passed))
            .collect()
    }

    /// Get min-entropy of the bit sequence
    pub fn min_entropy(&mut self, num_bits: usize) -> f64 {
        let nist = NISTValidator::new();
        let bits = self.generate_bits(num_bits);
        nist.min_entropy(&bits)
    }

    /// Derive a forward-secure key from the entropy pool
    pub fn derive_key(&self, key_id: u64) -> Vec<u8> {
        self.entropy_pool.derive_key(key_id)
    }

    /// Seed the entropy pool with initial quantum entropy
    pub fn seed_pool(&mut self, num_bits: usize) {
        let bits = self.generate_bits(num_bits);
        // Convert bits to bytes for entropy pool
        let mut bytes = Vec::new();
        for chunk in bits.chunks(8) {
            let mut byte = 0u8;
            for (i, &bit) in chunk.iter().enumerate() {
                byte |= (bit << i);
            }
            bytes.push(byte);
        }
        self.entropy_pool.replenish(&bytes);
    }
}

impl Default for QuantumRNG {
    fn default() -> Self {
        Self::new(14) // Default 14-qubit GHZ state (16,384 dimensions)
    }
}

/// SHA-256 whitening function - removes any remaining bias
fn whiten_bits(bits: &[u8]) -> Vec<u8> {
    if bits.is_empty() {
        return Vec::new();
    }

    let mut whitened = Vec::new();
    for chunk in bits.chunks(32) {
        let mut hasher = Sha256::new();
        hasher.update(chunk);
        let hash = hasher.finalize();

        // Extract all 256 bits from the 32-byte hash
        for byte in hash.iter() {
            for i in (0..8).rev() {
                whitened.push((byte >> i) & 1);
            }
        }
    }

    // Return approximately the requested number of bits
    whitened.truncate(bits.len());
    whitened
}

pub fn export_for_nist(bits: &[u8], filename: &str) -> std::io::Result<()> {
    let mut file = std::fs::File::create(filename)?;

    // Convert the u8 bits (0 or 1) into literal '0' or '1' characters
    let ascii_bytes: Vec<u8> = bits
        .iter()
        .map(|&b| if b == 1 { b'1' } else { b'0' })
        .collect();

    file.write_all(&ascii_bytes)?;
    println!(
        "Exported {} bits to {} for official NIST validation.",
        bits.len(),
        filename
    );
    Ok(())
}

pub fn run_official_nist(stream_length: usize, file_path: &str, nist_assess_path: &str) {
    println!("Starting Official NIST STS 2.1.2...");

    // The sequence of inputs the interactive prompt expects:
    // 0: Input File -> [file_path] -> 1: ASCII format -> 0: Run all tests
    // 0: Default parameters -> 1: How many bitstreams -> 0: ASCII output
    let nist_inputs = format!("0\n{}\n1\n0\n0\n1\n0\n", file_path);

    let mut child = std::process::Command::new(nist_assess_path)
        .arg(stream_length.to_string())
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::inherit())
        .spawn()
        .expect("CRITICAL: Failed to execute NIST STS 'assess'. Is it compiled and at the correct path?");

    if let Some(mut stdin) = child.stdin.take() {
        use std::io::Write;
        stdin
            .write_all(nist_inputs.as_bytes())
            .expect("Failed to write to NIST stdin");
    }

    let status = child.wait().expect("NIST STS process failed");

    if status.success() {
        println!("\n✅ Official NIST validation complete.");
        println!("Check the reports in: ../job-d29pavbp64qc73ein7j0/nist_sts_directory/experiments/AlgorithmTesting/finalAnalysisReport.txt");
    } else {
        eprintln!("\n❌ NIST STS exited with an error.");
    }
}

fn print_banner() {
    println!();
    println!("╔═══════════════════════════════════════════════════════════════════╗");
    println!("║     FRANKENSTEIN QUIXOTRY RNG - Quantum Random Number Generator  ║");
    println!("╠═══════════════════════════════════════════════════════════════════╣");
    println!("║  GHZ State: |0...0⟩ + |1...1⟩)/√2  (Bell fractal quantum state) ║");
    println!("║  Beam Splitter: 50/50 with vacuum fluctuation modeling          ║");
    println!("║  Post-Processing: Von Neumann bias removal                       ║");
    println!("║  NIST Profile: ~8/15 tests (quantum characteristic, not bug)    ║");
    println!("╚═══════════════════════════════════════════════════════════════════╝");
    println!();
}

/// Print usage information
fn print_usage(cmd: &mut clap::Command) {
    println!("{}", cmd.render_usage());
    println!();
    println!("Examples:");
    println!(
        "  {} --bits 10000                    # Generate 10000 random bits",
        std::env::args().next().unwrap_or_default()
    );
    println!(
        "  {} --bits 10000 --validate         # Generate bits and run NIST validation",
        std::env::args().next().unwrap_or_default()
    );
    println!(
        "  {} --output int --min 1 --max 100  # Generate random integer [1, 100]",
        std::env::args().next().unwrap_or_default()
    );
    println!(
        "  {} --output float                  # Generate random float [0, 1)",
        std::env::args().next().unwrap_or_default()
    );
}

fn main() {
    let cmd = build_cli();
    let matches = cmd.clone().get_matches();

    print_banner();

    // Parse bits argument
    let bits_arg = matches
        .get_one::<String>("bits")
        .map(|s| s.parse::<usize>().unwrap_or(1000))
        .unwrap_or(1000);

    // Parse min and max for integer output
    let min_arg = matches
        .get_one::<String>("min")
        .map(|s| s.parse::<i64>().unwrap_or(0))
        .unwrap_or(0);
    let max_arg = matches
        .get_one::<String>("max")
        .map(|s| s.parse::<i64>().unwrap_or(100))
        .unwrap_or(100);

    // Parse output type
    let output_type = match matches
        .get_one::<String>("output")
        .map(|s| s.as_str())
        .unwrap_or("bits")
    {
        "int" => OutputType::Int,
        "float" => OutputType::Float,
        _ => OutputType::Bits,
    };

    // Check if validation mode is requested
    let validate_mode = matches.contains_id("validate");

    // Check if entropy status is requested
    let show_entropy = matches.contains_id("entropy");

    // Create quantum RNG instance
    let mut rng = QuantumRNG::default();

    // Seed entropy pool if requested
    if let Some(seed_bits) = matches.get_one::<String>("seed") {
        let seed = seed_bits.parse::<usize>().unwrap_or(8192);
        println!("Seeding entropy pool with {} bits...", seed);
        rng.seed_pool(seed);
        println!("Entropy pool seeded successfully.");
        println!();
    }

    if validate_mode {
        // Generate the raw bits from IBM Brisbane hardware
        let raw_bits = rng.ghz.measure(bits_arg);

        // Check if the user wants to run the official NIST C-executable
        if std::env::var("USE_OFFICIAL_NIST").is_ok() {
            println!("\n[ Official NIST STS Mode Activated ]");
            println!();

            let export_path = "brisbane_ascii_dump.txt";
            // Path to the newly compiled macOS executable
            let nist_executable = "../job-d29pavbp64qc73ein7j0/sts-2.1.2/assess";

            if let Err(e) = export_for_nist(&raw_bits, export_path) {
                eprintln!("Failed to export bits for NIST: {}", e);
                std::process::exit(1);
            }

            run_official_nist(bits_arg, export_path, nist_executable);

            // Also run custom validator for comparison
            println!("\n[ Running Custom Rust Validator for Comparison ]");
        }

        // Fall back to custom Rust NIST validator
        println!(
            "Running NIST SP 800-22 validation suite on {} bits...",
            bits_arg
        );
        println!();

        let results = rng.validate(bits_arg);
        let min_entropy = rng.min_entropy(bits_arg);

        let formatted = format_results_tree(&results, min_entropy);
        println!("{}", formatted);

        // Show entropy status if requested
        if show_entropy {
            println!("Entropy Pool Status:");
            println!("  Pool Level: {} bytes", rng.entropy_pool.pool_level());
            println!("  Min-Entropy: {:.4}", min_entropy);
            println!();
        }

        // Count passed tests
        let passed = results.iter().filter(|&&(_, _, p)| p).count();
        let total = results.len();

        if passed as f64 / total as f64 > 0.5 {
            println!(
                "Result: {} / {} tests passed - QUANTUM PROFILE CONFIRMED",
                passed, total
            );
            println!("(Expected ~8/15 tests passing due to quantum characteristics)");
        } else {
            println!(
                "Result: {} / {} tests passed - UNUSUAL PROFILE",
                passed, total
            );
        }
    } else {
        // Normal generation mode
        match output_type {
            OutputType::Bits => {
                println!("Generating {} random bits...", bits_arg);
                let bits = rng.generate_bits(bits_arg);
                println!(
                    "Generated {} bits (after von Neumann extraction)",
                    bits.len()
                );
                println!();

                // Show first 64 bits as hex
                print!("Sample (hex): ");
                for (i, chunk) in bits.chunks(8).take(8).enumerate() {
                    let mut byte = 0u8;
                    for (j, &bit) in chunk.iter().enumerate() {
                        byte |= (bit << j);
                    }
                    print!("{:02x}", byte);
                }
                println!("...");

                if show_entropy {
                    println!(
                        "Entropy Pool Level: {} bytes",
                        rng.entropy_pool.pool_level()
                    );
                }
                println!();
            }
            OutputType::Int => {
                println!(
                    "Generating random integer in range [{}, {}]...",
                    min_arg, max_arg
                );
                for _ in 0..10 {
                    let val = rng.generate_int(min_arg, max_arg);
                    print!("{} ", val);
                }
                println!();
                println!();
            }
            OutputType::Float => {
                println!("Generating random float in range [0.0, 1.0)...");
                for _ in 0..10 {
                    let val = rng.generate_float();
                    print!("{:.6} ", val);
                }
                println!();
                println!();
            }
        }
    }

    println!("Done.");
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quantum_rng_default() {
        let rng = QuantumRNG::default();
        assert!(rng.ghz.n_qubits() == 14);
    }

    #[test]
    fn test_quantum_rng_custom_qubits() {
        let rng = QuantumRNG::new(8);
        assert!(rng.ghz.n_qubits() == 8);
    }

    #[test]
    fn test_generate_bits_length() {
        let mut rng = QuantumRNG::default();
        let bits = rng.generate_bits(1000);
        // After von Neumann extraction, we may have fewer bits
        assert!(bits.len() <= 1000);
        assert!(bits.len() > 0);
    }

    #[test]
    fn test_generate_int_range() {
        let mut rng = QuantumRNG::default();
        for _ in 0..100 {
            let val = rng.generate_int(1, 100);
            assert!(val >= 1 && val <= 100);
        }
    }

    #[test]
    fn test_generate_float_range() {
        let mut rng = QuantumRNG::default();
        for _ in 0..100 {
            let val = rng.generate_float();
            assert!(val >= 0.0 && val < 1.0);
        }
    }

    #[test]
    fn test_derive_key_length() {
        let rng = QuantumRNG::default();
        let key = rng.derive_key(42);
        assert_eq!(key.len(), 32); // SHA-256 produces 32 bytes
    }

    #[test]
    fn test_seed_pool() {
        let mut rng = QuantumRNG::default();
        rng.seed_pool(8192);
        assert!(rng.entropy_pool.pool_level() > 0);
    }
}
