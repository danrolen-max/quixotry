// RNG/quixotry/src/cli.rs
//! CLI Interface - Command-line interface for Frankenstein Quixotry RNG.
//!
//! This module provides the command-line interface for the FQRNG,
//! allowing users to generate random bits, integers, or floats
//! with optional NIST validation.

use crate::beam_splitter::BeamSplitter;
use crate::entropy::EntropyPool;
use crate::ghz_state::GHZState;
use crate::nist_validator::NISTValidator;
use crate::von_neumann::von_neumann_extractor;
use clap::{Arg, ArgAction, Command};
use std::process::exit;

/// Output type for random number generation
#[derive(Clone, Debug, PartialEq)]
pub enum OutputType {
    /// Raw bits output
    Bits,
    /// Integer output (between min and max)
    Int,
    /// Floating-point output (between 0.0 and 1.0)
    Float,
}

/// CLI application for Frankenstein Quixotry RNG
pub struct CLI {
    /// GHZ state generator for quantum randomness
    ghz: GHZState,
    /// Beam splitter for vacuum fluctuation simulation
    beam_splitter: BeamSplitter,
    /// Entropy pool for forward-secure management
    entropy_pool: EntropyPool,
    /// NIST validator for statistical testing
    nist: NISTValidator,
}

impl CLI {
    /// Create a new CLI instance.
    pub fn new() -> Self {
        CLI {
            ghz: GHZState::new("brisbane_raw.bin", 14),
            beam_splitter: BeamSplitter::new(),
            entropy_pool: EntropyPool::new(1024),
            nist: NISTValidator::new(),
        }
    }

    /// Generate random bits using the quantum source.
    ///
    /// Combines GHZ state, beam splitter, and von Neumann extraction
    /// to produce high-quality quantum random bits.
    ///
    /// # Arguments
    ///
    /// * `num_bits` - Number of bits to generate
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - Vector of random bits (0 or 1)
    pub fn generate_bits(&mut self, num_bits: usize) -> Vec<u8> {
        // Generate raw quantum bits using GHZ state
        let ghz_bits = self.ghz.measure(num_bits);

        // Simulate beam splitter with vacuum fluctuations
        let bs_bits = self.beam_splitter.simulate_quantum(num_bits);

        // Combine GHZ and beam splitter output
        let mut combined = Vec::new();
        for i in 0..num_bits {
            let ghz_bit = ghz_bits[i % ghz_bits.len()];
            let bs_bit = bs_bits[i % bs_bits.len()];
            combined.push(ghz_bit ^ bs_bit);
        }

        // Apply von Neumann post-processing for bias removal
        let extracted = von_neumann_extractor(&combined);

        // Replenish entropy pool with raw bits
        self.entropy_pool.replenish(&combined);

        extracted
    }

    /// Generate a random integer between min and max (inclusive).
    ///
    /// # Arguments
    ///
    /// * `min` - Minimum value (inclusive)
    /// * `max` - Maximum value (inclusive)
    ///
    /// # Returns
    ///
    /// * `i64` - Random integer in range [min, max]
    pub fn generate_int(&mut self, min: i64, max: i64) -> i64 {
        if min >= max {
            eprintln!("Error: min must be less than max");
            exit(1);
        }

        let range = (max - min + 1) as usize;
        let bits_needed = ((range as f64).log2().ceil() as usize).max(1);

        // Generate enough bits to cover the range
        let bits = self.generate_bits(bits_needed * 8);

        // Convert bits to integer
        let mut value: u64 = 0;
        for (i, &bit) in bits.iter().take(bits_needed * 8).enumerate() {
            value |= (bit as u64) << i;
        }

        // Reduce to range
        let result = min + (value % range as u64) as i64;
        result.max(min).min(max)
    }

    /// Generate a random float between 0.0 and 1.0.
    ///
    /// # Returns
    ///
    /// * `f64` - Random float in range [0.0, 1.0)
    pub fn generate_float(&mut self) -> f64 {
        // Generate 53 bits for double precision
        let bits = self.generate_bits(53);

        let mut value: u64 = 0;
        for (i, &bit) in bits.iter().take(53).enumerate() {
            value |= (bit as u64) << i;
        }

        // Convert to float [0.0, 1.0)
        value as f64 / (1u64 << 53) as f64
    }

    /// Validate the random bits using NIST SP 800-22 tests.
    ///
    /// # Arguments
    ///
    /// * `num_bits` - Number of bits to validate
    ///
    /// # Returns
    ///
    /// * `Vec<(String, f64, bool)>` - Test name, p-value, and pass status
    pub fn validate(&mut self, num_bits: usize) -> Vec<(String, f64, bool)> {
        let bits = self.generate_bits(num_bits);
        let results = self.nist.run_suite(&bits);

        results
            .into_iter()
            .map(|r| (r.name, r.p_value, r.passed))
            .collect()
    }

