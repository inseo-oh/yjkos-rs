// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use alloc::vec::Vec;

use crate::{
    kglobals,
    klib::{self, hal::Hardware},
    println,
};
use core::ptr::{read_volatile, write_volatile};

pub(in crate::hal) const COMMON_FLAG_P: u32 = 1 << 0;
pub(in crate::hal) const COMMON_FLAG_RW: u32 = 1 << 1;
pub(in crate::hal) const COMMON_FLAG_US: u32 = 1 << 2;
pub(in crate::hal) const COMMON_FLAG_PWT: u32 = 1 << 3;
pub(in crate::hal) const COMMON_FLAG_PCD: u32 = 1 << 4;
pub(in crate::hal) const COMMON_FLAG_A: u32 = 1 << 5;
pub(in crate::hal) const COMMON_FLAG_D: u32 = 1 << 6;
pub(in crate::hal) const COMMON_FLAG_G: u32 = 1 << 8;

pub(in crate::hal) const PDE_FLAG_P: u32 = COMMON_FLAG_P;
pub(in crate::hal) const PDE_FLAG_RW: u32 = COMMON_FLAG_RW;
pub(in crate::hal) const PDE_FLAG_US: u32 = COMMON_FLAG_US;
pub(in crate::hal) const PDE_FLAG_PWT: u32 = COMMON_FLAG_PWT;
pub(in crate::hal) const PDE_FLAG_PCD: u32 = COMMON_FLAG_PCD;
pub(in crate::hal) const PDE_FLAG_A: u32 = COMMON_FLAG_A;
pub(in crate::hal) const PDE_FLAG_D: u32 = COMMON_FLAG_D;
pub(in crate::hal) const PDE_FLAG_PS: u32 = 1 << 7;
pub(in crate::hal) const PDE_FLAG_G: u32 = COMMON_FLAG_G;

pub(in crate::hal) const PTE_FLAG_P: u32 = COMMON_FLAG_P;
pub(in crate::hal) const PTE_FLAG_RW: u32 = COMMON_FLAG_RW;
pub(in crate::hal) const PTE_FLAG_US: u32 = COMMON_FLAG_US;
pub(in crate::hal) const PTE_FLAG_PWT: u32 = COMMON_FLAG_PWT;
pub(in crate::hal) const PTE_FLAG_PCD: u32 = COMMON_FLAG_PCD;
pub(in crate::hal) const PTE_FLAG_A: u32 = COMMON_FLAG_A;
pub(in crate::hal) const PTE_FLAG_D: u32 = COMMON_FLAG_D;
pub(in crate::hal) const PTE_FLAG_PAT: u32 = 1 << 7;
pub(in crate::hal) const PTE_FLAG_G: u32 = COMMON_FLAG_G;

pub(in crate::hal) const PAGE_SIZE: usize = 4096;
pub(in crate::hal) const ENTRY_SIZE: u32 = 4;
pub(in crate::hal) const ENTRY_COUNT: usize = 1024;

pub(in crate::hal) const KERNEL_PDE_START: u32 = 768;
pub(in crate::hal) const KERNEL_PDE_COUNT: usize = ENTRY_COUNT - KERNEL_PDE_START as usize - 1;

pub(in crate::hal) const SCRATCH_PDE: u32 = KERNEL_PDE_START + KERNEL_PDE_COUNT as u32 - 1;
pub(in crate::hal) const SCRATCH_PTE: u32 = ENTRY_COUNT as u32 - 1;

pub(in crate::hal) const MAX_MEMORY_PER_PTE: usize = PAGE_SIZE;
pub(in crate::hal) const MAX_MEMORY_PER_PDE: usize = MAX_MEMORY_PER_PTE * ENTRY_COUNT;
pub(in crate::hal) const KERNEL_AREA_SIZE: usize = MAX_MEMORY_PER_PDE * KERNEL_PDE_COUNT;

// PDE for recursive mapping the PD itself
pub(in crate::hal) const PAGE_DIR_PDE: u32 = ENTRY_COUNT as u32 - 1;

type PageTable = [u32; ENTRY_COUNT];

