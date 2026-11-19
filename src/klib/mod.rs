pub mod bitmap;
pub mod console;
mod cstring;
pub mod devlib;
pub mod diag;
pub mod hal;
pub mod irq;
pub mod list;
pub mod mem;
pub mod miscmath;
pub mod physptr;
pub mod tasks;
pub mod ticktime;
pub mod traps;

pub use bitmap::Bitmap;
pub use list::UnsafeList;
pub use physptr::PhysicalPtr;

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => {
        {
            _ = $crate::klib::console::write(::core::format_args!($($arg)*))
        }
    };
}

#[macro_export]
macro_rules! println {
    () => {
        _ = $crate::print!("\n")
    };

    ($($arg:tt)*) => {
        _ = $crate::print!("{}\n", ::core::format_args!($($arg)*))
    };
}

pub struct KGlobals<HW: hal::Hardware> {
    pub trap_manager: irq::Mutex<HW, traps::TrapManager<HW>>,
    pub heap: irq::Mutex<HW, mem::Heap<HW>>,
    pub pmm: irq::Mutex<HW, mem::PhysicalMemoryManager<HW>>,
    pub vmm: irq::Mutex<HW, mem::VirtualMemoryManager<HW>>,
    pub sched: irq::Mutex<HW, tasks::Scheduler<HW>>,
}
// SAFETY: the kernel is uniprocessor. Every mutable field is accessed through
// an IRQ mutex, which prevents both task preemption and interrupt reentry while
// a guard is held. This must be revisited before adding SMP support.
unsafe impl<HW: hal::Hardware> Sync for KGlobals<HW> {}

impl<HW: hal::Hardware> KGlobals<HW> {
    pub const fn new() -> Self {
        Self {
            trap_manager: irq::Mutex::new(traps::TrapManager::new()),
            heap: irq::Mutex::new(mem::Heap::new()),
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

pub fn kmain<HW: hal::Hardware>(kglobals: &KGlobals<HW>) -> ! {
    initialize(kglobals);
    let mut shell = diag::Shell::new(kglobals);
    shell.run()
}

fn initialize<HW: hal::Hardware>(kglobals: &KGlobals<HW>) {
    println!("Welcome to YJKernel - Copyright (c) 2026 Oh Inseo");
    println!(
        "{}MiB allocatable memory",
        kglobals.pmm.lock().get_total_mem_size() / (1024 * 1024)
    );
    print!("> Expanding heap..........................................................");
    if let Err(e) = mem::Heap::expand(&kglobals.heap, &kglobals.pmm, &kglobals.vmm) {
        println!("[FAIL]");
        println!("warning: failed to expand heap: {}", e);
    } else {
        println!("[ OK ]");
    }
    println!("> Initialize initial process (ignore the 0k stack it says)");
    kglobals.sched.lock().init_initial_process();

    println!(">> DONE");
}
