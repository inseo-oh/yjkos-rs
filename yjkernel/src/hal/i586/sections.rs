// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

//! Addresses exported by the i586 linker script.

unsafe extern "C" {
    static __KERNEL_PHYSICAL_ADDRESS_START__: u8;
    static __KERNEL_PHYSICAL_ADDRESS_END__: u8;
    static __KERNEL_TEXT_START__: u8;
    static __KERNEL_TEXT_END__: u8;
    static __KERNEL_RO_AFTER_EARLY_INIT_START__: u8;
    static __KERNEL_RO_AFTER_EARLY_INIT_END__: u8;
    static __ARCH_KERNEL_VIRTUAL_ADDRESS_START__: u8;
    static __ARCH_KERNEL_VIRTUAL_ADDRESS_END__: u8;
}

pub(super) fn kernel_physical_address_start() -> usize {
    core::ptr::addr_of!(__KERNEL_PHYSICAL_ADDRESS_START__) as usize
}

pub(super) fn kernel_physical_address_end() -> usize {
    core::ptr::addr_of!(__KERNEL_PHYSICAL_ADDRESS_END__) as usize
}

pub(super) fn kernel_text_start() -> usize {
    core::ptr::addr_of!(__KERNEL_TEXT_START__) as usize
}

pub(super) fn kernel_text_end() -> usize {
    core::ptr::addr_of!(__KERNEL_TEXT_END__) as usize
}

pub(super) fn kernel_ro_after_early_init_start() -> usize {
    core::ptr::addr_of!(__KERNEL_RO_AFTER_EARLY_INIT_START__) as usize
}

pub(super) fn kernel_ro_after_early_init_end() -> usize {
    core::ptr::addr_of!(__KERNEL_RO_AFTER_EARLY_INIT_END__) as usize
}

pub(super) fn kernel_virtual_address_start() -> usize {
    core::ptr::addr_of!(__ARCH_KERNEL_VIRTUAL_ADDRESS_START__) as usize
}

pub(super) fn kernel_virtual_address_end() -> usize {
    core::ptr::addr_of!(__ARCH_KERNEL_VIRTUAL_ADDRESS_END__) as usize
}