    /// Get min-entropy of the generated bits.
    ///
    /// # Arguments
    ///
    /// * `num_bits` - Number of bits to analyze
    ///
    /// # Returns
    ///
    /// * `f64` - Min-entropy value (0.0 to 1.0)
    pub fn min_entropy(&mut self, num_bits: usize) -> f64 {
        let bits = self.generate_bits(num_bits);
        self.nist.min_entropy(&bits)
    }
}

impl Default for CLI {
    fn default() -> Self {
        Self::new()
    }
}

/// Build and parse the CLI arguments.
pub fn build_cli() -> Command {
    Command::new("Frankenstein Quixotry RNG")
        .version("1.0.0")
        .author("Frankenstein Quixotry RNG Team")
        .about("Quantum RNG with GHZ state, beam splitter, and von Neumann extraction")
        .arg(
            Arg::new("bits")
                .long("bits")
                .short('b')
                .value_name("NUM")
                .help("Number of bits to generate")
                .default_value("1000"),
        )
        .arg(
            Arg::new("validate")
                .long("validate")
                .short('v')
                .action(ArgAction::SetTrue)
                .help("Run NIST SP 800-22 validation suite"),
        )
        .arg(
            Arg::new("output")
                .long("output")
                .short('o')
                .value_name("TYPE")
                .help("Output type: bits, int, float")
                .default_value("bits"),
        )
        .arg(
            Arg::new("min")
                .long("min")
                .short('m')
                .value_name("VALUE")
                .help("Minimum value for integer output")
                .default_value("0"),
        )
        .arg(
            Arg::new("max")
                .long("max")
                .short('M')
                .value_name("VALUE")
                .help("Maximum value for integer output")
                .default_value("100"),
        )
        .arg(
            Arg::new("entropy")
                .long("entropy")
                .short('e')
                .action(ArgAction::SetTrue)
                .help("Show entropy pool status"),
        )
        .arg(
            Arg::new("seed")
                .long("seed")
                .short('s')
                .value_name("NUM")
                .help("Number of bits to use for entropy pool seeding")
                .default_value("8192"),
        )
}

/// Format results tree for validation output.
pub fn format_results_tree(results: &[(String, f64, bool)], min_entropy: f64) -> String {
    let mut output = String::new();

    output.push_str("╔══════════════════════════════════════════════════════════════════╗\n");
    output.push_str("║            FRANKENSTEIN QUIXOTRY RNG - NIST VALIDATION         ║\n");
    output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");

    let passed = results.iter().filter(|&&(_, _, p)| p).count();
    let total = results.len();

    output.push_str(&format!(
        "║  Profile: {:>2}/{:>2} tests passed (quantum characteristic)     ║\n",
        passed, total
    ));
    output.push_str(&format!(
        "║  Min-Entropy: {:.4} (target: >0.98)                           ║\n",
        min_entropy
    ));
    output.push_str("╠══════════════════════════════════════════════════════════════════╣\n");

    for (name, p_value, passed) in results {
        let status = if *passed { "✓ PASS" } else { "✗ FAIL" };
        let p_str = format!("p = {:.4}", p_value);
        output.push_str(&format!("║  {:50} {:>12} ║\n", name, status));
        output.push_str(&format!("║    {:>60}  ║\n", p_str));
    }

    output.push_str("╚══════════════════════════════════════════════════════════════════╝\n");

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cli_creation() {
        let cli = CLI::new();
        assert!(cli.ghz.n_qubits() == 14);
    }

    #[test]
    fn test_generate_bits_length() {
        let mut cli = CLI::new();
        let bits = cli.generate_bits(100);
        assert!(bits.len() <= 100); // May be less due to von Neumann extraction
    }

    #[test]
    fn test_generate_bits_values() {
        let mut cli = CLI::new();
        let bits = cli.generate_bits(1000);
        for bit in &bits {
            assert!(*bit == 0 || *bit == 1);
        }
    }

    #[test]
    fn test_output_type_equality() {
        assert_eq!(OutputType::Bits, OutputType::Bits);
        assert_eq!(OutputType::Int, OutputType::Int);
        assert_eq!(OutputType::Float, OutputType::Float);
        assert_ne!(OutputType::Bits, OutputType::Int);
    }

    #[test]
    fn test_int_range() {
        let mut cli = CLI::new();
        for _ in 0..100 {
            let val = cli.generate_int(0, 100);
            assert!(val >= 0 && val <= 100);
        }
    }

    #[test]
    fn test_float_range() {
        let mut cli = CLI::new();
        for _ in 0..100 {
            let val = cli.generate_float();
            assert!(val >= 0.0 && val < 1.0);
        }
    }
}
