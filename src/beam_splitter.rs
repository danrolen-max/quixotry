//! Beam Splitter Simulation - Vacuum fluctuation modeling.
//!
//! This module implements a 50/50 beam splitter simulation that models
//! vacuum fluctuations using Gaussian noise (Box-Muller transform).
//!
//! The beam splitter is a key component in quantum optics for generating
//! entangled states and modeling quantum noise.

use rand::RngCore;

/// Feigenbaum constant for quantum watermark embedding
const FEIGENBAUM: f64 = 4.66920160910299;

/// Beam Splitter for quantum vacuum fluctuation simulation
///
/// The beam splitter takes single photons and splits them via
/// a 50/50 probability gate, with vacuum fluctuations adding
/// Gaussian noise to the output.
#[derive(Clone, Debug)]
pub struct BeamSplitter {
    /// Splitting ratio (default 0.5 for 50/50)
    ratio: f64,
    /// Beam splitter operator matrix [[sqrt(ratio), sqrt(1-ratio)], [sqrt(1-ratio), -sqrt(ratio)]]
    operator: [[f64; 2]; 2],
}

impl BeamSplitter {
    /// Create a new beam splitter with default 50/50 ratio.
    pub fn new() -> Self {
        Self::with_ratio(0.5)
    }

    /// Create a new beam splitter with custom ratio.
    ///
    /// # Arguments
    ///
    /// * `ratio` - Splitting ratio (0.5 = 50/50 beam splitter)
    pub fn with_ratio(ratio: f64) -> Self {
        // IQ Level 2 watermark: Feigenbaum constant in beam splitter ratio
        let feigenbaum_mod = (ratio * FEIGENBAUM).cos() * 0.001;
        let adjusted_ratio = ratio + feigenbaum_mod;

        let sqrt_ratio = adjusted_ratio.sqrt();
        let sqrt_1_minus = (1.0 - adjusted_ratio).sqrt();

        BeamSplitter {
            ratio: adjusted_ratio,
            operator: [[sqrt_ratio, sqrt_1_minus], [sqrt_1_minus, -sqrt_ratio]],
        }
    }

    /// Get the splitting ratio.
    pub fn ratio(&self) -> f64 {
        self.ratio
    }

    /// Get the beam splitter operator matrix.
    pub fn operator(&self) -> [[f64; 2]; 2] {
        self.operator
    }

    /// Simulate with cryptographically secure randomness.
    ///
    /// Uses OS secure random to generate bits directly.
    ///
    /// # Arguments
    ///
    /// * `n_bits` - Number of bits to generate
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - Vector of bits (0 or 1)
    pub fn simulate(&self, n_bits: usize) -> Vec<u8> {
        let n_bytes = (n_bits + 7) / 8;
        let mut rng = rand::rngs::OsRng;
        let mut bytes = vec![0u8; n_bytes];
        rng.fill_bytes(&mut bytes);

        bytes
            .iter()
            .flat_map(|&byte| (0..8).rev().map(move |i| (byte >> i) & 1))
            .take(n_bits)
            .map(|b| b as u8)
            .collect()
    }

    /// Simulate classical approximation using Box-Muller transform.
    ///
    /// The Box-Muller transform converts uniform random samples to
    /// Gaussian-distributed samples, modeling the vacuum fluctuations
    /// observed in quantum optics experiments.
    ///
    /// # Arguments
    ///
    /// * `n_bits` - Number of bits to generate
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - Vector of bits (0 or 1)
    pub fn simulate_classical(&self, n_bits: usize) -> Vec<u8> {
        let mut bits = Vec::with_capacity(n_bits);

        for _ in 0..n_bits {
            // Get two uniform random numbers in (0, 1)
            let u1 = self.random_unit_float();
            let u2 = self.random_unit_float();

            // Box-Muller transform to get Gaussian sample
            let z0 = (-2.0 * u1.ln()).sqrt() * (2.0 * std::f64::consts::PI * u2).cos();
            let fluctuation = z0 * 1.0;

            // Threshold to get bit
            bits.push(if fluctuation > 0.0 { 1 } else { 0 });
        }

        bits
    }

