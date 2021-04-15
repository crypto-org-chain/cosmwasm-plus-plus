/// u64 is enough for subscription contract
#[derive(Default, PartialEq, Eq, PartialOrd, Ord)]
pub struct BitSet(pub u64);
const WORD_SIZE: usize = 64;

#[derive(Debug, PartialEq, Eq)]
pub struct OutOfBoundError;

impl BitSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, i: usize) -> Result<(), OutOfBoundError> {
        if i >= WORD_SIZE {
            Err(OutOfBoundError)
        } else {
            self.0 |= 1 << i;
            Ok(())
        }
    }

    pub fn clear(&mut self, i: usize) -> Result<(), OutOfBoundError> {
        if i >= WORD_SIZE {
            Err(OutOfBoundError)
        } else {
            self.0 &= !(1 << i);
            Ok(())
        }
    }

    pub fn test(&self, i: usize) -> Result<bool, OutOfBoundError> {
        if i >= WORD_SIZE {
            Err(OutOfBoundError)
        } else {
            Ok(self.0 & (1 << i) != 0)
        }
    }

    pub fn len(&self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn inplace_union(&mut self, other: BitSet) {
        self.0 |= other.0;
    }

    pub fn intersection(&self, other: BitSet) -> BitSet {
        BitSet(self.0 & other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bitset_operations_work() {
        let mut set = BitSet::new();
        assert_eq!(set.len(), 0);

        // OutOfBoundError
        assert_eq!(set.test(64), Err(OutOfBoundError));
        assert_eq!(set.set(64), Err(OutOfBoundError));
        assert_eq!(set.clear(64), Err(OutOfBoundError));

        assert_eq!(set.test(0), Ok(false));
        assert_eq!(set.test(63), Ok(false));

        assert_eq!(set.set(0), Ok(()));
        assert_eq!(set.set(63), Ok(()));

        assert_eq!(set.test(0), Ok(true));
        assert_eq!(set.test(63), Ok(true));

        assert_eq!(set.len(), 2);
    }
}
