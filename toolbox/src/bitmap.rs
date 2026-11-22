// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::miscmath;

pub type Word = u32;
const WORD_ALL_ONES: Word = !0;
const BITS_PER_WORD: usize = Word::BITS as usize;

fn find_first_set_bit(word: Word, start_pos: usize) -> Option<usize> {
    if BITS_PER_WORD <= start_pos {
        return None;
    }
    let mut shifted = word >> start_pos as Word;
    if shifted == 0 {
        return None;
    }
    let mut index = start_pos;
    while (shifted & 1) == 0 {
        if index == (BITS_PER_WORD - 1) {
            return None;
        }
        shifted >>= 1;
        index += 1;
    }
    Some(index)
}

fn find_last_contiguous_bit(word: Word, start_pos: usize) -> Option<usize> {
    if BITS_PER_WORD <= start_pos {
        return None;
    }
    if word == WORD_ALL_ONES {
        return Some(BITS_PER_WORD - 1);
    }
    let mut shifted = word >> start_pos;
    if (shifted & 0x1) == 0 {
        return None;
    }
    let mut index = start_pos;
    while (shifted & 1) != 0 {
        shifted >>= 1;
        index += 1;
    }
    Some(index - 1)
}

fn make_bitmask(offset: usize, len: usize) -> Word {
    if len == 0 {
        return 0;
    }
    (WORD_ALL_ONES >> (BITS_PER_WORD - len)) << offset
}

pub fn needed_word_count(bits: usize) -> usize {
    miscmath::size_to_blocks(bits, BITS_PER_WORD)
}

impl<'a> Bitmap<'a> {
    pub fn needed_word_count(bits: usize) -> usize {
        needed_word_count(bits)
    }
}

