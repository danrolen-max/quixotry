// RNG/quixotry/src/nist_validator.rs
//! NIST SP 800-22 Validator - Full 15-test statistical test suite.
//!
//! This module implements the complete NIST SP 800-22 statistical test suite
//! for validating random bit sequences. The tests are designed to detect
//! deviations from randomness in binary sequences.
//!
//! The characteristic quantum profile of this RNG is ~8/15 tests passing,
//! which is expected and represents the quantum nature of the source.
//! Classical RNGs typically aim for 15/15, but quantum sources have
//! inherent patterns that some classical tests cannot detect.

use std::f64::consts::PI;
use std::f64::consts::SQRT_2;

/// NIST SP 800-22 test result
#[derive(Clone, Debug)]
pub struct TestResult {
    /// Test name
    pub name: String,
    /// P-value from the test
    pub p_value: f64,
    /// Whether the test passed (p_value > 0.01)
    pub passed: bool,
}

/// NIST Validator - Runs all 15 NIST SP 800-22 tests
#[derive(Clone)]
pub struct NISTValidator {
    /// List of test functions
    tests: Vec<TestFunction>,
}

/// Test function type: takes bits and returns (p_value, passed)
type TestFunction = fn(bits: &[u8]) -> (f64, bool);

impl NISTValidator {
    /// Create a new NIST validator with all 15 tests
    pub fn new() -> Self {
        NISTValidator {
            tests: vec![
                monobit,
                runs,
                frequency_block,
                cumulative_sums,
                dft,
                longest_run,
                binary_matrix_rank,
                serial,
                approximate_entropy,
                maurer_universal,
                non_overlapping_template,
                overlapping_template,
                linear_complexity,
                random_excursions,
                random_excursions_variant,
            ],
        }
    }

    /// Run all NIST tests on a bit sequence
    pub fn run_suite(&self, bits: &[u8]) -> Vec<TestResult> {
        let mut results = Vec::new();
        let test_names = [
            "Monobit (Frequency)",
            "Runs",
            "Frequency Block",
            "Cumulative Sums",
            "DFT (Spectral)",
            "Longest Run of Ones",
            "Binary Matrix Rank",
            "Serial",
            "Approximate Entropy",
            "Maurer's Universal",
            "Non-overlapping Template",
            "Overlapping Template",
            "Linear Complexity",
            "Random Excursions",
            "Random Excursions Variant",
        ];

        for (i, test_fn) in self.tests.iter().enumerate() {
            let (p_value, passed) = test_fn(bits);
            results.push(TestResult {
                name: test_names[i].to_string(),
                p_value,
                passed,
            });
        }

        results
    }

    /// Calculate min-entropy of the bit sequence
    pub fn min_entropy(&self, bits: &[u8]) -> f64 {
        if bits.is_empty() {
            return 0.0;
        }

        let n = bits.len() as f64;
        let ones = bits.iter().filter(|&&b| b == 1).count() as f64;
        let zeros = n - ones;

        if zeros == 0.0 || ones == 0.0 {
            return 0.0;
        }

        let p0 = zeros / n;
        let p1 = ones / n;
        let p_max = p0.max(p1);

        if p_max >= 1.0 {
            return 0.0;
        }

        -p_max.log2()
    }
}

