use std::mem;
use std::num::NonZeroUsize;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// u64 is enough for subscription contract
type Num = u64;

/// non-emptiness is ensured by smart constructor.
#[derive(
    Debug, Copy, Clone, Default, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema,
)]
#[serde(transparent)]
pub struct NonEmptyBitSet(pub Num);
const NUM_SIZE: usize = mem::size_of::<Num>() * 8;

#[derive(Debug, PartialEq, Eq)]
pub struct OutOfBoundError;

impl NonEmptyBitSet {
    #[inline]
    pub const fn new(i: BitSetIndex) -> Self {
        Self(1 << i.get())
    }

    /// Construct with iterator
    pub fn from_items<I: IntoIterator<Item = usize>>(iter: I) -> Option<Self> {
        let mut set = Self(0);
        for i in iter {
            set.set(BitSetIndex::new(i)?);
        }
        if set.0 != 0 {
            Some(set)
        } else {
            None
        }
    }

    /// Construct by merging multiple bitsets
    pub fn from_bitsets<I: IntoIterator<Item = NonEmptyBitSet>>(iter: I) -> Option<Self> {
        let mut set = Self(0);
        for i in iter {
            set.inplace_union(i);
        }
        if set.0 != 0 {
            Some(set)
        } else {
            None
        }
    }

    /// Build bitset from range, only for compile time call(const).
    /// Use `from_items` instead for non-const situation.
    ///
    /// PANIC: if input is out of range or empty
    pub const fn from_range(start: usize, end: usize) -> Self {
        let start = BitSetIndex::new(start).unwrap();
        let end = BitSetIndex::new(end).unwrap();
        let mut set = 0;
        let mut idx = start.0;
        while idx <= end.0 {
            set |= 1 << idx;
            idx += 1;
        }
        if set == 0 {
            panic!("bitset is empty");
        }
        NonEmptyBitSet(set)
    }

    pub fn set(&mut self, i: BitSetIndex) {
        self.0 |= 1 << i.get();
    }

    pub fn test(&self, i: BitSetIndex) -> bool {
        self.0 & (1 << i.get()) != 0
    }

    #[inline]
    pub fn len(self) -> NonZeroUsize {
        // SAFETY: guaranteed by NonEmptyBitSet constructor
        unsafe { NonZeroUsize::new_unchecked(self.0.count_ones() as usize) }
    }

    #[inline]
    pub fn inplace_union(&mut self, other: NonEmptyBitSet) {
        self.0 |= other.0;
    }

    /// Return the index of lowest set bit,
    /// return None if empty
    #[inline]
    pub fn min(self) -> BitSetIndex {
        // SAFETY: non-empty guarantee
        BitSetIndex(self.0.trailing_zeros() as u8)
    }

    /// Return the index of highest set bit,
    /// return None if empty
    #[inline]
    pub fn max(self) -> BitSetIndex {
        // SAFETY: non-empty guarantee
        BitSetIndex(63u8 - self.0.leading_zeros() as u8)
    }

    /// Returns the next bit set from the specified index,
    /// including possibly the current index
    pub fn next_set(self, i: BitSetIndex) -> Option<BitSetIndex> {
        let n = self.0 >> i.0;
        if n != 0 {
            // SAFETY: the add result never exceed 64
            Some(BitSetIndex(i.0 + n.trailing_zeros() as u8))
        } else {
            None
        }
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

    #[inline]
    pub const fn get(self) -> usize {
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
        let mut set = NonEmptyBitSet::new(BitSetIndex::new(0).unwrap());
        assert_eq!(set.len().get(), 1);
        assert_eq!(set.min().get(), 0);
        assert_eq!(set.max().get(), 0);

        let v0 = BitSetIndex::new(0).unwrap();
        let v63 = BitSetIndex::new(63).unwrap();

        assert!(set.test(v0));
        assert!(!set.test(v63));

        set.set(v63);

        assert!(set.test(v63));

        assert_eq!(set.len().get(), 2);
        assert_eq!(set.min().get(), 0);
        assert_eq!(set.max().get(), 63);
    }

    #[test]
    fn bitset_from_range() {
        const SET: NonEmptyBitSet = NonEmptyBitSet::from_range(1, 12);
        assert_eq!(SET.max().get(), 12);
        assert_eq!(SET.min().get(), 1);
        assert_eq!(SET.len().get(), 12);
    }

    #[test]
    fn bitset_next_set() {
        let mut set = NonEmptyBitSet::new(BitSetIndex::new(0).unwrap());
        set.set(BitSetIndex::new(10).unwrap());

        assert_eq!(
            set.next_set(BitSetIndex::new(0).unwrap()),
            Some(BitSetIndex::new(0).unwrap())
        );
        assert_eq!(
            set.next_set(BitSetIndex::new(1).unwrap()),
            Some(BitSetIndex::new(10).unwrap())
        );
        assert_eq!(set.next_set(BitSetIndex::new(11).unwrap()), None);
    }
}