const ENTRY_BIT_MASK: u32 = 0x3ff;

const OFFSET_BIT_OFFSET: u32 = 0;
const OFFSET_BIT_MASK: u32 = 0xfff;

const PTE_BIT_OFFSET: u32 = 12;
const PTE_BIT_MASK: u32 = ENTRY_BIT_MASK << PTE_BIT_OFFSET;

const PDE_BIT_OFFSET: u32 = 22;
const PDE_BIT_MASK: u32 = ENTRY_BIT_MASK << PDE_BIT_OFFSET;

const fn make_virtaddr(pde: u32, pte: u32, offset: u32) -> usize {
    ((pde as usize) << PDE_BIT_OFFSET)
        | ((pte as usize) << PTE_BIT_OFFSET)
        | ((offset as usize) << OFFSET_BIT_OFFSET)
}

const PAGEDIR_PD_BASE: usize = make_virtaddr(PAGE_DIR_PDE, PAGE_DIR_PDE, 0);
const fn pagedir_pt_base(pde: u32) -> usize {
    make_virtaddr(PAGE_DIR_PDE, pde, 0)
}
fn read_pde(idx: u32) -> u32 {
    unsafe { read_volatile((PAGEDIR_PD_BASE as *mut u32).add(idx as usize)) }
}
fn write_pde(idx: u32, value: u32) {
    unsafe { write_volatile((PAGEDIR_PD_BASE as *mut u32).add(idx as usize), value) }
}
fn pte_addr(pde_idx: u32, pte_idx: u32) -> usize {
    let table = pagedir_pt_base(pde_idx) as *const u32;
    unsafe { table.add(pte_idx as usize).addr() }
}
fn read_pte(pde_idx: u32, pte_idx: u32) -> u32 {
    let table = pagedir_pt_base(pde_idx) as *const u32;
    unsafe { read_volatile(table.add(pte_idx as usize)) }
}
fn write_pte(pde_idx: u32, pte_idx: u32, value: u32) {
    let table = pagedir_pt_base(pde_idx) as *mut u32;
    unsafe { write_volatile(table.add(pte_idx as usize), value) }
}

