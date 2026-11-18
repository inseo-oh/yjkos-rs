pub const fn is_aligned(x: usize, align: usize) -> bool {
    assert!(align != 0);
    x.is_multiple_of(align)
}
pub const fn align_usize_up(x: usize, align: usize) -> usize {
    assert!(align != 0);
    if is_aligned(x, align) {
        return x;
    }
    x.checked_add(align - (x % align))
        .expect("alignment overflow")
}
pub const fn align_usize_down(x: usize, align: usize) -> usize {
    assert!(align != 0);
    x - (x % align)
}
pub fn is_ptr_aligned<T>(x: *mut T, align: usize) -> bool {
    assert!(align != 0);
    x.addr().is_multiple_of(align)
}
pub fn align_ptr_up<T>(x: *mut T, align: usize) -> *mut T {
    assert!(align != 0);
    x.map_addr(|addr| {
        addr.checked_add((align - (addr % align)) % align)
            .expect("pointer alignment overflow")
    })
}
pub fn align_ptr_down<T>(x: *mut T, align: usize) -> *mut T {
    assert!(align != 0);
    x.map_addr(|addr| addr - (addr % align))
}
pub const fn size_to_blocks(size: usize, block_size: usize) -> usize {
    assert!(block_size != 0);
    if size.is_multiple_of(block_size) {
        return size / block_size;
    }
    size / block_size + 1
}
pub const fn u16le_at(x: &[u8]) -> u32 {
    ((x[1] as u32) << 8) | x[0] as u32
}

pub const fn u32le_at(x: &[u8]) -> u32 {
    ((x[3] as u32) << 24) | ((x[2] as u32) << 16) | ((x[1] as u32) << 8) | x[0] as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aligned_values_are_unchanged_when_aligned_up_or_down() {
        assert_eq!(align_usize_up(24, 8), 24);
        assert_eq!(align_usize_down(24, 8), 24);
    }

    #[test]
    fn integer_alignment_handles_non_power_of_two_alignments() {
        assert_eq!(align_usize_up(10, 6), 12);
        assert_eq!(align_usize_down(10, 6), 6);
        assert_eq!(align_usize_up(12, 6), 12);
    }

    #[test]
    fn integer_alignment_handles_zero_and_maximum_values() {
        assert_eq!(align_usize_up(0, 7), 0);
        assert_eq!(align_usize_down(0, 7), 0);
        assert_eq!(align_usize_down(usize::MAX, 1), usize::MAX);
    }

    #[test]
    #[should_panic(expected = "alignment overflow")]
    fn align_up_panics_when_result_overflows() {
        let _ = align_usize_up(usize::MAX, 2);
    }

    #[test]
    fn alignment_predicate_supports_arbitrary_alignments() {
        assert!(is_aligned(21, 7));
        assert!(!is_aligned(22, 7));
    }

    #[test]
    #[should_panic]
    fn alignment_predicate_rejects_zero_alignment() {
        let _ = is_aligned(0, 0);
    }

    #[test]
    fn pointer_alignment_preserves_aligned_addresses() {
        let aligned = 0x4000usize as *mut u8;
        assert_eq!(align_ptr_down(aligned, 4096).addr(), aligned.addr());
        assert_eq!(align_ptr_up(aligned, 4096).addr(), aligned.addr());
        assert!(is_ptr_aligned(aligned, 4096));
    }

    #[test]
    fn pointer_alignment_rounds_addresses_without_dereferencing_them() {
        let ptr = 0x1237usize as *mut u8;
        assert_eq!(align_ptr_down(ptr, 16).addr(), 0x1230);
        assert_eq!(align_ptr_up(ptr, 16).addr(), 0x1240);
        assert!(!is_ptr_aligned(ptr, 16));
    }

    #[test]
    #[should_panic(expected = "pointer alignment overflow")]
    fn pointer_align_up_panics_when_result_overflows() {
        let _ = align_ptr_up(usize::MAX as *mut u8, 2);
    }

    #[test]
    #[should_panic]
    fn pointer_alignment_rejects_zero_alignment() {
        let _ = align_ptr_down(core::ptr::null_mut::<u8>(), 0);
    }

    #[test]
    fn size_to_blocks_rounds_up_and_handles_exact_multiples() {
        assert_eq!(size_to_blocks(0, 8), 0);
        assert_eq!(size_to_blocks(1, 8), 1);
        assert_eq!(size_to_blocks(8, 8), 1);
        assert_eq!(size_to_blocks(9, 8), 2);
        assert_eq!(size_to_blocks(17, 8), 3);
    }

    #[test]
    fn size_to_blocks_handles_large_values_without_overflow() {
        assert_eq!(size_to_blocks(usize::MAX, 1), usize::MAX);
        assert_eq!(size_to_blocks(usize::MAX, usize::MAX), 1);
    }

    #[test]
    #[should_panic]
    fn size_to_blocks_rejects_zero_block_size() {
        let _ = size_to_blocks(1, 0);
    }

    #[test]
    fn little_endian_readers_decode_values() {
        assert_eq!(u16le_at(&[0x34, 0x12]), 0x1234);
        assert_eq!(u32le_at(&[0x78, 0x56, 0x34, 0x12]), 0x1234_5678);
    }

    #[test]
    #[should_panic]
    fn u16_reader_rejects_short_input() {
        let _ = u16le_at(&[0x34]);
    }

    #[test]
    #[should_panic]
    fn u32_reader_rejects_short_input() {
        let _ = u32le_at(&[0x78, 0x56, 0x34]);
    }
}