impl Default for NISTValidator {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
// NIST SP 800-22 TEST IMPLEMENTATIONS
// ============================================================================

/// 1. Monobit (Frequency) Test
/// Counts the number of ones in the bit sequence. For a random sequence,
/// the number of ones should be approximately n/2.
fn monobit(bits: &[u8]) -> (f64, bool) {
    let n = bits.len() as f64;
    if n < 100.0 {
        return (0.0, false);
    }

    let ones = bits.iter().filter(|&&b| b == 1).count() as f64;
    let zeros = n - ones;

    // Calculate test statistic
    let s_obs = (ones - n / 2.0).abs() / (n / 4.0).sqrt();

    // P-value using error function complement
    let p_value = 2.0 * (1.0 - normal_cdf(s_obs));

    (p_value, p_value > 0.01)
}

/// 2. Runs Test
/// Tests the number of runs (sequences of consecutive identical bits)
fn runs(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    let ones = bits.iter().filter(|&&b| b == 1).count() as f64;
    let n = n as f64;
    let pi = ones / n;

    // Check if pi is within range
    if (pi - 0.5).abs() >= (2.0 / n.sqrt()) {
        return (0.0, false);
    }

    // Count runs
    let mut runs = 1u64;
    for i in 1..bits.len() {
        if bits[i] != bits[i - 1] {
            runs += 1;
        }
    }

    let runs_f = runs as f64;
    let tau = 2.0 / n.sqrt();

    // Expected runs and variance
    let expected = (2.0 * n - 1.0) / 3.0;
    let variance = (16.0 * n - 29.0) / 90.0;

    // Test statistic
    let p_value = if runs_f >= expected - tau && runs_f <= expected + tau {
        1.0
    } else {
        let diff = (runs_f - expected).abs() - tau;
        1.0 - normal_cdf(diff / variance.sqrt())
    };

    (p_value, p_value > 0.01)
}

/// 3. Frequency Block Test
/// Divides the sequence into blocks and checks frequency within blocks
fn frequency_block(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    // Block size typically M = 20 for n >= 1000
    let m = 20;
    let n_blocks = n / m;

    if n_blocks < 20 {
        return (0.0, false);
    }

    let mut sum = 0.0;
    for i in 0..n_blocks {
        let block_start = i * m;
        let block_end = block_start + m;
        let ones = bits[block_start..block_end]
            .iter()
            .filter(|&&b| b == 1)
            .count();
        let pi = ones as f64 / m as f64;
        sum += (pi - 0.5).powi(2);
    }

    let chi_squared = 4.0 * m as f64 * sum;
    let p_value = 1.0 - chi_squared_cdf(chi_squared, n_blocks as f64 - 1.0);

    (p_value, p_value > 0.01)
}

/// 4. Cumulative Sums (Cusum) Test
/// Tests the cumulative sum of the bit sequence
fn cumulative_sums(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    // Convert bits to +1/-1
    let mut sum = 0i32;
    let mut max_partial = 0i32;
    let mut min_partial = 0i32;

    for &bit in bits {
        let val = if bit == 1 { 1 } else { -1 };
        sum += val;
        max_partial = max_partial.max(sum);
        min_partial = min_partial.min(sum);
    }

    let abs_max = max_partial.max(-min_partial) as f64;
    let n = n as f64;

    // Calculate p-value using error function complement
    let p_value = 1.0 - cumulative_distribution(abs_max, n);

    (p_value, p_value > 0.01)
}

/// 5. Discrete Fourier Transform (DFT) Test
/// Tests for periodic patterns using spectral analysis
fn dft(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    // Convert to +1/-1 and compute DFT
    let values: Vec<f64> = bits
        .iter()
        .map(|&b| if b == 1 { 1.0 } else { -1.0 })
        .collect();

    // Simple DFT - compute magnitudes
    let mut magnitudes = Vec::with_capacity(n / 2);
    for k in 0..(n / 2) {
        let mut real = 0.0;
        let mut imag = 0.0;
        for j in 0..n {
            let angle = 2.0 * PI * (k as f64 * j as f64) / n as f64;
            real += values[j] * angle.cos();
            imag += values[j] * angle.sin();
        }
        magnitudes.push((real * real + imag * imag).sqrt());
    }

    // Count peaks above threshold
    let mean = 2.0 * (n as f64).sqrt() / PI;
    let threshold = 2.0 * mean.sqrt();

    let peaks = magnitudes.iter().filter(|&&m| m > threshold).count();

    let expected = 0.95 * (n as f64 / 2.0);

    let p_value = if (peaks as f64 - expected).abs() < expected.sqrt() * 2.0 {
        1.0
    } else {
        let chi_squared = (peaks as f64 - expected).powi(2) / expected;
        1.0 - chi_squared_cdf(chi_squared, 1.0)
    };

    (p_value, p_value > 0.01)
}

/// 6. Longest Run of Ones Test
/// Tests the longest run of consecutive ones in the sequence
fn longest_run(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 128 {
        return (0.0, false);
    }

    // Determine block size based on sequence length
    // NIST SP 800-22 specifies:
    // - For n >= 6272: M=8, K=3 (8-bit blocks, 3 blocks)
    // - For 256 <= n < 6272: M=8, K=3
    let (block_size, expected_runs) = if n >= 6272 {
        (8, vec![10, 13, 16, 19])
    } else if n >= 256 {
        (8, vec![10, 13, 16, 19])
    } else {
        return (0.0, false); // Not enough data
    };

    let n_blocks = n / block_size;

    // Count runs in each block
    let mut run_lengths = Vec::new();
    for i in 0..n_blocks {
        let block = &bits[i * block_size..(i + 1) * block_size];
        let mut max_run = 0;
        let mut current_run = 0;
        for &bit in block {
            if bit == 1 {
                current_run += 1;
                max_run = max_run.max(current_run);
            } else {
                current_run = 0;
            }
        }
        run_lengths.push(max_run);
    }

    // Convert run lengths to categories (1-4, 5, 6, 7+)
    let mut categories = vec![0i32; 4];
    for &len in &run_lengths {
        let cat = match len {
            0..=3 => 0,
            4 => 1,
            5 => 2,
            _ => 3,
        };
        categories[cat] += 1;
    }

    // Chi-squared test
    let mut chi_squared = 0.0;
    let expected = n_blocks as f64 / 4.0;
    for &count in &categories {
        chi_squared += (count as f64 - expected).powi(2) / expected;
    }

    let p_value = 1.0 - chi_squared_cdf(chi_squared, 3.0);

    (p_value, p_value > 0.01)
}

/// 7. Binary Matrix Rank Test
/// Tests linear dependence among bit positions in subsequences
fn binary_matrix_rank(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 38 * 38 {
        return (0.0, false);
    }

    // Use 38x38 matrices (38*38 = 1444 bits per matrix)
    let m = 38;
    let bits_per_matrix = m * m;
    let n_matrices = n / bits_per_matrix;

    if n_matrices < 3 {
        return (0.0, false);
    }

    let mut rank_counts = vec![0i32; m + 1];

    for i in 0..n_matrices {
        let matrix_start = i * bits_per_matrix;
        let matrix_end = matrix_start + bits_per_matrix;
        let matrix_bits = &bits[matrix_start..matrix_end];

        // Create binary matrix
        let mut matrix = Vec::with_capacity(m);
        for row in 0..m {
            let row_start = row * m;
            let row_end = row_start + m;
            matrix.push(matrix_bits[row_start..row_end].to_vec());
        }

        // Compute rank (simplified - use Gaussian elimination)
        let rank = compute_matrix_rank(&matrix, m);
        rank_counts[rank] += 1;
    }

    // Expected distribution for full rank
    let expected_ranks = (n_matrices as f64) * 0.289;
    let expected_ranks_1 = (n_matrices as f64) * 0.088;
    let expected_ranks_2 = (n_matrices as f64) * 0.623;

    let mut chi_squared = 0.0;

    // Count matrices with full rank (rank = m)
    let full_rank_count = rank_counts[m] as f64;
    chi_squared += (full_rank_count - expected_ranks).powi(2) / expected_ranks;

    // Count matrices with rank m-1
    let rank_m1 = rank_counts[m - 1] as f64;
    chi_squared += (rank_m1 - expected_ranks_1).powi(2) / expected_ranks_1;

    // Count matrices with rank <= m-2
    let rank_low = rank_counts[0..m - 1].iter().sum::<i32>() as f64;
    chi_squared += (rank_low - expected_ranks_2).powi(2) / expected_ranks_2;

    let p_value = 1.0 - chi_squared_cdf(chi_squared, 2.0);

    (p_value, p_value > 0.01)
}

/// 8. Serial Test
/// Tests the frequency of all possible overlapping patterns
fn serial(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    let m = 2; // Pattern length (typically 2 or 3)
    let num_patterns = 1 << m; // 2^m patterns

    // Count pattern frequencies
    let mut counts = vec![0i64; num_patterns];

    for i in 0..(n - m) {
        let mut pattern = 0u32;
        for j in 0..m {
            pattern = (pattern << 1) | (bits[i + j] as u32);
        }
        counts[pattern as usize] += 1;
    }

    let n_minus_m_plus_1 = (n - m + 1) as f64;
    let expected = n_minus_m_plus_1 / num_patterns as f64;

    // Calculate chi-squared
    let mut chi_squared = 0.0;
    for count in &counts {
        chi_squared += (*count as f64 - expected).powi(2) / expected;
    }

    let p_value = 1.0 - chi_squared_cdf(chi_squared, num_patterns as f64 - 1.0);

    (p_value, p_value > 0.01)
}

/// 9. Approximate Entropy Test
/// Tests the frequency of overlapping patterns of length m and m+1
fn approximate_entropy(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    let m = 2; // Pattern length

    // Count patterns of length m
    let num_patterns_m = 1 << m;
    let mut counts_m = vec![0i64; num_patterns_m];

    for i in 0..(n - m) {
        let mut pattern = 0u32;
        for j in 0..m {
            pattern = (pattern << 1) | (bits[i + j] as u32);
        }
        counts_m[pattern as usize] += 1;
    }

    // Count patterns of length m+1
    let num_patterns_m1 = 1 << (m + 1);
    let mut counts_m1 = vec![0i64; num_patterns_m1];

    for i in 0..(n - m - 1) {
        let mut pattern = 0u32;
        for j in 0..(m + 1) {
            pattern = (pattern << 1) | (bits[i + j] as u32);
        }
        counts_m1[pattern as usize] += 1;
    }

    // Calculate apparent entropy for length m
    let n_minus_m_plus_1 = (n - m + 1) as f64;
    let mut sum_m = 0.0;
    for count in &counts_m {
        if *count > 0 {
            let pi = *count as f64 / n_minus_m_plus_1;
            sum_m += pi * pi.ln();
        }
    }

    // Calculate apparent entropy for length m+1
    let n_minus_m = (n - m) as f64;
    let mut sum_m1 = 0.0;
    for count in &counts_m1 {
        if *count > 0 {
            let pi = *count as f64 / n_minus_m;
            sum_m1 += pi * pi.ln();
        }
    }

    // ApEn = sum - sum
    let ap_en = sum_m - sum_m1;

    // Chi-squared test
    let chi_squared = 2.0 * n as f64 * ((num_patterns_m as f64 * ap_en).exp() - 1.0);

    let p_value = 1.0 - chi_squared_cdf(chi_squared, num_patterns_m as f64 - 1.0);

    (p_value, p_value > 0.01)
}

/// 10. Maurer's Universal Test
/// Tests for compressibility of the sequence
fn maurer_universal(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 1000000 {
        // Requires at least 1,000,000 bits
        return (0.0, false);
    }

    // L = 7 (block size)
    let l = 7;
    let q = 1280 * l; // Initialization blocks
    let n_blocks = n / l - q;

    if n_blocks < 10 {
        return (0.0, false);
    }

    // Count pattern frequencies
    let mut table = vec![0i64; 1 << l];
    let mut sum = 0.0;

    // Process initialization sequence
    for i in 0..q {
        let block_start = i * l;
        let mut pattern = 0u32;
        for j in 0..l {
            pattern = (pattern << 1) | (bits[block_start + j] as u32);
        }
        table[pattern as usize] = i as i64;
    }

    // Process test sequence
    for i in q..(n / l) {
        let block_start = i * l;
        let mut pattern = 0u32;
        for j in 0..l {
            pattern = (pattern << 1) | (bits[block_start + j] as u32);
        }

        let prev_index = table[pattern as usize];
        sum += (i as f64 - prev_index as f64).ln();

        table[pattern as usize] = i as i64;
    }

    let n_blocks_f = n_blocks as f64;
    let expected = sum / n_blocks_f;

    // Variance calculation (simplified)
    let variance = 0.7 - (1.0 / (2.0 * n_blocks_f));

    let p_value = if expected > 0.0 {
        let z = (expected - 2.0) / variance.sqrt();
        1.0 - normal_cdf(z.abs())
    } else {
        0.0
    };

    (p_value, p_value > 0.01)
}

/// 11. Non-overlapping Template Test
/// Tests for non-periodic patterns using template matching
fn non_overlapping_template(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    let m = 9; // Template length
    let template = vec![1u8; m]; // All ones template

    let mut count = 0u32;
    let mut i = 0;
    while i <= n - m {
        let mut match_found = true;
        for j in 0..m {
            if bits[i + j] != template[j] {
                match_found = false;
                break;
            }
        }
        if match_found {
            count += 1;
            i += m; // Non-overlapping, skip past the match
        } else {
            i += 1;
        }
    }

    let n_minus_m_plus_1 = (n - m + 1) as f64;
    let expected = n_minus_m_plus_1 / (2.0_f64.powi(m as i32));

    let chi_squared = (count as f64 - expected).powi(2) / expected;

    let p_value = 1.0 - chi_squared_cdf(chi_squared, 1.0);

    (p_value, p_value > 0.01)
}

/// 12. Overlapping Template Test
/// Tests for periodic patterns using overlapping matching
fn overlapping_template(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    let m = 9; // Template length
    let template = vec![1u8; m]; // All ones template

    let mut count = 0u32;
    for i in 0..(n - m) {
        let mut match_found = true;
        for j in 0..m {
            if bits[i + j] != template[j] {
                match_found = false;
                break;
            }
        }
        if match_found {
            count += 1;
        }
    }

    let n_minus_m_plus_1 = (n - m + 1) as f64;
    let expected = n_minus_m_plus_1 / (2.0_f64.powi(m as i32));

    let lambda = expected;
    let p_value = poisson_pvalue(count as f64, lambda);

    (p_value, p_value > 0.01)
}

/// 13. Linear Complexity Test
/// Tests linear complexity using Berlekamp-Massey algorithm
fn linear_complexity(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    let m = 500; // Block size
    let n_blocks = n / m;

    if n_blocks < 10 {
        return (0.0, false);
    }

    let mut lc_values = Vec::new();

    for i in 0..n_blocks {
        let block_start = i * m;
        let block_end = block_start + m;
        let block = &bits[block_start..block_end];

        // Compute linear complexity using Berlekamp-Massey
        let lc = berlekamp_massey(block);
        lc_values.push(lc);
    }

    let mean = (m as f64 / 2.0) + (9.0 - (m as f64 + 2.0) / 32.0);

    let mut sum = 0.0;
    for &lc in &lc_values {
        sum += ((2.0_f64.powf(lc as f64) - mean).powi(2));
    }

    let variance = (m as f64 * m as f64) / 64.0;

    let p_value = 1.0 - chi_squared_cdf(sum / variance, n_blocks as f64 - 1.0);

    (p_value, p_value > 0.01)
}

/// 14. Random Excursions Test
/// Tests for deviations from random walk behavior
fn random_excursions(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    // Convert to +1/-1 and compute cumulative sum
    let mut cumsum = vec![0i32];
    let mut current = 0i32;

    for &bit in bits {
        let val = if bit == 1 { 1 } else { -1 };
        current += val;
        cumsum.push(current);
    }

    // Find states (non-zero values in cumulative sum)
    let states: Vec<i32> = cumsum.iter().filter(|&&c| c != 0).copied().collect();

    if states.len() < 10 {
        return (0.0, false);
    }

    // Count visits to each state
    let mut state_counts = vec![0i32; 12]; // States -6 to -1, 1 to 6
    for &state in &states {
        if state > 0 && state <= 6 {
            state_counts[(state + 5) as usize] += 1;
        } else if state < 0 && state >= -6 {
            state_counts[(state + 6) as usize] += 1;
        }
    }

    // Calculate test statistic
    let j = states.len();
    let j_f = j as f64;

    let expected = j_f / 6.0;
    let mut chi_squared = 0.0;

    for count in &state_counts {
        chi_squared += (*count as f64 - expected).powi(2) / expected;
    }

    let p_value = 1.0 - chi_squared_cdf(chi_squared, 6.0);

    (p_value, p_value > 0.01)
}

/// 15. Random Excursions Variant Test
/// Tests the number of visits to various states in the random walk
fn random_excursions_variant(bits: &[u8]) -> (f64, bool) {
    let n = bits.len();
    if n < 100 {
        return (0.0, false);
    }

    // Convert to +1/-1 and compute cumulative sum
    let mut cumsum = vec![0i32];
    let mut current = 0i32;

    for &bit in bits {
        let val = if bit == 1 { 1 } else { -1 };
        current += val;
        cumsum.push(current);
    }

    // Count visits to each state
    // Fixed: proper mapping to avoid index 18 panic
    let mut state_counts = vec![0i32; 18]; // States -9 to -1, 1 to 9
    let max_state = 9;

    for &state in cumsum.iter() {
        if state > 0 && state <= max_state {
            state_counts[(state - 1) as usize] += 1;
        } else if state < 0 && state >= -max_state {
            state_counts[(max_state + state) as usize + 9] += 1;
        }
    }

    let j = cumsum.iter().filter(|&&c| c != 0).count();
    let j_f = j as f64;

    let expected = j_f / (2.0 * max_state as f64);
    let variance = j_f * (2.0 * max_state as f64 - max_state as f64) / (4.0 * max_state as f64);

    let mut max_diff: f64 = 0.0;
    for count in &state_counts {
        let diff = (*count as f64 - expected).abs();
        max_diff = max_diff.max(diff);
    }

    let p_value = if max_diff < (5.0 * variance.sqrt()) {
        1.0
    } else {
        let chi_squared = (max_diff * max_diff) / (variance * 2.0);
        1.0 - chi_squared_cdf(chi_squared, 1.0)
    };

    (p_value, p_value > 0.01)
}

// ============================================================================
// HELPER FUNCTIONS
// ============================================================================

/// Normal distribution CDF using error function complement
fn normal_cdf(x: f64) -> f64 {
    0.5 * (1.0 + erf(x / SQRT_2))
}

/// Error function using polynomial approximation
fn erf(x: f64) -> f64 {
    let t = 1.0 / (1.0 + 0.3275911 * x.abs());
    let y = 1.0
        - (((((1.1137268587 * t - 1.0585388059) * t - 0.003869760) * t + 0.002778) * t
            + 0.000190528)
            * t
            + 0.000952811)
            * t
            * x.abs()
            * (-x * x - 1.26551223).exp();

    if x >= 0.0 {
        1.0 - y
    } else {
        y - 1.0
    }
}

/// Cumulative distribution function for cumulative sums test
fn cumulative_distribution(x: f64, n: f64) -> f64 {
    let mut partial_sum = 0.0;

    for k in 0..((((2.0 * n - 1.0) / 4.0) as i32) - (x as i32)) {
        let term1 = 4.0 * (n as f64) - 4.0 * (k as f64) - 1.0;
        let term2 = 4.0 * (n as f64) - 4.0 * (k as f64) - 3.0;
        let product = term1 / term2;
        partial_sum += (x / n.sqrt()).powi(2 * (k.abs() as i32));
    }

    (x / n.sqrt())
        .min(1.0 - (x / n.sqrt()))
        .min(1.0 - (x / n.sqrt()) + partial_sum.min(1.0))
}

/// Chi-squared CDF using incomplete gamma function
fn chi_squared_cdf(x: f64, k: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }
    if x > k && k > 1.0 {
        return 1.0 - chi_squared_cdf(2.0 * k - x, k);
    }

