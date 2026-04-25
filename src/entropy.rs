//! Entropy Pool Management - Forward-secure SHA-256 entropy extraction.
//!
//! This module implements forward-secure entropy pool management for the
//! Frankenstein Quixotry RNG. The entropy pool provides a secure buffer
//! for quantum entropy before extraction and key derivation.
//!
//! The forward-secure design ensures that compromising a single key does
//! not compromise past or future keys.

use hmac::{Hmac, Mac};
use rand::RngCore;
use sha2::Sha256;

/// Type alias for HMAC-SHA256
type HmacSha256 = Hmac<Sha256>;

/// Entropy Pool - Forward-secure entropy management
///
/// Manages a pool of entropy bytes with forward-secure key derivation.
/// Each call to derive_key produces a different key based on a key ID,
/// ensuring forward security (compromising key N doesn't reveal key N-1).
#[derive(Clone)]
pub struct EntropyPool {
    /// Entropy pool buffer
    pool: Vec<u8>,
    /// Target pool size in bytes
    pool_size: usize,
}

impl EntropyPool {
    /// Create a new entropy pool with specified size.
    ///
    /// # Arguments
    ///
    /// * `pool_size` - Target size for the entropy pool in bytes
    ///
    /// # Example
    ///
    /// ```
    /// let pool = EntropyPool::new(1024);
    /// ```
    pub fn new(pool_size: usize) -> Self {
        EntropyPool {
            pool: Vec::with_capacity(pool_size),
            pool_size,
        }
    }

    /// Replenish the entropy pool with new entropy data.
    ///
    /// Appends entropy to the pool, truncating if it exceeds pool_size.
    ///
    /// # Arguments
    ///
    /// * `entropy` - Raw entropy bytes to add to the pool
    pub fn replenish(&mut self, entropy: &[u8]) {
        // Add new entropy to pool
        self.pool.extend_from_slice(entropy);

        // Truncate to pool_size if needed
        if self.pool.len() > self.pool_size {
            self.pool.truncate(self.pool_size);
        }
    }

    /// Extract bits from the entropy pool.
    ///
    /// Uses secure random to select bits from the pool. If the pool
    /// doesn't have enough entropy, it is replenished first.
    ///
    /// # Arguments
    ///
    /// * `num_bits` - Number of bits to extract
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - Extracted bits (0 or 1 values)
    pub fn extract_bits(&mut self, num_bits: usize) -> Vec<u8> {
        // Ensure pool has enough entropy
        let min_bytes = (num_bits + 7) / 8;
        if self.pool.len() < min_bytes {
            // We need to generate more entropy - use secure random as supplement
            let mut rng = rand::rngs::OsRng;
            let needed = min_bytes - self.pool.len();
            let mut supplement = vec![0u8; needed];
            rng.fill_bytes(&mut supplement);
            self.pool.extend_from_slice(&supplement);
        }

        // Use secure random to select bits from pool
        let mut rng = rand::rngs::OsRng;
        let mut bytes = vec![0u8; (num_bits + 7) / 8];
        rng.fill_bytes(&mut bytes);

        // Mix pool content with random selection
        let mut bits = Vec::with_capacity(num_bits);
        for i in 0..num_bits {
            let byte_idx = i / 8;
            let bit_idx = i % 8;
            let pool_idx = (i + rng.next_u32() as usize) % self.pool.len().max(1);

            // XOR pool byte with random byte for extraction
            let bit = (bytes[byte_idx] >> bit_idx) & 1;
            let pool_bit = (self.pool[pool_idx] >> bit_idx) & 1;
            bits.push(bit ^ pool_bit);
        }

        bits
    }

