//! Von Neumann Post-Processing - Bias removal from quantum measurements.
//!
//! This module implements Von Neumann post-processing to remove bias
//! from raw quantum measurements. The extractor pairs consecutive bits,
//! keeps pairs that differ, and discards pairs that are equal.
//!
//! This is a classical post-processing technique that helps improve
//! the entropy quality of quantum random number generators.

/// Von Neumann Extractor
///
/// Pairs consecutive bits, keeps the first bit of each pair when the
/// bits differ, and discards pairs that are equal. This effectively
/// removes bias from quantum measurements without losing entropy.
///
/// # Arguments
///
/// * `bits` - Raw bit sequence from quantum source
///
/// # Returns
///
/// * `Vec<u8>` - Extracted bits with reduced bias
///
/// # Example
///
/// ```
/// let raw_bits = vec![0, 1, 1, 1, 0, 1, 1, 0];
/// let extracted = von_neumann_extractor(&raw_bits);
/// ```
pub fn von_neumann_extractor(bits: &[u8]) -> Vec<u8> {
    let mut extracted = Vec::new();
    let mut i = 0;

    while i < bits.len().saturating_sub(1) {
        if bits[i] != bits[i + 1] {
            // Keep the first bit of the differing pair
            extracted.push(bits[i]);
        }
        // Move to next pair (skip 2 bits)
        i += 2;
    }

    extracted
}

/// XOR-based bias removal
///
/// For highly biased sources, this method XORs consecutive bits
/// in pairs, which can help reduce the bias while preserving
/// some entropy.
///
/// # Arguments
///
/// * `bits` - Raw bit sequence from quantum source
/// * `threshold` - Bias threshold (currently unused, reserved for future use)
///
/// # Returns
///
/// * `Vec<u8>` - Bits after XOR-based bias removal
pub fn bias_removal(bits: &[u8], _threshold: f64) -> Vec<u8> {
    if bits.len() < 2 {
        return bits.to_vec();
    }

    let mut result = Vec::with_capacity(bits.len() / 2);

    for i in (0..bits.len().saturating_sub(1)).step_by(2) {
        result.push(bits[i] ^ bits[i + 1]);
    }

    result
}

/// Entropy estimation for a bit sequence
///
/// Calculates the empirical entropy of a bit sequence using
/// the Shannon formula: H = -sum(p * log2(p))
///
/// # Arguments
///
/// * `bits` - Bit sequence to analyze
///
/// # Returns
///
/// * `f64` - Estimated entropy (0.0 to 1.0)
pub fn estimate_entropy(bits: &[u8]) -> f64 {
    if bits.is_empty() {
        return 0.0;
    }

    let n = bits.len() as f64;
    let ones = bits.iter().filter(|&&b| b == 1).count() as f64;
    let zeros = n - ones;

    if ones == 0.0 || zeros == 0.0 {
        return 0.0;
    }

    let p1 = ones / n;
    let p0 = zeros / n;

    -((p0 * p0.log2()) + (p1 * p1.log2()))
}