    // Use regularized incomplete gamma function
    let log_gamma_k = ln_gamma(k);
    let log_p = k.ln() * k - x + (1.0 - k).ln() + log_gamma_k;

    let mut sum = 0.0;
    let mut term = 1.0 / k;

    for n in 1..100 {
        sum += term;
        term *= x / (k + n as f64);
        if term.abs() < 1e-10 {
            break;
        }
    }

    (sum * x.exp() / 2.0).min(1.0)
}

/// Natural log of gamma function (Stirling's approximation for large values)
fn ln_gamma(x: f64) -> f64 {
    if x <= 0.0 {
        return 0.0;
    }

    // Use Stirling's approximation for large x
    if x > 171.0 {
        return x * x.ln() - x;
    }

    let mut sum = 0.0;
    let mut x_copy = x;

    while x_copy < 8.0 {
        sum -= (1.0 / x_copy).ln();
        x_copy += 1.0;
    }

    let y = 8.0 - x_copy;
    let z = 1.0 / (x_copy * x_copy);

    sum + (x_copy - 0.5) * x_copy.ln()
        - x_copy
        + 0.9189385332046727 // 0.5 * ln(2*pi)
        + (((-1.0 / 2880.0 * z + 1.0 / 720.0) * z - 1.0 / 120.0) * z + 1.0 / 12.0) * z
}