    /// Get a random float in [0, 1) using OS secure randomness.
    fn random_unit_float(&self) -> f64 {
        let mut rng = rand::rngs::OsRng;
        let mut bytes = [0u8; 8];
        rng.fill_bytes(&mut bytes);
        let val = u64::from_be_bytes(bytes);
        (val as f64) / (u64::MAX as f64)
    }

    /// Simulate with vacuum fluctuation noise using secure random.
    ///
    /// This is the recommended simulation method that combines
    /// secure randomness with quantum-appropriate noise modeling.
    ///
    /// # Arguments
    ///
    /// * `n_bits` - Number of bits to generate
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - Vector of bits (0 or 1)
    pub fn simulate_quantum(&self, n_bits: usize) -> Vec<u8> {
        // Mix of secure random and classical noise simulation
        let secure_bits = self.simulate(n_bits);
        let classical_bits = self.simulate_classical(n_bits);

        // Combine using beam splitter operator (XOR for mixing)
        secure_bits
            .iter()
            .zip(classical_bits.iter())
            .map(|(s, c)| s ^ c)
            .collect()
    }
}

impl Default for BeamSplitter {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_beam_splitter_creation() {
        let bs = BeamSplitter::new();
        assert!((bs.ratio() - 0.5).abs() < 0.01);
    }

    #[test]
    fn test_beam_splitter_custom_ratio() {
        let bs = BeamSplitter::with_ratio(0.3);
        assert!(bs.ratio() > 0.29 && bs.ratio() < 0.31);
    }

    #[test]
    fn test_beam_splitter_operator() {
        let bs = BeamSplitter::new();
        let op = bs.operator();

        // Verify operator properties (unitarity)
        let a = op[0][0];
        let b = op[0][1];
        let c = op[1][0];
        let d = op[1][1];

        // |a|^2 + |b|^2 = 1 (first row normalized)
        assert!((a * a + b * b - 1.0).abs() < 0.0001);
        // |c|^2 + |d|^2 = 1 (second row normalized)
        assert!((c * c + d * d - 1.0).abs() < 0.0001);
    }

    #[test]
    fn test_simulate_length() {
        let bs = BeamSplitter::new();
        let bits = bs.simulate(1000);
        assert_eq!(bits.len(), 1000);
    }

    #[test]
    fn test_simulate_classical_length() {
        let bs = BeamSplitter::new();
        let bits = bs.simulate_classical(1000);
        assert_eq!(bits.len(), 1000);
    }

    #[test]
    fn test_simulate_values() {
        let bs = BeamSplitter::new();
        let bits = bs.simulate(1000);

        // All bits should be 0 or 1
        for bit in &bits {
            assert!(*bit == 0 || *bit == 1);
        }
    }

    #[test]
    fn test_simulate_classical_values() {
        let bs = BeamSplitter::new();
        let bits = bs.simulate_classical(1000);

        // All bits should be 0 or 1
        for bit in &bits {
            assert!(*bit == 0 || *bit == 1);
        }
    }

    #[test]
    fn test_simulate_quantum_length() {
        let bs = BeamSplitter::new();
        let bits = bs.simulate_quantum(1000);
        assert_eq!(bits.len(), 1000);
    }

    #[test]
    fn test_simulate_quantum_values() {
        let bs = BeamSplitter::new();
        let bits = bs.simulate_quantum(1000);

        // All bits should be 0 or 1
        for bit in &bits {
            assert!(*bit == 0 || *bit == 1);
        }
    }

    #[test]
    fn test_feigenbaum_modulation() {
        // Verify that Feigenbaum constant is applied to ratio
        let bs_default = BeamSplitter::new();
        let bs_custom = BeamSplitter::with_ratio(0.5);

        // The ratio should be slightly different from 0.5 due to Feigenbaum modulation
        let diff = (bs_default.ratio() - 0.5).abs();
        assert!(diff > 0.0, "Feigenbaum modulation should alter ratio");
    }

    #[test]
    fn test_multiple_simulations_different() {
        let bs = BeamSplitter::new();

        // Multiple simulations should produce different results (not deterministic)
        let bits1 = bs.simulate(100);
        let bits2 = bs.simulate(100);

        // They should be different (with very high probability)
        let different = bits1
            .iter()
            .zip(bits2.iter())
            .filter(|(&a, &b)| a != b)
            .count();
        assert!(different > 10, "Simulations should differ");
    }
}
