// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

pub(crate) mod console;
pub(crate) mod devlib;
pub(crate) mod hal;
pub(crate) mod irq;
pub(crate) mod mem;
pub(crate) mod syscall;
pub(crate) mod tasks;
pub(crate) mod ticktime;
pub(crate) mod traps;

pub(in crate::klib) mod diag;
pub(in crate::klib) mod elf_loader;
pub(in crate::klib) mod physptr;

use core::mem::forget;

pub(crate) use physptr::PhysicalPtr;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        { let _ = $crate::klib::console::write(::core::format_args!($($arg)*)); }
    };
}

#[macro_export]
macro_rules! println {
    () => {
        { let _ = $crate::print!("\n"); }
    };

    ($($arg:tt)*) => {
        {let _ = $crate::print!("{}\n", ::core::format_args!($($arg)*));}
    };
}

pub(crate) struct KGlobals<HW: hal::Hardware> {
    pub(crate) trap_manager: irq::Mutex<HW, traps::TrapManager<HW>>,
    pub(crate) heap: irq::Mutex<HW, toolbox::Heap>,
    pub(crate) pmm: irq::Mutex<HW, mem::PhysicalMemoryManager<HW>>,
    pub(crate) vmm: irq::Mutex<HW, mem::VirtualMemoryManager<HW>>,
    pub(crate) sched: irq::Mutex<HW, tasks::Scheduler<HW>>,
}
// SAFETY: the kernel is uniprocessor. Every mutable field is accessed through
// an IRQ mutex, which prevents both task preemption and interrupt reentry while
// a guard is held. This must be revisited before adding SMP support.
unsafe impl<HW: hal::Hardware> Sync for KGlobals<HW> {}

impl<HW: hal::Hardware> KGlobals<HW> {
    pub(crate) const fn new() -> Self {
        Self {
            trap_manager: irq::Mutex::new(traps::TrapManager::new()),
            heap: irq::Mutex::new(toolbox::Heap::new()),
            pmm: irq::Mutex::new(mem::PhysicalMemoryManager::new()),
            vmm: irq::Mutex::new(mem::VirtualMemoryManager::new()),
            sched: irq::Mutex::new(tasks::Scheduler::new()),
        }
    }
}

impl<HW: hal::Hardware> Default for KGlobals<HW> {
    fn default() -> Self {
        Self::new()
    }
}

pub(crate) fn kmain<HW: hal::Hardware>(kglobals: &'static KGlobals<HW>) -> ! {
    init(kglobals);
    let mut shell = diag::Shell::new(kglobals);
    shell.run()
}

static USERSPACE_INIT: &[u8] = include_bytes!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../target/latest-yjkos"
));

fn init<HW: hal::Hardware>(kglobals: &'static KGlobals<HW>) {
    println!("Welcome to YJKernel - Copyright (c) 2026 Oh Inseo");
    println!(
        "{}MiB allocatable memory",
        kglobals.pmm.lock().get_total_mem_size() / (1024 * 1024)
    );
    print!("> Expanding heap..........................................................");
    if let Err(e) = expand_heap(kglobals) {
        println!("[FAIL]");
        println!("warning: failed to expand heap: {}", e);
    } else {
        println!("[ OK ]");
    }
    print!("> Initialize syscall......................................................");
    syscall::init(&mut kglobals.trap_manager.lock(), kglobals);
    println!("[DONE]");
    println!("> Initialize initial process");
    let kernel_address_space = kglobals.vmm.lock().kernel_address_space();
    kglobals
        .sched
        .lock()
        .init_initial_process(kglobals, kernel_address_space)
        .expect("failed to initialize initial process");

    // Launch init process =====================================================

    println!("> Launching YJK/OS");
    let init_aspace = mem::vmm::AddressSpace::<HW>::new(
        elf_loader::USER_MODE_BASE as *mut u8,
        HW::KERNEL_SPACE_BASE_ADDRESS - elf_loader::USER_MODE_BASE,
        true,
    );
    let entry = elf_loader::load(USERSPACE_INIT, &init_aspace, &kglobals.vmm)
        .expect("invalid userspace init program");
    let user_process = tasks::process::Process::new(init_aspace);
    let thread = tasks::thread::Thread::new(
        kglobals,
        &user_process,
        tasks::thread::THREAD_STACK_SIZE,
        hal::ThreadStart::User { entry },
    )
    .expect("failed to create userspace init thread");
    let mut scheduler = kglobals.sched.lock();
    scheduler.queue(thread.clone());
    forget(thread);
    forget(user_process);
}

fn expand_heap<HW: hal::Hardware>(kglobals: &KGlobals<HW>) -> Result<(), &'static str> {
    let heap_size = kglobals.pmm.lock().get_total_mem_size();
    let object = {
        let mut vmm = kglobals.vmm.lock();
        mem::vmm::alloc(
            &vmm.kernel_address_space(),
            heap_size,
            mem::vmm::MapFlags {
                read: true,
                write: true,
                ..Default::default()
            },
        )?
    };
    let (start, size) = {
        let object = object.borrow();
        (object.start, object.size)
    };
    let mut heap_guard = kglobals.heap.lock();
    let heap = unsafe { core::pin::Pin::new_unchecked(&mut *heap_guard) };
    unsafe { heap.add_mem(start, size, HW::PAGE_SIZE) }
}