const KERNEL_SPACE_BASE: usize = make_virtaddr(KERNEL_PDE_START, 0, 0);
pub(in crate::hal) const SCRATCH_MAP_BASE: usize = make_virtaddr(SCRATCH_PDE, SCRATCH_PTE, 0);
fn kernel_image_address_start() -> usize {
    super::sections::kernel_virtual_address_start()
}
fn kernel_image_address_end() -> usize {
    super::sections::kernel_virtual_address_end()
}
pub(super) fn kernel_vm_start() -> usize {
    kernel_image_address_end()
}
pub(super) const fn kernel_vm_end() -> usize {
    SCRATCH_MAP_BASE - 1
}
fn pde_index(ptr: *mut u8) -> u32 {
    (ptr.addr() as u32 & PDE_BIT_MASK) >> PDE_BIT_OFFSET
}
fn pte_index(ptr: *mut u8) -> u32 {
    (ptr.addr() as u32 & PTE_BIT_MASK) >> PTE_BIT_OFFSET
}
fn assert_addr_valid(addr: usize, count: usize) {
    assert_ne!(addr, 0, "address must be non-null");
    let byte_count = count
        .checked_mul(PAGE_SIZE)
        .expect("page count overflows address size");
    assert!(
        addr.checked_add(byte_count).is_some(),
        "address range overflows"
    );
}
fn assert_physical_addr_valid(addr: klib::PhysicalPtr, count: usize) {
    assert!(!addr.is_null(), "address must be non-null");
    let byte_count = count
        .checked_mul(PAGE_SIZE)
        .expect("page count overflows address size");
    assert!(
        addr.checked_add(byte_count).is_some(),
        "address range overflows"
    );
}
fn create_pd(pde: u32) -> Result<(), &'static str> {
    let alloc = kglobals().pmm.lock().malloc(1)?;
    let addr = alloc.base.addr();
    let _size = alloc.page_count;
    write_pde(pde, addr as u32 | PDE_FLAG_P | PDE_FLAG_RW | PDE_FLAG_US);
    unsafe { super::invlpg(pte_addr(pde, 0)) };
    for i in 0..1024 {
        write_pte(pde, i, 0);
    }
    // Flush TLB just to be safe ===============================================
    for i in 0..ENTRY_COUNT {
        unsafe { super::invlpg(make_virtaddr(pde, i as u32, 0)) };
    }
    Ok(())
}
fn map_single_page(
    virt: *mut u8,
    phys: klib::PhysicalPtr,
    flags: klib::mem::MapFlags,
    user_access: bool,
) {
    let pde = pde_index(virt);
    let pte = pte_index(virt);
    let old_pte = read_pte(pde, pte);
    let mut should_flush = false;
    if old_pte & PTE_FLAG_P != 0 {
        // See if we need to invalidate old TLB ================================
        if (old_pte & PTE_FLAG_RW) != 0 && !flags.write {
            should_flush = true;
        }
        if (old_pte & PTE_FLAG_US) != 0 && !user_access {
            should_flush = true;
        }
        let old_addr = old_pte & !0xfff;
        if old_addr != phys.addr() as u32 {
            should_flush = true;
        }
    }
    let mut pte_val = phys.addr() as u32 | PTE_FLAG_P;
    if flags.write {
        pte_val |= PTE_FLAG_RW;
    }
    if flags.cache_inhibit {
        pte_val |= PTE_FLAG_PCD;
    }
    if user_access {
        pte_val |= PTE_FLAG_US;
    }
    write_pte(pde, pte, pte_val);
    if should_flush {
        unsafe { super::invlpg(virt.addr()) };
    }
}
fn check_presence(virt: *mut u8) -> Result<(), &'static str> {
    let pde = pde_index(virt);
    let pte = pte_index(virt);
    let pd_entry = read_pde(pde);
    if (pd_entry & PDE_FLAG_P) == 0 {
        return Err("non-present PDE");
    }
    let old_pte = read_pte(pde, pte);
    if (old_pte & PTE_FLAG_P) == 0 {
        return Err("non-present PTE");
    }
    Ok(())
}
fn remap_single_page(virt: *mut u8, flags: klib::mem::MapFlags, user_access: bool) {
    let pde = pde_index(virt);
    let pte = pte_index(virt);
    let old_pte = read_pte(pde, pte);
    let mut should_flush = false;
    // See if we need to invalidate old TLB ====================================
    if (old_pte & PTE_FLAG_RW) != 0 && !flags.write {
        should_flush = true;
    }
    if (old_pte & PTE_FLAG_US) != 0 && !user_access {
        should_flush = true;
    }
    // Update the table ========================================================
    let mut pte_value = read_pte(pde, pte);
    pte_value &= !(0xfff & (!COMMON_FLAG_P));
    if flags.write {
        pte_value |= PTE_FLAG_RW;
    }
    if flags.cache_inhibit {
        pte_value |= PTE_FLAG_PCD;
    }
    if user_access {
        pte_value |= PTE_FLAG_US;
    }
    write_pte(pde, pte, pte_value);
    if should_flush {
        unsafe { super::invlpg(virt.addr()) };
    }
}

