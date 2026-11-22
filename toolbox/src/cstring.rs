// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

//! Memory routines required by compiler-generated code for `no_std` targets.
//! These are not meant to be used by normal Rust code directly.

/// # Safety
/// `dst` must be valid for writes of `n` bytes, unless `n` is zero.
#[allow(suspicious_runtime_symbol_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memset(dst: *mut u8, value: i32, n: usize) -> *mut u8 {
    for offset in 0..n {
        // Volatile accesses prevent LLVM from lowering this implementation back
        // into a call to the `memset` symbol itself.
        unsafe { dst.add(offset).write_volatile(value as u8) };
    }
    dst
}

/// # Safety
/// `src` must be valid for reads and `dst` for writes of `n` bytes, unless
/// `n` is zero. The regions must not overlap.
#[allow(suspicious_runtime_symbol_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcpy(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    for offset in 0..n {
        // Volatile accesses prevent LLVM from lowering this implementation back
        // into a call to the `memcpy` symbol itself.
        let byte = unsafe { src.add(offset).read_volatile() };
        unsafe { dst.add(offset).write_volatile(byte) };
    }
    dst
}

/// # Safety
/// `src` and `dst` must be valid for reads and writes of `n` bytes,
/// respectively, unless `n` is zero. The regions may overlap.
#[allow(suspicious_runtime_symbol_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memmove(dst: *mut u8, src: *const u8, n: usize) -> *mut u8 {
    let dst_addr = dst.addr();
    let src_addr = src.addr();

    if dst_addr <= src_addr || dst_addr - src_addr >= n {
        for offset in 0..n {
            let byte = unsafe { src.add(offset).read_volatile() };
            unsafe { dst.add(offset).write_volatile(byte) };
        }
    } else {
        for offset in (0..n).rev() {
            let byte = unsafe { src.add(offset).read_volatile() };
            unsafe { dst.add(offset).write_volatile(byte) };
        }
    }

    dst
}

/// # Safety
/// `lhs` and `rhs` must be valid for reads of `n` bytes, unless `n` is zero.
#[allow(suspicious_runtime_symbol_definitions)]
#[unsafe(no_mangle)]
pub unsafe extern "C" fn memcmp(lhs: *const u8, rhs: *const u8, n: usize) -> i32 {
    for offset in 0..n {
        let lhs_byte = unsafe { lhs.add(offset).read_volatile() };
        let rhs_byte = unsafe { rhs.add(offset).read_volatile() };
        if lhs_byte != rhs_byte {
            return i32::from(lhs_byte) - i32::from(rhs_byte);
        }
    }
    0
}
