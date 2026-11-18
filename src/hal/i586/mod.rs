mod bootinfo;
mod entry;
mod gdt;
mod idt;
mod ioport;
mod isr_entry;
mod mmu;
mod pic;
mod pit;
mod sections;
mod stacktrace;
mod thread;
mod traps;

use crate::klib;
use core::arch::asm;

pub static SERIAL: klib::irq::Mutex<
    Hardware,
    klib::devlib::serial_i8250::SerialI8250<HardwarePortIO>,
> = klib::irq::Mutex::new(klib::devlib::serial_i8250::SerialI8250::new(0x3f8, 115200));

fn write_serial(args: core::fmt::Arguments<'_>) -> core::fmt::Result {
    let mut serial = SERIAL.lock();
    core::fmt::Write::write_fmt(&mut *serial, args)
}

static SERIAL_WRITER: klib::console::ConsoleWriter = write_serial;

pub struct HardwarePortIO;
impl klib::devlib::HardwarePortIO for HardwarePortIO {
    fn in8(port: u16) -> u8 {
        unsafe { ioport::in8(port) }
    }

    fn out8(port: u16, value: u8) {
        unsafe { ioport::out8(port, value) }
    }
}

pub struct Hardware;
impl klib::hal::Hardware for Hardware {
    // IRQ control =============================================================
    fn irq_enabled() -> bool {
        (get_eflags() & EFLAGS_FLAG_IF) != 0
    }
    fn irq_disable() {
        unsafe { cli() };
    }
    fn irq_enable() {
        unsafe { sti() };
    }

    // MMU =====================================================================

    const PAGE_SIZE: usize = mmu::PAGE_SIZE;
    const SCRATCH_MAP_BASE: *mut u8 = mmu::SCRATCH_MAP_BASE as *mut u8;

    fn kernel_vm_start_address() -> usize {
        mmu::kernel_vm_start()
    }
    fn kernel_vm_end_address() -> usize {
        mmu::kernel_vm_end()
    }
    const KERNEL_SPACE_BASE_ADDRESS: usize =
        mmu::KERNEL_PDE_START as usize * mmu::MAX_MEMORY_PER_PDE;

    fn mmu_flush_tlb_for(ptr: *mut u8) {
        unsafe { invlpg(ptr.addr()) };
    }
    fn mmu_emulate(
        virt: *mut u8,
        flags: klib::mem::vmm::MapFlags,
        is_from_user: bool,
    ) -> Result<klib::PhysicalPtr, &'static str> {
        mmu::emulate(virt, flags, is_from_user)
    }
    fn mmu_virtual_to_physical(virt: *mut u8) -> Result<klib::PhysicalPtr, &'static str> {
        mmu::virtual_to_physical(virt)
    }
    fn mmu_map(
        virt_base: *mut u8,
        phys_base: klib::PhysicalPtr,
        page_count: usize,
        flags: klib::mem::vmm::MapFlags,
        user_access: bool,
    ) -> Result<(), &'static str> {
        mmu::map(virt_base, phys_base, page_count, flags, user_access)
    }
    fn mmu_unmap(base: *mut u8, count: usize) -> Result<(), &'static str> {
        mmu::unmap(base, count)
    }
    fn mmu_scratch_map(phys_base: klib::PhysicalPtr, no_cache: bool) {
        mmu::scratch_map(phys_base, no_cache);
    }

    // Threading ===============================================================

    type Thread = thread::Thread;

    fn thread_create(
        init_stack_size: usize,
        init_main_func: fn(data: usize),
        init_data: usize,
    ) -> Self::Thread {
        thread::create(init_stack_size, init_main_func, init_data)
    }

    unsafe fn thread_switch(from: *mut Self::Thread, to: *mut Self::Thread) {
        unsafe { thread::switch(from, to) };
    }

    // Other stuff =============================================================

    type TrapFrame = traps::TrapFrame;

    fn hcf() -> ! {
        loop {
            unsafe { cli() };
            unsafe { hlt() };
        }
    }

    fn stack_trace_for_trap_frame(trap_frame: &Self::TrapFrame) {
        stacktrace::for_trap_frame(trap_frame);
    }
    fn stack_trace() {
        stacktrace::current();
    }
}

const EFLAGS_FLAG_IF: u32 = 1 << 9;

#[inline]
pub fn get_eflags() -> u32 {
    let flags: u32;

    unsafe {
        asm!(
            "pushfd",
            "pop {flags:e}",
            flags = out(reg) flags,
        );
    }

    flags
}

#[inline]
pub unsafe fn cli() {
    unsafe {
        asm!("cli", options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub unsafe fn sti() {
    unsafe {
        asm!("sti", options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub unsafe fn hlt() {
    unsafe {
        asm!("hlt", options(nomem, nostack, preserves_flags));
    }
}

#[inline]
pub fn rdtsc() -> u64 {
    let low: u32;
    let high: u32;

    unsafe {
        asm!(
            "rdtsc",
            out("eax") low,
            out("edx") high,
            options(nomem, nostack, preserves_flags),
        );
    }

    ((high as u64) << 32) | low as u64
}

#[inline]
pub unsafe fn invlpg(address: usize) {
    unsafe {
        asm!(
            "invlpg [{addr}]",
            addr = in(reg) address,
            options(nostack, preserves_flags),
        );
    }
}

#[inline]
#[allow(unused)]
pub unsafe fn reload_cr3() {
    let value: u32;

    unsafe {
        asm!(
            "mov {value:e}, cr3",
            "mov cr3, {value:e}",
            value = out(reg) value,
            options(nostack, preserves_flags),
        );
    }
}

#[inline]
pub fn read_cr0() -> u32 {
    let cr0: u32;

    unsafe {
        asm!(
            "mov {flags:e}, cr0",
            flags = out(reg) cr0,
        );
    }

    cr0
}

#[inline]
pub fn read_cr2() -> u32 {
    let cr2: u32;

    unsafe {
        asm!(
            "mov {flags:e}, cr2",
            flags = out(reg) cr2,
        );
    }

    cr2
}
