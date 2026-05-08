//! Efficient bitset-based storage for box positions.

use std::hash::{Hash, Hasher};

use crate::math::IVector2;

/// Maximum map dimensions supported (64x64 = 4096 positions).
const MAX_POSITIONS: usize = 4096;
const BITS_PER_U64: usize = 64;
const NUM_U64S: usize = MAX_POSITIONS / BITS_PER_U64; // 64

/// A bitset-based collection of box positions.
///
/// This is more efficient than `HashSet<IVector2>` because:
/// - Smaller memory footprint (512 bytes vs ~48+ bytes per box)
/// - Faster hashing (no sorting needed, just hash the raw bits)
/// - Cache-friendly (contiguous memory)
/// - Deterministic iteration order
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct BoxSet {
    bits: [u64; NUM_U64S],
    width: i32,
    count: usize,
}

impl BoxSet {
    /// Creates a new empty `BoxSet` with the given map width.
    pub fn new(width: i32) -> Self {
        Self {
            bits: [0; NUM_U64S],
            width,
            count: 0,
        }
    }

    /// Creates a `BoxSet` from an iterator of positions.
    pub fn from_iter(width: i32, positions: impl IntoIterator<Item = IVector2>) -> Self {
        let mut set = Self::new(width);
        for pos in positions {
            set.insert(pos);
        }
        set
    }

    /// Converts a position to a bit index.
    #[inline]
    fn position_to_index(&self, position: IVector2) -> usize {
        (position.y * self.width + position.x) as usize
    }

    /// Converts a bit index to a position.
    #[inline]
    fn index_to_position(&self, index: usize) -> IVector2 {
        let x = (index as i32) % self.width;
        let y = (index as i32) / self.width;
        IVector2::new(x, y)
    }

    /// Inserts a position into the set.
    ///
    /// Returns `true` if the position was newly inserted.
    #[inline]
    pub fn insert(&mut self, position: IVector2) -> bool {
        let index = self.position_to_index(position);
        debug_assert!(index < MAX_POSITIONS, "Position out of bounds");

        let word_index = index / BITS_PER_U64;
        let bit_index = index % BITS_PER_U64;
        let mask = 1u64 << bit_index;

        let was_present = (self.bits[word_index] & mask) != 0;
        self.bits[word_index] |= mask;

        if !was_present {
            self.count += 1;
        }
        !was_present
    }

    /// Removes a position from the set.
    ///
    /// Returns `true` if the position was present.
    #[inline]
    pub fn remove(&mut self, position: IVector2) -> bool {
        let index = self.position_to_index(position);
        debug_assert!(index < MAX_POSITIONS, "Position out of bounds");

        let word_index = index / BITS_PER_U64;
        let bit_index = index % BITS_PER_U64;
        let mask = 1u64 << bit_index;

        let was_present = (self.bits[word_index] & mask) != 0;
        self.bits[word_index] &= !mask;

        if was_present {
            self.count -= 1;
        }
        was_present
    }

    /// Returns `true` if the set contains the position.
    #[inline]
    pub fn contains(&self, position: &IVector2) -> bool {
        let index = self.position_to_index(*position);
        debug_assert!(index < MAX_POSITIONS, "Position out of bounds");

        let word_index = index / BITS_PER_U64;
        let bit_index = index % BITS_PER_U64;
        (self.bits[word_index] & (1u64 << bit_index)) != 0
    }

    /// Returns the number of positions in the set.
    #[inline]
    pub fn len(&self) -> usize {
        self.count
    }

    /// Returns `true` if the set is empty.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.count == 0
    }

    /// Clears all positions from the set.
    pub fn clear(&mut self) {
        self.bits = [0; NUM_U64S];
        self.count = 0;
    }

    /// Returns an iterator over the positions in the set.
    pub fn iter(&self) -> BoxSetIter<'_> {
        BoxSetIter {
            set: self,
            word_index: 0,
            current_word: self.bits[0],
        }
    }

    /// Returns the map width used for position calculations.
    #[inline]
    pub fn width(&self) -> i32 {
        self.width
    }

    /// Returns an iterator over positions in this set that are not in the other
    /// set.
    pub fn difference<'a>(&'a self, other: &'a BoxSet) -> impl Iterator<Item = IVector2> + 'a {
        self.iter().filter(move |pos| !other.contains(pos))
    }
}

impl Hash for BoxSet {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.width.hash(state);
        // hash all words; deterministic
        for &word in &self.bits {
            word.hash(state);
        }
    }
}

/// Iterator over positions in a `BoxSet`.
/// O(number_of_set_bits), not O(4096).
pub struct BoxSetIter<'a> {
    set: &'a BoxSet,
    word_index: usize,
    current_word: u64,
}