pub struct Bitmap<'a>(pub &'a mut [Word]);
impl<'a> Bitmap<'a> {
    pub fn find_first_set_bit(&self, start_pos: usize) -> Option<usize> {
        let start_word_idx = start_pos / BITS_PER_WORD;
        let mut bit_offset = start_pos % BITS_PER_WORD;
        for word_idx in start_word_idx..self.0.len() {
            if let Some(idx) = find_first_set_bit(self.0[word_idx], bit_offset) {
                return Some((word_idx * BITS_PER_WORD) + idx);
            }
            bit_offset = 0;
        }
        None
    }
    fn find_last_contiguous_bit(&self, start_pos: usize) -> Option<usize> {
        let start_word_idx = start_pos / BITS_PER_WORD;
        let mut bit_offset = start_pos % BITS_PER_WORD;
        if self.0.len() <= start_word_idx {
            return None;
        }
        for word_idx in start_word_idx..self.0.len() {
            {
                let idx = find_last_contiguous_bit(self.0[word_idx], bit_offset)?;
                if (idx != (BITS_PER_WORD - 1)) ||
                // We found 1 at MSB, and it doesn't continue on the next word.
                ((idx == (BITS_PER_WORD - 1)) && ((word_idx + 1) < self.0.len()) && ((self.0[word_idx + 1] & 0x1) == 0))
                {
                    return Some((word_idx * BITS_PER_WORD) + idx);
                }
            }
            bit_offset = 0;
        }
        Some((self.0.len() * BITS_PER_WORD) - 1)
    }
    pub fn find_set_bits(&self, start_pos: usize, min_len: usize) -> Option<usize> {
        let mut first_bit_idx = start_pos;
        let mut last_bit_idx;
        loop {
            {
                let found = self.find_first_set_bit(first_bit_idx)?;
                first_bit_idx = found;
            }
            last_bit_idx = self.find_last_contiguous_bit(first_bit_idx).unwrap();
            let foundlen = last_bit_idx - first_bit_idx + 1;
            if min_len <= foundlen {
                return Some(first_bit_idx);
            }
            first_bit_idx = last_bit_idx + 1
        }
    }
    fn are_bits_set(&self, offset: usize, len: usize) -> bool {
        if len == 0 {
            return false;
        }
        let mut word_idx = offset / BITS_PER_WORD;
        let mut bit_offset = offset % BITS_PER_WORD;
        let mut remaining_len = len;
        let mut current_len;
        while remaining_len != 0 {
            if self.0.len() <= word_idx {
                return false;
            }
            let word = self.0[word_idx];
            if word == 0 {
                return false;
            }
            current_len = remaining_len;
            if BITS_PER_WORD < (bit_offset + current_len) {
                current_len = BITS_PER_WORD - bit_offset;
            }
            let mask = make_bitmask(bit_offset, current_len);
            if (word & mask) != mask {
                return false;
            }
            remaining_len -= current_len;
            word_idx += 1;
            bit_offset = 0;
        }
        true
    }
    pub fn set_bits(&mut self, offset: usize, len: usize) {
        if len == 0 {
            return;
        }
        let mut word_idx = offset / BITS_PER_WORD;
        let mut bit_offset = offset % BITS_PER_WORD;
        let mut remaining_len = len;
        let mut current_len;
        while remaining_len != 0 {
            if self.0.len() <= word_idx {
                break;
            }
            current_len = remaining_len;
            if BITS_PER_WORD < (bit_offset + current_len) {
                current_len = BITS_PER_WORD - bit_offset;
            }
            let mask = make_bitmask(bit_offset, current_len);
            self.0[word_idx] |= mask;
            remaining_len -= current_len;
            word_idx += 1;
            bit_offset = 0;
        }
    }
    pub fn clear_bits(&mut self, offset: usize, len: usize) {
        if len == 0 {
            return;
        }
        let mut word_idx = offset / BITS_PER_WORD;
        let mut bit_offset = offset % BITS_PER_WORD;
        let mut remaining_len = len;
        let mut current_len;
        while remaining_len != 0 {
            if self.0.len() <= word_idx {
                break;
            }
            current_len = remaining_len;
            if BITS_PER_WORD < (bit_offset + current_len) {
                current_len = BITS_PER_WORD - bit_offset;
            }
            let mask = make_bitmask(bit_offset, current_len);
            self.0[word_idx] &= !mask;
            remaining_len -= current_len;
            word_idx += 1;
            bit_offset = 0;
        }
    }
    pub fn set_bit(&mut self, offset: usize) {
        if self.0.len() <= (offset / BITS_PER_WORD) {
            return;
        }
        self.0[offset / BITS_PER_WORD] |= 1 << (offset % BITS_PER_WORD);
    }
    pub fn clear_bit(&mut self, offset: usize) {
        if self.0.len() <= (offset / BITS_PER_WORD) {
            return;
        }
        self.0[offset / BITS_PER_WORD] &= !(1 << (offset % BITS_PER_WORD));
    }
    pub fn is_bit_set(&self, offset: usize) -> bool {
        if self.0.len() <= (offset / BITS_PER_WORD) {
            return false;
        }
        self.0[offset / BITS_PER_WORD] & (1 << (offset % BITS_PER_WORD)) != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_first_set_bit_in_word() {
        assert_eq!(find_first_set_bit(0, 0), None);
        assert_eq!(find_first_set_bit(1, 0), Some(0));
        assert_eq!(find_first_set_bit(0b1000, 0), Some(3));
        assert_eq!(find_first_set_bit(0b1010, 2), Some(3));
        assert_eq!(find_first_set_bit(0b1010, 4), None);
        assert_eq!(find_first_set_bit(1 << 31, 31), Some(31));
        assert_eq!(find_first_set_bit(u32::MAX, 32), None);
    }

    #[test]
    fn test_find_last_contiguous_bit_in_word() {
        assert_eq!(find_last_contiguous_bit(0, 0), None);
        assert_eq!(find_last_contiguous_bit(0b1, 0), Some(0));
        assert_eq!(find_last_contiguous_bit(0b1110, 1), Some(3));
        assert_eq!(find_last_contiguous_bit(0b1010, 1), Some(1));
        assert_eq!(find_last_contiguous_bit(u32::MAX, 0), Some(31));
        assert_eq!(find_last_contiguous_bit(1 << 31, 31), Some(31));
        assert_eq!(find_last_contiguous_bit(u32::MAX, 32), None);
    }

    #[test]
    fn test_make_bitmask() {
        assert_eq!(make_bitmask(0, 0), 0);
        assert_eq!(make_bitmask(0, 1), 0x0000_0001);
        assert_eq!(make_bitmask(3, 4), 0x0000_0078);
        assert_eq!(make_bitmask(0, 32), u32::MAX);
        assert_eq!(make_bitmask(31, 1), 0x8000_0000);
        assert_eq!(make_bitmask(16, 16), 0xffff_0000);
    }

    #[test]
    fn test_needed_word_count() {
        assert_eq!(needed_word_count(0), 0);
        assert_eq!(needed_word_count(1), 1);
        assert_eq!(needed_word_count(31), 1);
        assert_eq!(needed_word_count(32), 1);
        assert_eq!(needed_word_count(33), 2);
        assert_eq!(needed_word_count(64), 2);
        assert_eq!(needed_word_count(65), 3);
    }

    #[test]
    fn test_set_and_clear_bits() {
        let mut words = [0u32; 3];
        let mut bitmap = Bitmap(&mut words);

        bitmap.set_bits(0, 4);
        assert_eq!(bitmap.0[0], 0x0000_000f);

        bitmap.set_bits(28, 8);
        assert_eq!(bitmap.0[0], 0xf000_000f);
        assert_eq!(bitmap.0[1], 0x0000_000f);

        bitmap.clear_bits(30, 4);
        assert_eq!(bitmap.0[0], 0x3000_000f);
        assert_eq!(bitmap.0[1], 0x0000_000c);

        bitmap.clear_bits(0, 4);
        assert_eq!(bitmap.0[0], 0x3000_0000);
    }

    #[test]
    fn test_individual_bits() {
        let mut words = [0u32; 2];
        let mut bitmap = Bitmap(&mut words);

        for bit in [0, 31, 32, 63] {
            bitmap.set_bit(bit);
            assert!(bitmap.is_bit_set(bit));
        }

        assert_eq!(bitmap.0[0], 0x8000_0001);
        assert_eq!(bitmap.0[1], 0x8000_0001);

        bitmap.clear_bit(31);
        bitmap.clear_bit(32);

        assert!(!bitmap.is_bit_set(31));
        assert!(!bitmap.is_bit_set(32));
        assert!(bitmap.is_bit_set(0));
        assert!(bitmap.is_bit_set(63));
    }

    #[test]
    fn test_find_first_set_bit_across_words() {
        let mut words = [0, 0, 0b1000];
        let bitmap = Bitmap(&mut words);

        assert_eq!(bitmap.find_first_set_bit(0), Some(67));
        assert_eq!(bitmap.find_first_set_bit(64), Some(67));
        assert_eq!(bitmap.find_first_set_bit(68), None);
        assert_eq!(bitmap.find_first_set_bit(96), None);
    }

    #[test]
    fn test_find_last_contiguous_bit_across_words() {
        let mut words = [0xf000_0000, 0xffff_ffff, 0x0000_000f];
        let bitmap = Bitmap(&mut words);

        assert_eq!(bitmap.find_last_contiguous_bit(28), Some(67));
        assert_eq!(bitmap.find_last_contiguous_bit(32), Some(67));
        assert_eq!(bitmap.find_last_contiguous_bit(64), Some(67));
        assert_eq!(bitmap.find_last_contiguous_bit(68), None);
    }

    #[test]
    fn test_find_set_bits() {
        let mut words = [0x0000_000f, 0xffff_ffff, 0x0000_00ff];
        let bitmap = Bitmap(&mut words);

        assert_eq!(bitmap.find_set_bits(0, 4), Some(0));
        assert_eq!(bitmap.find_set_bits(0, 5), Some(32));
        assert_eq!(bitmap.find_set_bits(32, 40), Some(32));
        assert_eq!(bitmap.find_set_bits(32, 41), None);
        assert_eq!(bitmap.find_set_bits(64, 8), Some(64));
        assert_eq!(bitmap.find_set_bits(64, 9), None);
    }

    #[test]
    fn test_are_bits_set() {
        let mut words = [0xffff_ffff, 0x0000_000f];
        let bitmap = Bitmap(&mut words);

        assert!(bitmap.are_bits_set(0, 32));
        assert!(bitmap.are_bits_set(28, 8));
        assert!(bitmap.are_bits_set(32, 4));

        assert!(!bitmap.are_bits_set(28, 9));
        assert!(!bitmap.are_bits_set(36, 1));
        assert!(!bitmap.are_bits_set(0, 0));
        assert!(!bitmap.are_bits_set(63, 2));
    }

    #[test]
    fn test_full_bitmap_allocation() {
        let mut words = [u32::MAX; 4];
        let mut bitmap = Bitmap(&mut words);

        // 10개 연속 블록 할당
        let first = bitmap.find_set_bits(0, 10).unwrap();
        assert_eq!(first, 0);
        bitmap.clear_bits(first, 10);

        // 다음 20개 연속 블록 할당
        let second = bitmap.find_set_bits(0, 20).unwrap();
        assert_eq!(second, 10);
        bitmap.clear_bits(second, 20);

        assert!(!bitmap.are_bits_set(0, 30));
        assert_eq!(bitmap.find_set_bits(0, 30), Some(30));

        // 첫 번째 할당 해제
        bitmap.set_bits(first, 10);
        assert!(bitmap.are_bits_set(0, 10));
        assert_eq!(bitmap.find_set_bits(0, 10), Some(0));
    }

    #[test]
    fn test_empty_bitmap() {
        let mut words: [u32; 0] = [];
        let bitmap = Bitmap(&mut words);

        assert_eq!(bitmap.find_first_set_bit(0), None);
        assert_eq!(bitmap.find_last_contiguous_bit(0), None);
        assert_eq!(bitmap.find_set_bits(0, 1), None);
        assert!(!bitmap.are_bits_set(0, 1));
    }

    #[test]
    fn test_out_of_bounds() {
        let mut words = [0u32; 1];
        let mut bitmap = Bitmap(&mut words);

        assert!(!bitmap.is_bit_set(32));
        assert!(!bitmap.is_bit_set(64));

        bitmap.set_bit(32);
        bitmap.clear_bit(64);

        assert_eq!(bitmap.0[0], 0);
    }

    #[test]
    fn bit_range_updates_match_a_boolean_reference_model() {
        const BIT_COUNT: usize = 64;

        for offset in 0..=BIT_COUNT {
            for len in 0..=BIT_COUNT {
                let mut words = [0u32; 2];
                let mut expected = [false; BIT_COUNT];
                let mut bitmap = Bitmap(&mut words);

                bitmap.set_bits(offset, len);
                for bit in offset..offset.saturating_add(len).min(BIT_COUNT) {
                    expected[bit] = true;
                }
                for (bit, expected_value) in expected.iter().enumerate() {
                    assert_eq!(
                        bitmap.is_bit_set(bit),
                        *expected_value,
                        "set {offset}+{len}, bit {bit}"
                    );
                }

                bitmap.clear_bits(offset, len);
                for bit in offset..offset.saturating_add(len).min(BIT_COUNT) {
                    expected[bit] = false;
                }
                for (bit, expected_value) in expected.iter().enumerate() {
                    assert_eq!(
                        bitmap.is_bit_set(bit),
                        *expected_value,
                        "clear {offset}+{len}, bit {bit}"
                    );
                }
            }
        }
    }

    #[test]
    fn first_set_and_contiguous_run_searches_match_a_reference_model() {
        for pattern in 0u32..=u8::MAX as u32 {
            let mut words = [pattern, 0];
            let bitmap = Bitmap(&mut words);
            let expected: [bool; 8] = core::array::from_fn(|bit| pattern & (1 << bit) != 0);

            for start in 0..=8 {
                let expected_first = (start..8).find(|bit| expected[*bit]);
                assert_eq!(bitmap.find_first_set_bit(start), expected_first);

                for min_len in 0..=8 {
                    let mut search_from = start;
                    let mut expected_run = None;
                    while let Some(first) = (search_from..8).find(|bit| expected[*bit]) {
                        let mut last = first;
                        while last + 1 < 8 && expected[last + 1] {
                            last += 1;
                        }
                        if min_len <= last - first + 1 {
                            expected_run = Some(first);
                            break;
                        }
                        search_from = last + 1;
                    }
                    assert_eq!(bitmap.find_set_bits(start, min_len), expected_run);
                }
            }
        }
    }
}
