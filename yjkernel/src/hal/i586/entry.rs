// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::{
    kglobals,
    klib::{self, hal::Hardware},
    print, println,
};
use core::arch::global_asm;

#[unsafe(no_mangle)]
extern "C" fn hal_i586_init(magic: u32, multiboot_info: u32) -> ! {
    {
        let serial = super::SERIAL.lock();
        _ = serial.init(115200);
    }
    klib::console::set_writer(&super::SERIAL_WRITER);
    klib::console::set_reader(&super::SERIAL_READER);
    println!();
    println!("Initializing YJKernel i586 HAL. Please wait...");

    print!("> Initialize MMU..........................................................");
    super::mmu::init();
    println!("[DONE]");

    print!("> Write-protect kernel .text..............................................");
    super::mmu::write_protect_kernel_text();
    println!("[DONE]");

    // CR0.WP should've been enabled during early boot process, but if it isn't, the CPU probably doesn't support the feature.
    if (super::read_cr0() & (1 << 16)) == 0 {
        println!("warning: CR0.WP does not work: kernel can't be write-protected!!!");
    }

    print!("> Initialize GDT..........................................................");
    super::gdt::init();
    println!("[DONE]");

    print!("> Initialize IDT..........................................................");
    super::idt::init();
    println!("[DONE]");

    print!("> Write-protect more data.................................................");
    super::mmu::write_protect_after_early_init();
    println!("[DONE]");

    print!("> Initialize traps........................................................");
    super::traps::init();
    println!("[DONE]");

    print!("> Load GDT to CPU.........................................................");
    unsafe { super::gdt::load() };
    println!("[DONE]");

    print!("> Reload CPU segment selectors............................................");
    unsafe { super::gdt::reload_selectors() };
    println!("[DONE]");

    print!("> Load IDT to CPU.........................................................");
    unsafe { super::idt::load() };
    println!("[DONE]");

    println!("> Processing boot information from bootloader");
    if magic != super::bootinfo::MULTIBOOT_BOOTLOADER_MAGIC {
        println!(">> Bad multiboot magic value found!");
        println!(
            "expected: {:08x}",
            super::bootinfo::MULTIBOOT_BOOTLOADER_MAGIC
        );
        println!("     got: {:08x}", magic);
    } else {
        unsafe {
            super::bootinfo::process(
                klib::PhysicalPtr::new(multiboot_info as usize),
                &mut kglobals().pmm.lock(),
            );
        }
        println!(">> DONE");
    }
    // NOTE: After this point, we shouldn't rely on multiboot stuff anymore,
    //       as new PMM allocations can overwrite them (unless we copied them of course).

    print!("> Initialize PIC..........................................................");
    {
        let mut trap_manager = kglobals().trap_manager.lock();
        super::pic::init(&mut trap_manager);
    }
    println!("[DONE]");

    print!("> Initialize PIT..........................................................");
    super::pit::init();
    println!("[DONE]");

    print!("> Enable IRQs.............................................................");
    super::Hardware::irq_enable();
    println!("[DONE]");

    println!("");
    println!("================================================================================");
    println!("HAL initialization complete.");
    println!("Transferring control to main kernel initialization code.");
    println!("================================================================================");
    println!("");

    klib::kmain(kglobals());
}