    /// Derive a forward-secure key using HMAC-SHA256.
    ///
    /// Each key_id produces a different key, ensuring forward security:
    /// compromising key N does not reveal key N-1.
    ///
    /// # Arguments
    ///
    /// * `key_id` - Unique identifier for this key derivation
    ///
    /// # Returns
    ///
    /// * `Vec<u8>` - Derived key (32 bytes)
    pub fn derive_key(&self, key_id: u64) -> Vec<u8> {
        // Create HMAC with pool content as key
        let mut mac =
            HmacSha256::new_from_slice(&self.pool).expect("HMAC can take key of any size");

        // Update with key_id for forward-secure derivation
        mac.update(&key_id.to_le_bytes());

        // Also include pool length and a constant for domain separation
        mac.update(&(self.pool.len() as u64).to_le_bytes());
        mac.update(b"Frankenstein Quixotry RNG v1.0 - Forward Secure Key");

        let result = mac.finalize();
        result.into_bytes().to_vec()
    }

    /// Get current pool fill level.
    ///
    /// # Returns
    ///
    /// * `usize` - Current number of bytes in the pool
    pub fn pool_level(&self) -> usize {
        self.pool.len()
    }

    /// Check if pool needs replenishment.
    ///
    /// # Returns
    ///
    /// * `bool` - True if pool has less than 25% of target size
    pub fn needs_replenishment(&self) -> bool {
        self.pool.len() < self.pool_size / 4
    }

    /// Clear the entropy pool.
    pub fn clear(&mut self) {
        // Zero out the pool before clearing for security
        for byte in &mut self.pool {
            *byte = 0;
        }
        self.pool.clear();
    }
}

impl Default for EntropyPool {
    /// Default pool size of 1024 bytes (8192 bits)
    fn default() -> Self {
        Self::new(1024)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_entropy_pool_creation() {
        let pool = EntropyPool::new(1024);
        assert_eq!(pool.pool_level(), 0);
        assert_eq!(pool.pool_size, 1024);
    }

    #[test]
    fn test_entropy_pool_replenish() {
        let mut pool = EntropyPool::new(100);
        let entropy = vec![1u8, 2, 3, 4, 5];
        pool.replenish(&entropy);
        assert_eq!(pool.pool_level(), 5);
    }

    #[test]
    fn test_entropy_pool_truncate() {
        let mut pool = EntropyPool::new(10);
        let entropy = vec![1u8; 20];
        pool.replenish(&entropy);
        assert_eq!(pool.pool_level(), 10);
    }

    #[test]
    fn test_extract_bits_length() {
        let mut pool = EntropyPool::new(1024);
        let bits = pool.extract_bits(100);
        assert_eq!(bits.len(), 100);
    }

    #[test]
    fn test_extract_bits_values() {
        let mut pool = EntropyPool::new(1024);
        let bits = pool.extract_bits(1000);
        for bit in &bits {
            assert!(*bit == 0 || *bit == 1);
        }
    }

    #[test]
    fn test_derive_key_length() {
        let pool = EntropyPool::new(1024);
        let key = pool.derive_key(0);
        assert_eq!(key.len(), 32); // SHA-256 produces 32 bytes
    }

    #[test]
    fn test_derive_key_different_ids() {
        let pool = EntropyPool::new(1024);
        let key0 = pool.derive_key(0);
        let key1 = pool.derive_key(1);
        // Keys should be different for different IDs
        assert_ne!(key0, key1);
    }

    #[test]
    fn test_needs_replenishment() {
        let mut pool = EntropyPool::new(1024);
        assert!(pool.needs_replenishment());

        // Fill to 20% - still needs replenishment
        pool.replenish(&vec![1u8; 200]);
        assert!(pool.needs_replenishment());

        // Fill to 30% - now above the 25% threshold
        pool.replenish(&vec![1u8; 100]);
        assert!(!pool.needs_replenishment());
    }

    #[test]
    fn test_clear() {
        let mut pool = EntropyPool::new(100);
        pool.replenish(&vec![1u8; 50]);
        assert_eq!(pool.pool_level(), 50);
        pool.clear();
        assert_eq!(pool.pool_level(), 0);
    }

    #[test]
    fn test_default_pool_size() {
        let pool = EntropyPool::default();
        assert_eq!(pool.pool_size, 1024);
    }
}