pub(in crate::hal) fn emulate(
    virt: *mut u8,
    flags: klib::mem::MapFlags,
    is_from_user: bool,
) -> Result<klib::PhysicalPtr, &'static str> {
    let pde = pde_index(virt);
    let pte = pte_index(virt);
    let pd_entry = read_pde(pde);
    if (pd_entry & PDE_FLAG_P) == 0 {
        return Err("non-present PDE");
    }
    if (pd_entry & PDE_FLAG_RW) == 0 && flags.write {
        return Err("non-writable PDE");
    }
    if (pd_entry & PDE_FLAG_US) == 0 && is_from_user {
        return Err("non-user PDE");
    }
    let pt_entry = read_pte(pde, pte);
    if (pt_entry & PTE_FLAG_P) == 0 {
        return Err("non-present PTE");
    }
    if (pt_entry & PTE_FLAG_RW) == 0 && flags.write {
        return Err("non-writable PTE");
    }
    if (pt_entry & PTE_FLAG_US) == 0 && is_from_user {
        return Err("non-user PTE");
    }
    Ok(klib::PhysicalPtr::new((pt_entry & !0xfff) as usize))
}
pub(in crate::hal) fn virtual_to_physical(
    virt: *mut u8,
) -> Result<klib::PhysicalPtr, &'static str> {
    let pde = pde_index(virt);
    let pte = pte_index(virt);
    let pd_entry = read_pde(pde);
    if (pd_entry & PDE_FLAG_P) == 0 {
        return Err("non-present PDE");
    }
    let pt_entry = read_pte(pde, pte);
    if (pt_entry & PTE_FLAG_P) == 0 {
        return Err("non-present PTE");
    }
    Ok(klib::PhysicalPtr::new(
        ((pt_entry & !0xfff) + (virt.addr() as u32 & 0xfff)) as usize,
    ))
}
pub(in crate::hal) fn map(
    virt_base: *mut u8,
    phys_base: klib::PhysicalPtr,
    page_count: usize,
    flags: klib::mem::MapFlags,
    user_access: bool,
) -> Result<(), &'static str> {
    assert!(!super::Hardware::irq_enabled());
    let mut ret;
    let mut pd_created = false;

    assert_addr_valid(virt_base.addr(), page_count);
    assert_physical_addr_valid(phys_base, page_count);
    assert!(toolbox::miscmath::is_aligned(phys_base.addr(), PAGE_SIZE));
    'attempt: {
        if !flags.read {
            ret = Err("attempted to create non-readable page");
            break 'attempt;
        }
        for i in 0..page_count {
            let current_virt = unsafe { virt_base.add(i * PAGE_SIZE) };
            let pde = pde_index(current_virt);
            let pd_entry = read_pde(pde);
            if pd_entry & PDE_FLAG_P != 0 {
                continue;
            }
            // Create new PD ===================================================
            ret = create_pd(pde);
            if ret.is_err() {
                break 'attempt;
            }
            pd_created = true;
        }
        for i in 0..page_count {
            let virt = unsafe { virt_base.add(i * PAGE_SIZE) };
            let phys = phys_base
                .checked_add(i * PAGE_SIZE)
                .expect("physical page address overflow");
            map_single_page(virt, phys, flags, user_access);
        }
        return Ok(());
    }
    if pd_created {
        // TODO: Clean-up unused PD entries
        println!("todo: clean-up unused pd entries");
    }
    ret
}
pub(in crate::hal) fn remap(
    virt_base: *mut u8,
    page_count: usize,
    flags: klib::mem::MapFlags,
    user_access: bool,
) -> Result<(), &'static str> {
    assert!(!super::Hardware::irq_enabled());
    assert_addr_valid(virt_base.addr(), page_count);
    if !flags.read {
        return Err("attempted to set non-readable");
    }
    for i in 0..page_count {
        let virt = unsafe { virt_base.add(i * PAGE_SIZE) };
        check_presence(virt)?;
    }

    for i in 0..page_count {
        let virt = unsafe { virt_base.add(i * PAGE_SIZE) };
        remap_single_page(virt, flags, user_access);
    }
    Ok(())
}
pub(in crate::hal) fn unmap(virt_base: *mut u8, page_count: usize) -> Result<(), &'static str> {
    assert!(!super::Hardware::irq_enabled());
    assert_addr_valid(virt_base.addr(), page_count);
    for i in 0..page_count {
        let virt = unsafe { virt_base.add(i * PAGE_SIZE) };
        check_presence(virt)?;
    }
    for i in 0..page_count {
        let current_virt_base = unsafe { virt_base.add(i * PAGE_SIZE) };
        let pde = pde_index(current_virt_base);
        let pte = pte_index(current_virt_base);
        write_pte(pde, pte, 0);
        unsafe { super::invlpg(current_virt_base.addr()) };
    }
    // TODO: Clean-up unused PD entries
    Ok(())
}
pub(in crate::hal) fn scratch_map(phys_base: klib::PhysicalPtr, cache_inhibit: bool) {
    assert!(!super::Hardware::irq_enabled());
    assert_physical_addr_valid(phys_base, 1);
    assert!(toolbox::miscmath::is_aligned(phys_base.addr(), PAGE_SIZE));
    let pde = SCRATCH_PDE;
    let pte = SCRATCH_PTE;
    let old_pte = read_pte(pde, pte);
    let mut should_flush = false;
    // See if we need to invalidate old TLB ====================================
    if (old_pte & PTE_FLAG_RW) != 0 {
        should_flush = true;
    }
    if (old_pte & !0xfff) != phys_base.addr() as u32 {
        should_flush = true;
    }
    // Update the table ========================================================
    let mut pte_value = phys_base.addr() as u32 | PTE_FLAG_P | PTE_FLAG_RW;
    if cache_inhibit {
        pte_value |= PTE_FLAG_PCD;
    }
    write_pte(pde, pte, pte_value);
    if should_flush {
        unsafe { super::invlpg(SCRATCH_MAP_BASE) };
    }
}
pub(in crate::hal) fn write_protect_kernel_text() {
    remap(
        super::sections::kernel_text_start() as *mut u8,
        (super::sections::kernel_text_end() - super::sections::kernel_text_start()) / PAGE_SIZE,
        klib::mem::MapFlags {
            read: true,
            ..Default::default()
        },
        false,
    )
    .unwrap();
}
pub(in crate::hal) fn write_protect_after_early_init() {
    remap(
        super::sections::kernel_ro_after_early_init_start() as *mut u8,
        (super::sections::kernel_ro_after_early_init_end()
            - super::sections::kernel_ro_after_early_init_start())
            / PAGE_SIZE,
        klib::mem::MapFlags {
            read: true,
            ..Default::default()
        },
        false,
    )
    .unwrap();
}