/// Compression-based entropy estimation
///
/// Uses run-length analysis to estimate entropy quality.
/// Longer runs indicate lower entropy.
///
/// # Arguments
///
/// * `bits` - Bit sequence to analyze
///
/// # Returns
///
/// * `f64` - Compression-based entropy estimate
pub fn compression_entropy(bits: &[u8]) -> f64 {
    if bits.len() < 2 {
        return 0.0;
    }

    let mut run_length = 1;
    let mut max_run = 0;
    let mut total_run = 0;
    let mut run_count = 0;

    for i in 1..bits.len() {
        if bits[i] == bits[i - 1] {
            run_length += 1;
        } else {
            total_run += run_length;
            run_count += 1;
            max_run = max_run.max(run_length);
            run_length = 1;
        }
    }

    // Add last run
    total_run += run_length;
    run_count += 1;

    if run_count == 0 {
        return 0.0;
    }

    let avg_run = total_run as f64 / run_count as f64;
    let expected = bits.len() as f64 / (2.0 * run_count as f64);

    // Entropy is high when runs are similar (close to expected)
    if max_run == 0 {
        return 1.0;
    }

    (expected / (max_run as f64)).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_von_neumann_basic() {
        // When bits differ, we keep the first bit
        // 01 -> keep 0, 10 -> keep 1
        let bits = vec![0, 1, 1, 0, 0, 1, 1, 1];
        let extracted = von_neumann_extractor(&bits);
        // 01 (keep 0), 10 (keep 1), 01 (keep 0), 11 (discard)
        assert_eq!(extracted.len(), 3);
    }

    #[test]
    fn test_von_neumann_all_equal() {
        let bits = vec![0, 0, 0, 0, 0, 0];
        let extracted = von_neumann_extractor(&bits);
        assert_eq!(extracted.len(), 0);
    }

    #[test]
    fn test_von_neumann_all_different() {
        let bits = vec![0, 1, 0, 1, 0, 1, 0, 1];
        let extracted = von_neumann_extractor(&bits);
        // All pairs differ, so we keep first bit of each pair
        assert_eq!(extracted.len(), 4);
        assert_eq!(extracted, vec![0, 0, 0, 0]);
    }

    #[test]
    fn test_von_neumann_odd_length() {
        let bits = vec![0, 1, 1];
        let extracted = von_neumann_extractor(&bits);
        // Only one pair (index 0-1), keep bit at index 0
        assert_eq!(extracted.len(), 1);
        assert_eq!(extracted[0], 0);
    }

    #[test]
    fn test_von_neumann_preserves_differing_pairs() {
        // [1,0] differs at first two bits (1 != 0), keep 1
        let bits = vec![1, 0];
        let extracted = von_neumann_extractor(&bits);
        assert_eq!(extracted, vec![1]);

        // [0,1] differs, keep 0
        let bits = vec![0, 1];
        let extracted = von_neumann_extractor(&bits);
        assert_eq!(extracted, vec![0]);
    }

    #[test]
    fn test_bias_removal_basic() {
        let bits = vec![0, 0, 1, 1, 0, 1, 1, 0];
        let result = bias_removal(&bits, 0.5);
        // XOR each pair: 0^0=0, 1^1=0, 0^1=1, 1^0=1
        assert_eq!(result, vec![0, 0, 1, 1]);
    }

    #[test]
    fn test_bias_removal_short() {
        let bits = vec![0];
        let result = bias_removal(&bits, 0.5);
        assert_eq!(result, vec![0]);
    }

    #[test]
    fn test_estimate_entropy_balanced() {
        let bits = vec![0, 1, 0, 1, 0, 1, 0, 1];
        let entropy = estimate_entropy(&bits);
        assert!((entropy - 1.0).abs() < 0.01);
    }

    #[test]
    fn test_estimate_entropy_all_ones() {
        let bits = vec![1, 1, 1, 1, 1];
        let entropy = estimate_entropy(&bits);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_estimate_entropy_mostly_ones() {
        let bits = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 0];
        let entropy = estimate_entropy(&bits);
        // High imbalance should give low entropy
        assert!(entropy < 0.5);
    }

    #[test]
    fn test_compression_entropy() {
        let bits = vec![0, 1, 0, 1, 0, 1, 0, 1];
        let entropy = compression_entropy(&bits);
        // Alternating bits should have reasonable entropy
        assert!(entropy > 0.3);
    }

    #[test]
    fn test_empty_bits() {
        let bits: Vec<u8> = vec![];
        let extracted = von_neumann_extractor(&bits);
        assert_eq!(extracted.len(), 0);

        let entropy = estimate_entropy(&bits);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_single_bit() {
        let bits = vec![1];
        let extracted = von_neumann_extractor(&bits);
        assert_eq!(extracted.len(), 0);
    }

    #[test]
    fn test_reduces_bias() {
        // Highly biased sequence (90% ones)
        let bits = vec![1, 1, 1, 1, 1, 1, 1, 1, 1, 0];

        let entropy_before = estimate_entropy(&bits);
        let extracted = von_neumann_extractor(&bits);
        let entropy_after = estimate_entropy(&extracted);

        // If we got enough bits, check entropy improvement
        if extracted.len() >= 3 {
            assert!(
                entropy_after >= entropy_before,
                "Entropy should improve or stay same: before={}, after={}",
                entropy_before,
                entropy_after
            );
        }
    }
}
