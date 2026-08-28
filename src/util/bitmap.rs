use crate::util::align_up;

/// A mutable view over a fixed number of bits.
///
/// Bit zero is the least-significant bit of the first byte. Bits in `storage`
/// beyond `bit_len` are padding and are never modified.
#[derive(Debug)]
pub struct Bitmap<'a> {
    storage: &'a mut [u8],
    bit_len: usize,
}

impl<'a> Bitmap<'a> {
    /// Creates a bitmap backed by `storage`.
    ///
    /// Returns `None` when `bit_len` does not fit in the supplied storage.
    pub fn new(storage: &'a mut [u8], bit_len: usize) -> Option<Self> {
        let capacity = storage.len().checked_mul(u8::BITS as usize)?;
        if bit_len > capacity {
            return None;
        }

        Some(Self { storage, bit_len })
    }

    pub const fn len(&self) -> usize {
        self.bit_len
    }

    pub const fn is_empty(&self) -> bool {
        self.bit_len == 0
    }

    pub fn get(&self, index: usize) -> Option<bool> {
        if index >= self.bit_len {
            return None;
        }

        Some(self.get_unchecked(index))
    }

    pub fn set(&mut self, index: usize, value: bool) -> Option<()> {
        if index >= self.bit_len {
            return None;
        }

        self.set_unchecked(index, value);
        Some(())
    }

    /// Sets every bit in `start..start + len` to `value`.
    ///
    /// An empty range at the end of the bitmap is valid. Returns `None` if the
    /// range-end calculation overflows or the range is out of bounds.
    pub fn set_range(&mut self, start: usize, len: usize, value: bool) -> Option<()> {
        let end = start.checked_add(len)?;
        if end > self.bit_len {
            return None;
        }

        let mut cursor = start;

        while cursor < end && !cursor.is_multiple_of(u8::BITS as usize) {
            self.set_unchecked(cursor, value);
            cursor += 1;
        }

        let whole_byte_end = end - end % u8::BITS as usize;
        if cursor < whole_byte_end {
            self.storage[cursor / u8::BITS as usize..whole_byte_end / u8::BITS as usize]
                .fill(if value { u8::MAX } else { 0 });
            cursor = whole_byte_end;
        }

        while cursor < end {
            self.set_unchecked(cursor, value);
            cursor += 1;
        }

        Some(())
    }

    /// Sets every usable bit without changing padding bits.
    pub fn fill(&mut self, value: bool) {
        // The complete bitmap range is valid by construction.
        let result = self.set_range(0, self.bit_len, value);
        debug_assert!(
            result.is_some(),
            "the complete range of a valid bitmap must always be in bounds"
        );
    }

    /// Finds the next bit equal to `value`, using `start` as a search hint.
    ///
    /// The search wraps to the beginning after reaching the end. A hint at or
    /// beyond `bit_len` starts the search at zero. Returns `None` only when no
    /// bit in the bitmap equals `value`.
    pub fn find_next(&self, start: usize, value: bool) -> Option<usize> {
        if self.bit_len == 0 {
            return None;
        }

        let start = if start < self.bit_len { start } else { 0 };
        (start..self.bit_len)
            .chain(0..start)
            .find(|&index| self.get_unchecked(index) == value)
    }

    /// Finds the next aligned run of `len` consecutive bits equal to `value`.
    ///
    /// `start` is a search hint. The search wraps to the beginning after
    /// reaching the end, and a hint at or beyond the length of bitmap starts at zero. The
    /// search order wraps, but a returned run never crosses the bitmap
    /// boundary.
    ///
    /// A zero-length run is valid and does not inspect any bits. Its returned
    /// position still satisfies `alignment`. A zero alignment is invalid.
    pub fn find_next_run(
        &self,
        start: usize,
        len: usize,
        alignment: usize,
        value: bool,
    ) -> Option<usize> {
        if alignment == 0 {
            return None;
        }

        let start = if start < self.bit_len { start } else { 0 };

        if len == 0 {
            return match align_up(start, alignment) {
                Some(candidate) if candidate <= self.bit_len => Some(candidate),
                _ => Some(0),
            };
        }

        if len > self.bit_len {
            return None;
        }

        // `len` is non-zero, so adding one cannot overflow here.
        let candidate_limit = self.bit_len - len + 1;

        self.find_run_between(start, candidate_limit, len, alignment, value)
            .or_else(|| self.find_run_between(0, start.min(candidate_limit), len, alignment, value))
    }