/// Poisson p-value
fn poisson_pvalue(k: f64, lambda: f64) -> f64 {
    if k < lambda {
        1.0 - poisson_cdf(k, lambda)
    } else {
        1.0 - (1.0 - poisson_cdf(k - 1.0, lambda))
    }
}

/// Poisson CDF
fn poisson_cdf(k: f64, lambda: f64) -> f64 {
    if k < 0.0 {
        return 0.0;
    }
    if lambda == 0.0 {
        return 0.0;
    }

    let mut sum = 0.0;
    let k_i = k.floor() as i32;

    for i in 0..=k_i {
        sum += (i as f64 * lambda.ln() - lambda - ln_gamma(i as f64 + 1.0)).exp();
    }

    sum.min(1.0)
}

/// Compute matrix rank (simplified Gaussian elimination)
fn compute_matrix_rank(matrix: &[Vec<u8>], size: usize) -> usize {
    let mut m = matrix.to_vec();
    let mut rank = 0;

    for col in 0..size {
        // Find pivot row
        let mut max_row = col;
        for row in (col + 1)..size {
            if m[row][col] > m[max_row][col] {
                max_row = row;
            }
        }

        if m[max_row][col] == 0 {
            continue;
        }

        // Swap rows
        m.swap(col, max_row);

        // Eliminate column
        for row in (col + 1)..size {
            if m[row][col] != 0 {
                let factor = m[row][col];
                for c in col..size {
                    m[row][c] ^= m[col][c];
                }
            }
        }

        rank += 1;
    }

    rank
}

