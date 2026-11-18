#[cfg(test)]
use core::ptr::null_mut;

use crate::klib;

pub trait Hardware {
    const PAGE_SIZE: usize;
    const KERNEL_SPACE_BASE_ADDRESS: usize;
    const SCRATCH_MAP_BASE: *mut u8;

    type TrapFrame;
    type Thread;

    fn hcf() -> !;
    fn irq_enabled() -> bool;
    fn irq_disable();
    fn irq_enable();
    fn kernel_vm_start_address() -> usize;
    fn kernel_vm_end_address() -> usize;
    fn mmu_virtual_to_physical(virt: *mut u8) -> Result<klib::PhysicalPtr, &'static str>;
    fn mmu_emulate(
        virt: *mut u8,
        flags: klib::mem::vmm::MapFlags,
        is_from_user: bool,
    ) -> Result<klib::PhysicalPtr, &'static str>;
    fn mmu_map(
        virt_base: *mut u8,
        phys_base: klib::PhysicalPtr,
        page_count: usize,
        flags: klib::mem::vmm::MapFlags,
        user_access: bool,
    ) -> Result<(), &'static str>;
    fn mmu_unmap(base: *mut u8, count: usize) -> Result<(), &'static str>;
    fn mmu_flush_tlb_for(ptr: *mut u8);
    fn mmu_scratch_map(phys_base: klib::PhysicalPtr, no_cache: bool);
    fn stack_trace_for_trap_frame(trap_frame: &Self::TrapFrame);
    fn thread_create(
        init_stack_size: usize,
        init_main_func: fn(data: usize),
        init_data: usize,
    ) -> Self::Thread;
    /// # Safety
    /// `from` and `to` must be distinct, valid, stable context pointers. No Rust
    /// references to either context may remain live while this call switches
    /// stacks. Access to each context must be serialized: no other CPU or
    /// interrupt handler may access it at the same time. A later switch may
    /// access a context after control has transferred, even while this call is
    /// suspended.
    unsafe fn thread_switch(from: *mut Self::Thread, to: *mut Self::Thread);
}

#[cfg(test)]
#[derive(Default)]
pub struct TestHW;

#[cfg(test)]
std::thread_local! {
    static TEST_IRQ_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

#[cfg(test)]
#[derive(Default)]
pub struct TestHWThread {
    pub init_stack_size: usize,
    pub init_main_func: Option<fn(usize)>,
    pub init_data: usize,
    pub switch_count: usize,
}
#[cfg(test)]
impl Hardware for TestHW {
    const PAGE_SIZE: usize = 4096;
    const KERNEL_SPACE_BASE_ADDRESS: usize = 0;
    const SCRATCH_MAP_BASE: *mut u8 = null_mut();

    type TrapFrame = ();
    type Thread = TestHWThread;

    fn kernel_vm_start_address() -> usize {
        0
    }
    fn kernel_vm_end_address() -> usize {
        (1024 * 1024 * 16) - 1
    }

    fn hcf() -> ! {
        unreachable!();
    }
    fn irq_enabled() -> bool {
        TEST_IRQ_ENABLED.with(std::cell::Cell::get)
    }
    fn irq_disable() {
        TEST_IRQ_ENABLED.with(|enabled| enabled.set(false));
    }
    fn irq_enable() {
        TEST_IRQ_ENABLED.with(|enabled| enabled.set(true));
    }
    fn mmu_virtual_to_physical(_virt: *mut u8) -> Result<klib::PhysicalPtr, &'static str> {
        Err("unmapped")
    }
    fn mmu_emulate(
        _virt: *mut u8,
        _flags: klib::mem::vmm::MapFlags,
        _is_from_user: bool,
    ) -> Result<klib::PhysicalPtr, &'static str> {
        Err("unmapped")
    }
    fn mmu_map(
        _virt_base: *mut u8,
        _phys_base: klib::PhysicalPtr,
        _page_count: usize,
        _flags: klib::mem::vmm::MapFlags,
        _user_access: bool,
    ) -> Result<(), &'static str> {
        Ok(())
    }
    fn mmu_unmap(_base: *mut u8, _count: usize) -> Result<(), &'static str> {
        Ok(())
    }
    fn mmu_flush_tlb_for(_ptr: *mut u8) {}
    fn stack_trace_for_trap_frame(_trap_frame: &Self::TrapFrame) {}
    fn thread_create(
        init_stack_size: usize,
        init_main_func: fn(data: usize),
        init_data: usize,
    ) -> Self::Thread {
        TestHWThread {
            init_stack_size,
            init_main_func: Some(init_main_func),
            init_data,
            switch_count: 0,
        }
    }
    fn mmu_scratch_map(_phys_base: klib::PhysicalPtr, _no_cache: bool) {}
    unsafe fn thread_switch(from: *mut Self::Thread, to: *mut Self::Thread) {
        let from = unsafe { from.as_mut().expect("from context must not be null") };
        let to = unsafe { to.as_mut().expect("to context must not be null") };
        from.switch_count += 1;
        to.switch_count += 1;
    }
}
