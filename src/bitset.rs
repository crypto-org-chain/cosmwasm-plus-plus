use std::mem;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// u64 is enough for subscription contract
type Num = u64;

#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct BitSet(pub Num);
const NUM_SIZE: usize = mem::size_of::<Num>() * 8;

#[derive(Debug, PartialEq, Eq)]
pub struct OutOfBoundError;

impl BitSet {
    #[inline]
    pub const fn new() -> Self {
        Self(0)
    }

    /// Build bitset from range, can be called at compile time.
    ///
    /// PANIC: if input is out of range
    pub const fn from_range(start: usize, end: usize) -> Self {
        let start = BitSetIndex::new(start).unwrap();
        let end = BitSetIndex::new(end).unwrap();
        let mut set = Self::new();
        let mut idx = start.0;
        while idx <= end.0 {
            // SAFETY: bound checked above
            set = set.copy_set(BitSetIndex(idx));
            idx += 1;
        }
        set
    }

    #[inline]
    pub fn full() -> Self {
        Self(Num::MAX)
    }

    pub fn set(&mut self, i: BitSetIndex) {
        self.0 |= 1 << i.value();
    }

    const fn copy_set(self, i: BitSetIndex) -> Self {
        Self(self.0 | 1 << i.value())
    }

    pub fn clear(&mut self, i: BitSetIndex) {
        self.0 &= !(1 << i.value());
    }

    pub fn test(&self, i: BitSetIndex) -> bool {
        self.0 & (1 << i.value()) != 0
    }

    #[inline]
    pub fn len(self) -> usize {
        self.0.count_ones() as usize
    }

    #[inline]
    pub fn is_empty(self) -> bool {
        self.len() == 0
    }

    #[inline]
    pub fn inplace_union(&mut self, other: BitSet) {
        self.0 |= other.0;
    }

    #[inline]
    pub fn intersection(&self, other: BitSet) -> BitSet {
        BitSet(self.0 & other.0)
    }

    /// Return the indics of lowest set bit and highest set bit.
    /// return None if empty
    pub fn bound(self) -> Option<(BitSetIndex, BitSetIndex)> {
        Some((self.min()?, self.max()?))
    }

    /// Return the index of lowest set bit,
    /// return None if empty
    pub fn min(self) -> Option<BitSetIndex> {
        BitSetIndex::new(self.0.trailing_zeros() as usize)
    }

    /// Return the index of highest set bit,
    /// return None if empty
    pub fn max(self) -> Option<BitSetIndex> {
        // SAFETY: safe
        63u8.checked_sub(self.0.leading_zeros() as u8)
            .map(BitSetIndex)
    }
}

/// An u8 which is ensured to be smaller than 64 by construction
#[derive(
    Copy, Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct BitSetIndex(pub u8);

impl BitSetIndex {
    pub const fn new(u: usize) -> Option<Self> {
        if u < NUM_SIZE {
            Some(BitSetIndex(u as u8))
        } else {
            None
        }
    }

    pub fn unsafe_new(u: u8) -> Self {
        Self(u)
    }

    #[inline]
    pub const fn value(self) -> usize {
        self.0 as usize
    }

    pub fn checked_add(self, other: Self) -> Option<Self> {
        self.0
            .checked_add(other.0)
            .and_then(|v| Self::new(v as usize))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitset_index_bound_check() {
        BitSetIndex::new(0).unwrap();
        BitSetIndex::new(63).unwrap();
        assert!(BitSetIndex::new(64).is_none());
    }

    #[test]
    fn bitset_operations_work() {
        let mut set = BitSet::new();

        assert_eq!(set.len(), 0);
        assert!(set.is_empty());
        assert_eq!(set.min(), None);
        assert_eq!(set.max(), None);

        let v0 = BitSetIndex::new(0).unwrap();
        let v63 = BitSetIndex::new(63).unwrap();

        assert!(!set.test(v0));
        assert!(!set.test(v63));

        set.set(v0);
        set.set(v63);

        assert!(set.test(v0));
        assert!(set.test(v63));

        assert_eq!(set.len(), 2);
        assert_eq!(set.min().unwrap().value(), 0);
        assert_eq!(set.max().unwrap().value(), 63);
    }

    #[test]
    fn bitset_from_range() {
        const SET: BitSet = BitSet::from_range(1, 12);
        assert_eq!(SET.max().unwrap().value(), 12);
        assert_eq!(SET.min().unwrap().value(), 1);
        assert_eq!(SET.len(), 12);
    }

    #[test]
    fn bitset_iterator() {}
}
