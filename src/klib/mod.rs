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
    pub trap_manager: traps::TrapManager<HW>,
    pub heap: irq::Mutex<HW, mem::Heap<HW>>,
    pub pmm: mem::PhysicalMemoryManager<HW>,
    pub vmm: mem::VirtualMemoryManager<HW>,
    pub sched: tasks::Scheduler<HW>,
}
impl<HW: hal::Hardware> KGlobals<HW> {
    pub const fn new() -> Self {
        Self {
            trap_manager: traps::TrapManager::new(),
            heap: irq::Mutex::new(mem::Heap::new()),
            pmm: mem::PhysicalMemoryManager::new(),
            vmm: mem::VirtualMemoryManager::new(),
            sched: tasks::Scheduler::new(),
        }
    }
}

impl<HW: hal::Hardware> Default for KGlobals<HW> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn kmain<HW: hal::Hardware>(kglobals: &mut KGlobals<HW>) -> ! {
    let kglobals = initialize(kglobals);
    // SAFETY: `initialize` returns after its mutable borrow has expired, and
    // KGlobals remains live for the lifetime of the kernel shell.
    let mut shell = diag::Shell::new();
    // SAFETY: `initialize` returned the live global pointer after its mutable
    // borrow expired; the kernel globals outlive this shell.
    unsafe { shell.run(kglobals) }
}

fn initialize<HW: hal::Hardware>(kglobals: &mut KGlobals<HW>) -> *mut KGlobals<HW> {
    println!("Welcome to YJKernel - Copyright (c) 2026 Oh Inseo");
    println!(
        "{}MiB allocatable memory",
        kglobals.pmm.get_total_mem_size() / (1024 * 1024)
    );
    print!("> Expanding heap..........................................................");
    if let Err(e) = mem::Heap::expand(&kglobals.heap, &mut kglobals.pmm, &mut kglobals.vmm) {
        println!("[FAIL]");
        println!("warning: failed to expand heap: {}", e);
    } else {
        println!("[ OK ]");
    }
    println!("> Initialize boot thread (ignore the 0k stack it says)");
    kglobals.sched.init_boot_thread();

    println!(">> DONE");

    kglobals
}
