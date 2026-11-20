#[cfg(test)]
use core::ptr::null_mut;

use crate::klib;

pub trait HardwareAddressSpace {
    fn current() -> Self
    where
        Self: Sized;
    fn snapshot_current(&mut self);
    fn load(&self);

    fn with_loaded<T>(&mut self, f: impl FnOnce() -> T) -> T
    where
        Self: Sized,
    {
        let current = Self::current();
        self.load();
        let result = f();
        self.snapshot_current();
        current.load();
        result
    }
}

#[derive(Clone, Copy)]
pub enum ThreadStart {
    /// Start a kernel function with its argument in the normal call frame.
    Kernel { entry: fn(usize), data: usize },
    /// IRET to a user instruction pointer using the thread's allocated stack.
    User { entry: usize },
}

pub trait Hardware: 'static {
    const PAGE_SIZE: usize;
    const KERNEL_SPACE_BASE_ADDRESS: usize;
    const SCRATCH_MAP_BASE: *mut u8;
    const SYSCALL_TRAP_NUMBER: usize;

    type TrapFrame;
    type Thread;
    type AddressSpace: HardwareAddressSpace;

    fn create_address_space(base: *mut u8, page_count: usize, is_user: bool) -> Self::AddressSpace;
    fn reboot();
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
    fn stack_trace();
    fn thread_create(
        _init_stack_base: *mut u32,
        init_stack_size: usize,
        start: ThreadStart,
    ) -> Self::Thread;
    /// # Safety
    /// `entry` and `user_stack` must be valid user-accessible addresses. The
    /// architecture's GDT, TSS, IDT, and trap handlers must be initialized so
    /// that user-mode traps can safely enter the kernel.
    unsafe fn enter_user_mode(entry: usize, user_stack: usize) -> !;
    /// # Safety
    /// `from` and `to` must be distinct, valid, stable context pointers. No Rust
    /// references to either context may remain live while this call switches
    /// stacks. Access to each context must be serialized: no other CPU or
    /// interrupt handler may access it at the same time. A later switch may
    /// access a context after control has transferred, even while this call is
    /// suspended.
    unsafe fn thread_switch(
        from: *mut Self::Thread,
        to: *mut Self::Thread,
        reload_address_space: bool,
    );
    fn decode_syscall_args(
        trap_frame: *mut Self::TrapFrame,
    ) -> [u32; klib::syscall::SYSCALL_MAX_PARAM_COUNT];
    fn prepare_syscall_return(result: u32, trap_frame: *mut Self::TrapFrame);
}

#[cfg(test)]
#[derive(Default)]
pub struct TestHW;

#[cfg(test)]
std::thread_local! {
    static TEST_IRQ_ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(true) };
}

pub struct TestHWAddressSpace;

impl HardwareAddressSpace for TestHWAddressSpace {
    fn current() -> Self {
        Self
    }

    fn snapshot_current(&mut self) {}
    fn load(&self) {}
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
    const SYSCALL_TRAP_NUMBER: usize = 0;

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
    fn stack_trace() {}
    fn stack_trace_for_trap_frame(_trap_frame: &Self::TrapFrame) {}
    fn thread_create(
        _init_stack_base: *mut u32,
        init_stack_size: usize,
        start: ThreadStart,
    ) -> Self::Thread {
        let (init_main_func, init_data) = match start {
            ThreadStart::Kernel { entry, data } => (Some(entry), data),
            ThreadStart::User { entry } => (None, entry),
        };
        TestHWThread {
            init_stack_size,
            init_main_func,
            init_data,
            switch_count: 0,
        }
    }
    unsafe fn enter_user_mode(_entry: usize, _user_stack: usize) -> ! {
        panic!("TestHW cannot enter user mode");
    }
    fn mmu_scratch_map(_phys_base: klib::PhysicalPtr, _no_cache: bool) {}
    unsafe fn thread_switch(
        from: *mut Self::Thread,
        to: *mut Self::Thread,
        _reload_address_space: bool,
    ) {
        let from = unsafe { from.as_mut().expect("from context must not be null") };
        let to = unsafe { to.as_mut().expect("to context must not be null") };
        from.switch_count += 1;
        to.switch_count += 1;
    }

    fn reboot() {}

    type AddressSpace = TestHWAddressSpace;

    fn create_address_space(
        _base: *mut u8,
        _page_count: usize,
        _is_user: bool,
    ) -> Self::AddressSpace {
        TestHWAddressSpace
    }

    fn decode_syscall_args(
        trap_frame: *mut Self::TrapFrame,
    ) -> [u32; klib::syscall::SYSCALL_MAX_PARAM_COUNT] {
        unimplemented!()
    }
    fn prepare_syscall_return(result: u32, trap_frame: *mut Self::TrapFrame) {}
}
