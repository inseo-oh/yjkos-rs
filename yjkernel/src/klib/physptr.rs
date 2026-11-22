// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use core::mem::{ManuallyDrop, MaybeUninit, align_of, size_of};

use crate::klib;

fn pmemcpy_in<HW: klib::hal::Hardware>(dest: *mut u8, src: usize, len: usize, cache_inhibit: bool) {
    let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
    let mut src_page = toolbox::miscmath::align_usize_down(src, HW::PAGE_SIZE);
    let mut offset = src - src_page;
    let mut dest_byte = dest;
    let mut copy_len;
    let mut i = 0;
    while i < len {
        copy_len = len - i;
        let max_len = HW::PAGE_SIZE - offset;
        if max_len < copy_len {
            copy_len = max_len;
        }
        HW::mmu_scratch_map(klib::PhysicalPtr::new(src_page), cache_inhibit);
        unsafe { dest_byte.copy_from(HW::SCRATCH_MAP_BASE.add(offset), copy_len) };
        dest_byte = unsafe { dest_byte.add(copy_len) };
        i += copy_len;
        if i < len {
            src_page += HW::PAGE_SIZE;
            offset = 0;
        }
    }
}

fn pmemcpy_out<HW: klib::hal::Hardware>(
    dest: usize,
    src: *const u8,
    len: usize,
    cache_inhibit: bool,
) {
    let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
    let mut dest_page = toolbox::miscmath::align_usize_down(dest, HW::PAGE_SIZE);
    let mut offset = dest - dest_page;
    let mut src_byte = src;
    let mut copy_len;
    let mut i = 0;
    while i < len {
        let max_len = HW::PAGE_SIZE - offset;
        copy_len = len - i;
        if max_len < copy_len {
            copy_len = max_len;
        }
        HW::mmu_scratch_map(klib::PhysicalPtr::new(dest_page), cache_inhibit);
        unsafe {
            HW::SCRATCH_MAP_BASE
                .add(offset)
                .copy_from(src_byte, copy_len)
        };
        src_byte = unsafe { src_byte.add(copy_len) };
        i += copy_len;
        if i < len {
            dest_page += HW::PAGE_SIZE;
            offset = 0;
        }
    }
}

#[repr(transparent)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub(crate) struct PhysicalPtr(usize);

/// Most APIs have similar behavior to `*mut u8`'s methods, with exception being read/write methods.
impl PhysicalPtr {
    pub(crate) const fn new(addr: usize) -> Self {
        Self(addr)
    }
    pub(crate) const fn addr(self) -> usize {
        self.0
    }

    pub(crate) fn checked_add(self, bytes: usize) -> Option<Self> {
        self.0.checked_add(bytes).map(Self)
    }

    pub(in crate::klib) fn checked_sub(self, bytes: usize) -> Option<Self> {
        self.0.checked_sub(bytes).map(Self)
    }

    pub(crate) fn checked_distance_from(self, origin: Self) -> Option<usize> {
        self.0.checked_sub(origin.0)
    }

    fn checked_byte_len<T>(count: usize) -> usize {
        size_of::<T>()
            .checked_mul(count)
            .expect("PhysicalPtr byte length overflow")
    }

    fn checked_end(self, byte_len: usize) -> usize {
        self.0
            .checked_add(byte_len)
            .expect("PhysicalPtr address range overflow")
    }

    pub(in crate::klib) fn map_addr(self, f: impl FnOnce(usize) -> usize) -> Self {
        Self(f(self.addr()))
    }
    /// # Safety
    /// The resulting range must remain within the physical allocation the
    /// caller intends to access.
    pub(in crate::klib) const unsafe fn offset(self, count: isize) -> Self {
        Self(self.0.strict_add_signed(count))
    }

    pub(in crate::klib) const fn wrapping_offset(self, count: isize) -> Self {
        Self(self.0.wrapping_add_signed(count))
    }
    /// # Safety
    /// The distance between these addresses must fit in `isize`.
    pub(in crate::klib) const unsafe fn offset_from(self, origin: Self) -> isize {
        self.0.wrapping_sub(origin.0) as isize
    }

    /// # Safety
    /// `self` must be at or after `origin` in the physical address space.
    pub(in crate::klib) unsafe fn offset_from_unsigned(self, origin: Self) -> usize {
        // SAFETY: The caller guarantees the subtraction is nonnegative.
        unsafe { self.0.checked_sub(origin.0).unwrap_unchecked() }
    }
    /// # Safety
    /// The resulting address must remain within the physical allocation the
    /// caller intends to access.
    pub(crate) const unsafe fn add(self, count: usize) -> Self {
        Self(self.0 + count)
    }