impl<'a> Iterator for BoxSetIter<'a> {
    type Item = IVector2;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        loop {
            if self.word_index >= NUM_U64S {
                return None;
            }

            if self.current_word != 0 {
                // extract lowest set bit
                let tz = self.current_word.trailing_zeros() as usize;
                self.current_word &= self.current_word - 1; // clear lowest set bit

                let global_index = self.word_index * BITS_PER_U64 + tz;
                return Some(self.set.index_to_position(global_index));
            }

            // move to next word
            self.word_index += 1;
            if self.word_index < NUM_U64S {
                self.current_word = self.set.bits[self.word_index];
            }
        }
    }
}

impl<'a> IntoIterator for &'a BoxSet {
    type Item = IVector2;
    type IntoIter = BoxSetIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_contains() {
        let mut set = BoxSet::new(10);
        assert!(set.insert(IVector2::new(5, 5)));
        assert!(set.contains(&IVector2::new(5, 5)));
        assert!(!set.insert(IVector2::new(5, 5))); // Already present
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn test_remove() {
        let mut set = BoxSet::new(10);
        set.insert(IVector2::new(3, 4));
        assert!(set.remove(IVector2::new(3, 4)));
        assert!(!set.contains(&IVector2::new(3, 4)));
        assert!(!set.remove(IVector2::new(3, 4))); // Already removed
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn test_iter() {
        let mut set = BoxSet::new(10);
        set.insert(IVector2::new(1, 2));
        set.insert(IVector2::new(5, 5));
        set.insert(IVector2::new(9, 9));

        let positions: Vec<_> = set.iter().collect();
        assert_eq!(positions.len(), 3);
        assert!(positions.contains(&IVector2::new(1, 2)));
        assert!(positions.contains(&IVector2::new(5, 5)));
        assert!(positions.contains(&IVector2::new(9, 9)));
    }

    #[test]
    fn test_hash_deterministic() {
        use std::collections::hash_map::DefaultHasher;

        let mut set1 = BoxSet::new(10);
        set1.insert(IVector2::new(1, 2));
        set1.insert(IVector2::new(5, 5));

        let mut set2 = BoxSet::new(10);
        set2.insert(IVector2::new(5, 5));
        set2.insert(IVector2::new(1, 2));

        let hash1 = {
            let mut hasher = DefaultHasher::new();
            set1.hash(&mut hasher);
            hasher.finish()
        };

        let hash2 = {
            let mut hasher = DefaultHasher::new();
            set2.hash(&mut hasher);
            hasher.finish()
        };

        assert_eq!(hash1, hash2, "Equal sets must have equal hashes");
    }

    #[test]
    fn test_clear() {
        let mut set = BoxSet::new(10);
        set.insert(IVector2::new(1, 2));
        set.insert(IVector2::new(5, 5));
        assert_eq!(set.len(), 2);

        set.clear();
        assert_eq!(set.len(), 0);
        assert!(!set.contains(&IVector2::new(1, 2)));
        assert!(!set.contains(&IVector2::new(5, 5)));
    }

    #[test]
    fn test_width_returns_constructor_arg() {
        let set = BoxSet::new(13);
        assert_eq!(set.width(), 13);
    }

    #[test]
    fn test_is_empty() {
        let mut set = BoxSet::new(8);
        assert!(set.is_empty());
        set.insert(IVector2::new(2, 2));
        assert!(!set.is_empty());
        set.remove(IVector2::new(2, 2));
        assert!(set.is_empty());
    }

    #[test]
    fn test_from_iter_dedupes() {
        let positions = [
            IVector2::new(1, 2),
            IVector2::new(3, 4),
            IVector2::new(1, 2), // duplicate
        ];
        let set = BoxSet::from_iter(8, positions);
        assert_eq!(set.len(), 2);
        assert!(set.contains(&IVector2::new(1, 2)));
        assert!(set.contains(&IVector2::new(3, 4)));
    }

    #[test]
    fn test_difference() {
        let a = BoxSet::from_iter(
            8,
            [
                IVector2::new(1, 1),
                IVector2::new(2, 2),
                IVector2::new(3, 3),
            ],
        );
        let b = BoxSet::from_iter(8, [IVector2::new(2, 2)]);

        let diff: Vec<IVector2> = a.difference(&b).collect();
        assert_eq!(diff.len(), 2);
        assert!(diff.contains(&IVector2::new(1, 1)));
        assert!(diff.contains(&IVector2::new(3, 3)));
        assert!(!diff.contains(&IVector2::new(2, 2)));
    }

    #[test]
    fn test_into_iter_for_reference() {
        // `&BoxSet: IntoIterator` lets you use a `for` loop directly.
        let set = BoxSet::from_iter(8, [IVector2::new(1, 2), IVector2::new(3, 4)]);
        let mut count = 0;
        for _pos in &set {
            count += 1;
        }
        assert_eq!(count, 2);
    }

    #[test]
    fn test_remove_idempotent_after_first_call() {
        let mut set = BoxSet::new(8);
        set.insert(IVector2::new(2, 2));
        assert!(set.remove(IVector2::new(2, 2)));
        assert!(!set.remove(IVector2::new(2, 2)));
        assert!(!set.remove(IVector2::new(2, 2)));
    }
}