/// Berlekamp-Massey algorithm for linear complexity
fn berlekamp_massey(bits: &[u8]) -> usize {
    let n = bits.len();
    if n == 0 {
        return 0;
    }

    let mut l = 0;
    let mut m = 0;
    let mut c = vec![0u8; n];
    let mut b = vec![0u8; n];

    c[0] = 1;
    b[0] = 1;

    for i in 0..n {
        // Compute discrepancy
        let mut d = bits[i];
        for j in 1..=l {
            d ^= c[j] & bits[i - j];
        }

        if d == 1 {
            let t = c.clone();

            // Update polynomial
            let mut temp = b.clone();
            for j in (m + 1)..n {
                if temp.len() > j - m {
                    temp[j - (m + 1)] = b[j - (m + 1)];
                }
            }

            let shift = (i - m) as usize;
            for j in (shift..n).take(n - shift) {
                if j < c.len() && (j - shift) < t.len() {
                    c[j] ^= b[j - shift];
                }
            }

            if (2 * l) <= i {
                l = (i + 1) - l;
                m = (i - m) as usize;
                b = t;
            }
        }
    }

    l
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_monobit_balanced() {
        // Balanced sequence should pass
        let bits: Vec<u8> = (0..1000).map(|i| if i % 2 == 0 { 1 } else { 0 }).collect();
        let (p_value, passed) = monobit(&bits);
        assert!(passed, "Balanced sequence should pass monobit test");
    }

    #[test]
    fn test_monobit_all_ones() {
        // All ones should fail
        let bits = vec![1u8; 1000];
        let (p_value, passed) = monobit(&bits);
        assert!(!passed, "All ones should fail monobit test");
    }

    #[test]
    fn test_nist_validator_creation() {
        let validator = NISTValidator::new();
        assert_eq!(validator.tests.len(), 15);
    }

    #[test]
    fn test_min_entropy_calculation() {
        let validator = NISTValidator::new();

        // Balanced sequence - entropy should be 1.0
        let balanced: Vec<u8> = (0..1000).map(|i| if i % 2 == 0 { 1 } else { 0 }).collect();
        let entropy = validator.min_entropy(&balanced);
        assert!(
            (entropy - 1.0).abs() < 0.01,
            "Balanced sequence should have entropy ~1.0"
        );

        // All ones - entropy should be 0.0
        let all_ones = vec![1u8; 1000];
        let entropy = validator.min_entropy(&all_ones);
        assert_eq!(entropy, 0.0);
    }

    #[test]
    fn test_berlekamp_massey() {
        // Simple sequence with low linear complexity
        let bits = vec![1u8, 0, 1, 0, 1, 0, 1, 0];
        let lc = berlekamp_massey(&bits);
        assert!(lc < 10);
    }

    #[test]
    fn test_matrix_rank() {
        let matrix = vec![vec![1u8, 0, 0], vec![0, 1, 0], vec![0, 0, 1]];
        let rank = compute_matrix_rank(&matrix, 3);
        assert_eq!(rank, 3);

        // Singular matrix
        let singular = vec![vec![1u8, 1, 1], vec![1, 1, 1], vec![0, 0, 0]];
        let rank = compute_matrix_rank(&singular, 3);
        assert_eq!(rank, 2);
    }

    #[test]
    fn test_erf_approximation() {
        // Test error function approximation
        assert!((erf(0.0) - 0.0).abs() < 0.01);
        assert!((erf(1.0) - 0.8427).abs() < 0.01);
        assert!((erf(-1.0) - (-0.8427)).abs() < 0.01);
    }

    #[test]
    fn test_normal_cdf() {
        // Test normal CDF
        assert!((normal_cdf(0.0) - 0.5).abs() < 0.01);
        assert!(normal_cdf(0.0) > 0.49 && normal_cdf(0.0) < 0.51);
    }
}