    /// Searches candidate start positions in `search_start..search_end`.
    fn find_run_between(
        &self,
        search_start: usize,
        search_end: usize,
        len: usize,
        alignment: usize,
        value: bool,
    ) -> Option<usize> {
        let mut candidate = align_up(search_start, alignment)?;

        while candidate < search_end {
            // `search_end` is limited to the last start position at which the
            // run fits, so this addition is in bounds.
            let end = candidate + len;

            let mismatch = (candidate..end).find(|&index| self.get_unchecked(index) != value);
            match mismatch {
                None => return Some(candidate),
                Some(index) => {
                    candidate = align_up(index.checked_add(1)?, alignment)?;
                }
            }
        }

        None
    }

    /// Reads a bit without checking it against `bit_len` first.
    ///
    /// `index` is expected to be less than `bit_len`. An out-of-range index
    /// may panic while indexing the backing storage.
    fn get_unchecked(&self, index: usize) -> bool {
        let byte = self.storage[index / u8::BITS as usize];
        let mask = 1 << (index % u8::BITS as usize);
        byte & mask != 0
    }

    /// Writes a bit without checking it against `bit_len` first.
    ///
    /// `index` is expected to be less than `bit_len`. An out-of-range index
    /// may panic while indexing the backing storage.
    fn set_unchecked(&mut self, index: usize, value: bool) {
        let byte = &mut self.storage[index / u8::BITS as usize];
        let mask = 1 << (index % u8::BITS as usize);

        if value {
            *byte |= mask;
        } else {
            *byte &= !mask;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Bitmap;

    #[test]
    fn construction_checks_storage_capacity() {
        let mut storage = [0; 1];
        assert!(Bitmap::new(&mut storage, 8).is_some());
        assert!(Bitmap::new(&mut storage, 9).is_none());
    }

    #[test]
    fn empty_bitmap_is_valid() {
        let mut storage = [];
        let bitmap = Bitmap::new(&mut storage, 0).expect("an empty bitmap must be valid");

        assert_eq!(bitmap.len(), 0);
        assert!(bitmap.is_empty());
        assert_eq!(bitmap.get(0), None);
    }

    #[test]
    fn get_and_set_use_least_significant_bit_first() {
        let mut storage = [0b1010_0000];
        let mut bitmap = Bitmap::new(&mut storage, 8).expect("storage has eight bits");

        assert_eq!(bitmap.get(5), Some(true));
        assert_eq!(bitmap.get(6), Some(false));
        assert_eq!(bitmap.get(7), Some(true));

        assert_eq!(bitmap.set(0, true), Some(()));
        assert_eq!(bitmap.set(7, false), Some(()));
        assert_eq!(bitmap.set(8, true), None);
        assert_eq!(bitmap.get(8), None);

        assert_eq!(storage[0], 0b0010_0001);
    }

    #[test]
    fn set_range_handles_partial_and_complete_bytes() {
        let mut storage = [0; 3];
        {
            let mut bitmap = Bitmap::new(&mut storage, 24).expect("storage has 24 bits");
            assert_eq!(bitmap.set_range(3, 18, true), Some(()));
        }
        assert_eq!(storage, [0b1111_1000, 0xff, 0b0001_1111]);

        {
            let mut bitmap = Bitmap::new(&mut storage, 24).expect("storage has 24 bits");
            assert_eq!(bitmap.set_range(7, 10, false), Some(()));
        }
        assert_eq!(storage, [0b0111_1000, 0, 0b0001_1110]);
    }

    #[test]
    fn set_range_rejects_invalid_ranges_without_modifying_storage() {
        let mut storage = [0x5a];
        let mut bitmap = Bitmap::new(&mut storage, 8).expect("storage has eight bits");

        assert_eq!(bitmap.set_range(8, 0, true), Some(()));
        assert_eq!(bitmap.set_range(9, 0, true), None);
        assert_eq!(bitmap.set_range(7, 2, true), None);
        assert_eq!(bitmap.set_range(usize::MAX, 2, true), None);

        assert_eq!(storage, [0x5a]);
    }

    #[test]
    fn fill_preserves_padding_bits() {
        let mut storage = [0, 0b1010_1100];
        {
            let mut bitmap = Bitmap::new(&mut storage, 11).expect("storage has enough bits");
            bitmap.fill(true);
        }
        assert_eq!(storage, [0xff, 0b1010_1111]);

        {
            let mut bitmap = Bitmap::new(&mut storage, 11).expect("storage has enough bits");
            bitmap.fill(false);
        }
        assert_eq!(storage, [0, 0b1010_1000]);
    }

    #[test]
    fn find_next_wraps_and_normalizes_the_hint() {
        let mut storage = [0b0101_0000, 0xff];
        let bitmap = Bitmap::new(&mut storage, 9).expect("storage has enough bits");

        assert_eq!(bitmap.find_next(0, true), Some(4));
        assert_eq!(bitmap.find_next(5, true), Some(6));
        assert_eq!(bitmap.find_next(7, false), Some(7));
        assert_eq!(bitmap.find_next(8, false), Some(0));
        assert_eq!(bitmap.find_next(9, true), Some(4));
        assert_eq!(bitmap.find_next(usize::MAX, true), Some(4));
    }

    #[test]
    fn find_next_returns_none_only_when_no_bit_matches() {
        let mut storage = [u8::MAX];
        let bitmap = Bitmap::new(&mut storage, 8).expect("storage has eight bits");

        assert_eq!(bitmap.find_next(4, false), None);
    }

    #[test]
    fn find_next_run_respects_length_and_alignment() {
        let mut storage = [0b0001_0001, 0b0000_0001];
        let bitmap = Bitmap::new(&mut storage, 16).expect("storage has 16 bits");

        assert_eq!(bitmap.find_next_run(0, 3, 1, false), Some(1));
        assert_eq!(bitmap.find_next_run(0, 3, 4, false), Some(12));
        assert_eq!(bitmap.find_next_run(5, 4, 2, false), Some(10));
        assert_eq!(bitmap.find_next_run(0, 8, 1, false), None);
    }

    #[test]
    fn find_next_run_wraps_without_joining_the_bitmap_edges() {
        let mut storage = [0b1111_0001];
        let bitmap = Bitmap::new(&mut storage, 8).expect("storage has eight bits");

        assert_eq!(bitmap.find_next_run(4, 3, 1, false), Some(1));
        assert_eq!(bitmap.find_next_run(usize::MAX, 3, 1, false), Some(1));

        let mut edge_storage = [0b0011_1100];
        let edge_bitmap = Bitmap::new(&mut edge_storage, 8).expect("storage has eight bits");
        assert_eq!(edge_bitmap.find_next_run(6, 4, 1, false), None);
    }

    #[test]
    fn find_next_run_accepts_an_empty_run() {
        let mut storage = [0];
        let bitmap = Bitmap::new(&mut storage, 8).expect("storage has eight bits");

        assert_eq!(bitmap.find_next_run(3, 0, 4, false), Some(4));
        assert_eq!(bitmap.find_next_run(7, 0, 4, false), Some(8));
        assert_eq!(bitmap.find_next_run(8, 0, 4, false), Some(0));
        assert_eq!(bitmap.find_next_run(usize::MAX, 0, 4, false), Some(0));

        let mut empty_storage = [];
        let empty_bitmap =
            Bitmap::new(&mut empty_storage, 0).expect("an empty bitmap must be valid");
        assert_eq!(empty_bitmap.find_next_run(0, 0, 1, false), Some(0));
    }

    #[test]
    fn find_next_run_rejects_only_unsatisfied_requests() {
        let mut storage = [0];
        let bitmap = Bitmap::new(&mut storage, 8).expect("storage has eight bits");

        assert_eq!(bitmap.find_next_run(0, 1, 0, false), None);
        assert_eq!(bitmap.find_next_run(0, 9, 1, false), None);
    }
}