global_asm!(
r#"
.equ MULTIBOOT_HEADER_MAGIC, 0x1BADB002
.equ MULTIBOOT_PAGE_ALIGN,   0x00000001
.equ MULTIBOOT_MEMORY_INFO,  0x00000002
.equ MULTIBOOT_VIDEO_MODE,   0x00000004

.section .multiboot, "a"
.section .text.init, "ax"
.section .bss.init, "aw", @nobits

.equ MULTIBOOT_INFO_ALIGN, 0x00000004

.equ MB_FLAGS,    (MULTIBOOT_PAGE_ALIGN | MULTIBOOT_MEMORY_INFO)
.equ MB_CHECKSUM, -(MULTIBOOT_HEADER_MAGIC + MB_FLAGS)

/* Multiboot header */
.section .multiboot 
.align MULTIBOOT_INFO_ALIGN
.long MULTIBOOT_HEADER_MAGIC
.long MB_FLAGS
.long MB_CHECKSUM

/******************************************************************************/
.section .text.init
.global _start

.equ PAGING_FLAGS, ({mmu_common_flag_p} | {mmu_common_flag_rw})

_start:
    /* Setup initial bootstrap stack */
    mov esp, OFFSET hal_i586_early_stack_top

    /* Fill page table */
    /* -> Lower 2M directory contains code that we are running right now, so it must be 1:1 mapped */
    mov esi, 0x00000000
    xor edi, edi
10:
    mov ecx, esi
    or ecx, PAGING_FLAGS
    mov dword ptr [hal_i586_lower_2m_page_table + edi * {mmu_entry_size}], ecx
    add esi, {mmu_page_size}
    inc edi
    cmp edi, {mmu_entry_count}
    jne 10b
    /* -> Initialize kernel page table */
    mov esi, OFFSET __physaddr_kernel_start__
    xor edi, edi
10:
    /*
     * For now, we map everything since startning of the kernel.
     * (Kernel will reconfigure memory mapping later)
     */
    mov ecx, esi
    or ecx, PAGING_FLAGS
    mov dword ptr [hal_i586_kernel_page_table + edi * {mmu_entry_size}], ecx
    add esi, {mmu_page_size}
    inc edi
    cmp edi, ({mmu_kernel_pde_count} * {mmu_entry_count})
    jne 10b
    /* Initialize page directory */
    /* -> Lower 2M page table */
    mov edi, 0
    mov ecx, OFFSET hal_i586_lower_2m_page_table
    or ecx, PAGING_FLAGS
    mov dword ptr [hal_i586_page_dir + edi * {mmu_entry_size}], ecx
    /* -> Kernel page table */
    mov edi, {mmu_kernel_pde_start}
    mov ecx, OFFSET hal_i586_kernel_page_table
    or ecx, PAGING_FLAGS
10:
    mov dword ptr [hal_i586_page_dir + edi * {mmu_entry_size}], ecx
    add ecx, {mmu_page_size}
    inc edi
    cmp edi, {mmu_entry_count}
    jne 10b
    /* -> Map directory table itself to one of PDEs */
    mov edi, {mmu_hal_i586_page_dir_pde}
    mov ecx, OFFSET hal_i586_page_dir
    or ecx, PAGING_FLAGS
    mov dword ptr [hal_i586_page_dir + edi * {mmu_entry_size}], ecx

    /* Enable paging */
    mov esi, OFFSET hal_i586_page_dir
    mov cr3, esi
    mov esi, cr0
    or esi, 0x80010000 /* Enable CR0.PG and CR0.WP */
    mov cr0, esi

    /* Switch to new stack */
    mov esp, OFFSET hal_i586_stack_top

    /* Jump to main initialization code */
    mov ebp, 0
    sub esp, 8 /* Subtract 8 bytes for 16-byte alignment */
    push ebx
    push eax
    call hal_i586_init

	cli
1:	hlt
	jmp 1b

/******************************************************************************/
.section .bss.init
.align 16

hal_i586_early_stack_bottom:
.skip 4096
hal_i586_early_stack_top:

.align 4096
hal_i586_page_dir:
.skip {mmu_page_size}
hal_i586_page_dir_end:

.align 4096
hal_i586_lower_2m_page_table:
.skip {mmu_page_size}
hal_i586_lower_2m_page_table_end:

.align 4096
hal_i586_kernel_page_table:
.skip ({mmu_kernel_pde_count} * {mmu_page_size})
hal_i586_kernel_page_table_end:

/******************************************************************************/

.section .bss

.align 4096
/*
 * A "trap" page that will be intentionally unmapped to immediately trigger page fault on stack overflow.
 * It won't prevent kernel from triple-fault, but at least it will do so with CR2 pointing to recongnizable
 * address, which will make debugging with emulators like QEMU a bit easier.
 */
.global hal_i586_stack_bottom_trap
hal_i586_stack_bottom_trap:
.skip 4096
hal_i586_stack_bottom_trap_end:

hal_i586_stack_bottom:
.skip 16384
hal_i586_stack_top:

.section .text
"#,
    mmu_kernel_pde_count = const super::mmu::KERNEL_PDE_COUNT,
    mmu_entry_count = const super::mmu::ENTRY_COUNT,
    mmu_entry_size = const super::mmu::ENTRY_SIZE,
    mmu_hal_i586_page_dir_pde = const super::mmu::PAGE_DIR_PDE,
    mmu_kernel_pde_start = const super::mmu::KERNEL_PDE_START,
    mmu_page_size = const super::mmu::PAGE_SIZE,
    mmu_common_flag_p = const super::mmu::COMMON_FLAG_P,
    mmu_common_flag_rw = const super::mmu::COMMON_FLAG_RW,
);
