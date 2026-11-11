
//! A fast, non-cryptographic hasher for the hot maps of the emulator.
//!
//! The keys are mostly guest addresses and small tuples, so a multiply and a fold are enough.

use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

/// A `HashMap` that uses `QuickHasher`.
pub type QuickMap<K, V> = HashMap<K, V, BuildHasherDefault<QuickHasher>>;
/// A `HashSet` that uses `QuickHasher`.
pub type QuickSet<K> = HashSet<K, BuildHasherDefault<QuickHasher>>;

/// A large odd constant that mixes the bits on every multiplication.
const MULTIPLIER: u64 = 0xF135_7AEA_2E62_A9C5;

/// Hashes by adding each word to the state and multiplying it by `MULTIPLIER`.
#[derive(Default, Clone, Copy)]
pub struct QuickHasher {
    hash: u64,
}

impl QuickHasher {
    /// Mixes one 64-bit word into the state.
    fn add(&mut self, word: u64) {
        self.hash = self.hash.wrapping_add(word).wrapping_mul(MULTIPLIER);
    }
}

impl Hasher for QuickHasher {
    fn write(&mut self, bytes: &[u8]) {
        let (words, rest) = bytes.as_chunks::<8>();
        for word in words {
            self.add(u64::from_le_bytes(*word));
        }
        // The last partial word is padded with zeros.
        if !rest.is_empty() {
            let mut last = [0; 8];
            last[..rest.len()].copy_from_slice(rest);
            self.add(u64::from_le_bytes(last));
        }
    }

    fn write_u8(&mut self, value: u8) {
        self.add(value as u64);
    }

    fn write_u16(&mut self, value: u16) {
        self.add(value as u64);
    }

    fn write_u32(&mut self, value: u32) {
        self.add(value as u64);
    }

    fn write_u64(&mut self, value: u64) {
        self.add(value);
    }

    fn write_usize(&mut self, value: usize) {
        self.add(value as u64);
    }

    fn finish(&self) -> u64 {
        // The high bits of a product are the best mixed, so the 128-bit result is folded onto itself.
        // This lets the low bits, which pick the bucket, depend on every input bit.
        let wide = self.hash as u128 * MULTIPLIER as u128;
        wide as u64 ^ (wide >> 64) as u64
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::hash::{BuildHasher, Hash};

    /// Hashes a value with `QuickHasher`.
    fn hash(value: impl Hash) -> u64 {
        BuildHasherDefault::<QuickHasher>::default().hash_one(value)
    }

    /// Flipping any single bit of a key changes the hash.
    #[test]
    fn keys_that_differ_hash_apart() {
        let key = (Some(0x1234u32), 7u8, [0x0FFF_0FFFu32; 30]);
        assert_eq!(hash(key), hash(key));
        for word in 0..30 {
            for bit in [0, 13, 31] {
                let mut other = key;
                other.2[word] ^= 1 << bit;
                assert_ne!(hash(key), hash(other), "word {word} bit {bit}");
            }
        }
        assert_ne!(hash(key), hash((None::<u32>, 7u8, key.2)));
        assert_ne!(hash([1u8, 2, 3]), hash([1u8, 2, 4]));
        assert_ne!(hash(&[0u8; 9][..]), hash(&[0u8, 0, 0, 0, 0, 0, 0, 0, 1][..]));
    }

    /// Keys that are 4096 bytes apart still fill many buckets.
    #[test]
    fn aligned_keys_spread_over_the_buckets() {
        let buckets: QuickSet<u64> = (0..256usize).map(|i| hash(0x7F00_0000_0000usize + i * 4096) & 255).collect();
        assert!(buckets.len() > 150, "{} buckets of 256", buckets.len());
    }

    /// Consecutive addresses fill many buckets, at the start, the middle and the end of the memory of the 3DS.
    #[test]
    fn consecutive_addresses_spread_over_the_buckets() {
        for base in [0x0810_0000u32, 0x1400_0000, 0x1FF8_0000] {
            let buckets: QuickSet<u64> = (0..256u32).map(|i| hash(base + i) & 255).collect();
            assert!(buckets.len() > 128, "{} buckets of 256 from 0x{base:08X}", buckets.len());
        }
    }
}