    /// # Safety
    /// The resulting address must remain within the physical allocation the
    /// caller intends to access.
    pub(in crate::klib) const unsafe fn sub(self, count: usize) -> Self {
        Self(self.0 - count)
    }
    pub(in crate::klib) const fn wrapping_add(self, count: usize) -> Self {
        Self(self.0.wrapping_add(count))
    }
    pub(in crate::klib) const fn wrapping_sub(self, count: usize) -> Self {
        Self(self.0.wrapping_sub(count))
    }
    /// # Safety
    /// The address must be valid and aligned for `T`, and its bytes must encode
    /// a valid `T`. Treat the source as uninitialized after this move if `T`
    /// is not `Copy`.
    pub(crate) unsafe fn read<HW: klib::hal::Hardware, T>(self, cache_inhibit: bool) -> T {
        self.checked_end(size_of::<T>());
        let mut value = MaybeUninit::<T>::uninit();
        pmemcpy_in::<HW>(
            value.as_mut_ptr().cast::<u8>(),
            self.addr(),
            size_of::<T>(),
            cache_inhibit,
        );
        unsafe { value.assume_init() }
    }
    /// # Safety
    /// Both ranges must be valid for `count` aligned, initialized `T` values;
    /// `dest` must be writable. `T` must be `Copy` so this byte copy does not
    /// duplicate ownership of a non-`Copy` value.
    pub(in crate::klib) unsafe fn copy_to_virtual<HW: klib::hal::Hardware, T: Copy>(
        self,
        dest: *mut T,
        count: usize,
        cache_inhibit: bool,
    ) {
        let byte_len = Self::checked_byte_len::<T>(count);
        self.checked_end(byte_len);
        pmemcpy_in::<HW>(dest.cast::<u8>(), self.addr(), byte_len, cache_inhibit);
    }
    /// # Safety
    /// Both physical ranges must be valid and aligned for `count` initialized
    /// `T` values. `T` must be `Copy`.
    pub(in crate::klib) unsafe fn copy_to_physical<HW: klib::hal::Hardware, T: Copy>(
        self,
        dest: PhysicalPtr,
        count: usize,
        cache_inhibit: bool,
    ) {
        let byte_len = Self::checked_byte_len::<T>(count);
        self.checked_end(byte_len);
        dest.checked_end(byte_len);
        let copy_one = |i: usize| {
            let byte_offset = i * size_of::<T>();
            // SAFETY: The caller guarantees the ranges contain initialized T values.
            let v = unsafe { self.add(byte_offset).read::<HW, T>(cache_inhibit) };
            // SAFETY: The caller guarantees the destination range is writable.
            unsafe { dest.add(byte_offset).write::<HW, T>(v, cache_inhibit) };
        };
        if dest.0 > self.0 && dest.0 < self.0 + byte_len {
            for i in (0..count).rev() {
                copy_one(i);
            }
        } else {
            for i in 0..count {
                copy_one(i);
            }
        }
    }
    pub(crate) fn align_offset(self, align: usize) -> usize {
        assert!(align.is_power_of_two());
        self.0.wrapping_neg() & (align - 1)
    }
    pub(in crate::klib) fn is_aligned<T>(self) -> bool {
        self.align_offset(align_of::<T>()) == 0
    }
    pub(crate) const fn is_null(self) -> bool {
        self.0 == 0
    }
    /// # Safety
    /// The address must be valid and aligned for `T`. The caller becomes
    /// responsible for the value's lifetime in physical memory, including
    /// eventually reading and dropping it if `T` needs drop.
    pub(in crate::klib) unsafe fn write<HW: klib::hal::Hardware, T>(
        self,
        value: T,
        cache_inhibit: bool,
    ) {
        self.checked_end(size_of::<T>());
        let value = ManuallyDrop::new(value);
        let ptr = (&value as *const ManuallyDrop<T>).cast::<T>();
        pmemcpy_out::<HW>(self.addr(), ptr.cast::<u8>(), size_of::<T>(), cache_inhibit);
    }

    /// # Safety
    /// `from` must be valid, aligned, and readable for `count` initialized
    /// `T` values. The physical destination must be valid and aligned for the
    /// same range. `T` must be `Copy`.
    pub(in crate::klib) unsafe fn copy_from_virtual<HW: klib::hal::Hardware, T: Copy>(
        self,
        from: *const T,
        count: usize,
        cache_inhibit: bool,
    ) {
        let byte_len = Self::checked_byte_len::<T>(count);
        self.checked_end(byte_len);
        for i in 0..count {
            let v = unsafe { from.add(i).read() };
            let byte_offset = i * size_of::<T>();
            unsafe { self.add(byte_offset).write::<HW, T>(v, cache_inhibit) };
        }
    }

    /// # Safety
    /// Both physical ranges must be valid and aligned for `count` initialized
    /// `T` values. `T` must be `Copy`.
    pub(in crate::klib) unsafe fn copy_from_physical<HW: klib::hal::Hardware, T: Copy>(
        self,
        from: PhysicalPtr,
        count: usize,
        cache_inhibit: bool,
    ) {
        // SAFETY: The caller guarantees both ranges meet the copy preconditions.
        unsafe { from.copy_to_physical::<HW, T>(self, count, cache_inhibit) };
    }

    /// # Safety
    /// The physical range `[self, self + count)` must be writable.
    pub(in crate::klib) unsafe fn write_bytes<HW: klib::hal::Hardware>(
        self,
        val: u8,
        count: usize,
        cache_inhibit: bool,
    ) {
        self.checked_end(count);
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        let mut dest_page = toolbox::miscmath::align_usize_down(self.addr(), HW::PAGE_SIZE);
        let mut offset = self.addr() - dest_page;
        let mut copy_len;
        let mut i = 0;
        while i < count {
            let max_len = HW::PAGE_SIZE - offset;
            copy_len = count - i;
            if max_len < copy_len {
                copy_len = max_len;
            }
            HW::mmu_scratch_map(klib::PhysicalPtr::new(dest_page), cache_inhibit);
            unsafe { HW::SCRATCH_MAP_BASE.add(offset).write_bytes(val, copy_len) };
            i += copy_len;
            if i < count {
                dest_page += HW::PAGE_SIZE;
                offset = 0;
            }
        }
    }
}