unsafe extern "C" {
    static hal_i586_stack_bottom_trap: u8;
}

pub(in crate::hal) fn init() {
    // Setup "stack bottom trap", which basically forces system to triple-fault immediately when kernel runs out of stack memory.
    unmap(
        core::ptr::addr_of!(hal_i586_stack_bottom_trap) as *mut u8,
        1,
    )
    .unwrap();
    // Unmap kernel VM region
    unmap(
        kernel_vm_start() as *mut u8,
        (kernel_vm_end() - kernel_vm_start() + 1) / PAGE_SIZE,
    )
    .unwrap();
}

pub(crate) struct AddressSpace {
    pde_list: Vec<u32>,
}

impl AddressSpace {
    pub(in crate::hal) fn new(base: *mut u8, page_count: usize, is_user: bool) -> Self {
        assert!(toolbox::miscmath::is_ptr_aligned(
            base,
            super::Hardware::PAGE_SIZE
        ));
        assert!(page_count > 0, "address space has no pages");
        let size = page_count
            .checked_mul(super::Hardware::PAGE_SIZE)
            .expect("address space size overflow");
        base.addr()
            .checked_add(size)
            .expect("address space range overflow");
        // A user address space starts without user mappings. Its page tables
        // are built while this address space is temporarily loaded. Kernel
        // PDEs live in the upper half and are shared, so they are not stored
        // here or affected by load().
        let mut pde_list = Vec::with_capacity(KERNEL_PDE_START as usize);
        if is_user {
            pde_list.resize(KERNEL_PDE_START as usize, 0);
        } else {
            for pde_off in 0..KERNEL_PDE_START {
                pde_list.push(read_pde(pde_off));
            }
        }
        Self { pde_list }
    }
}
impl klib::hal::HardwareAddressSpace for AddressSpace {
    fn current() -> Self {
        let mut pde_list = Vec::with_capacity(KERNEL_PDE_START as usize);
        for pde_off in 0..KERNEL_PDE_START {
            pde_list.push(read_pde(pde_off));
        }
        Self { pde_list }
    }

    fn snapshot_current(&mut self) {
        for (pde_off, pde) in self.pde_list.iter_mut().enumerate() {
            *pde = read_pde(pde_off as u32);
        }
    }
    fn load(&self) {
        for (pde_off, pde) in self.pde_list.iter().enumerate() {
            write_pde(pde_off as u32, *pde);
        }
    }
}
