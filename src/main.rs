#![no_std]
#![no_main]
extern crate alloc;

mod hal;
mod klib;

use core::{
    alloc::{GlobalAlloc, Layout},
    pin::Pin,
    ptr::{NonNull, addr_of_mut, null_mut},
};

static mut KGLOBALS: klib::KGlobals<hal::Hardware> = klib::KGlobals::new();
fn kglobals_mut() -> &'static mut klib::KGlobals<hal::Hardware> {
    unsafe { &mut *addr_of_mut!(KGLOBALS) }
}

fn heap_mutex() -> &'static klib::irq::Mutex<hal::Hardware, klib::mem::Heap<hal::Hardware>> {
    // SAFETY: KGLOBALS is statically allocated, and heap access is serialized by this mutex.
    unsafe { &*core::ptr::addr_of!(KGLOBALS.heap) }
}

#[unsafe(no_mangle)]
extern "C" fn trap_manager_trap(
    trap_num: usize,
    trap_frame: *mut <hal::Hardware as klib::hal::Hardware>::TrapFrame,
) {
    kglobals_mut().trap_manager.trap(trap_num, trap_frame);
}

struct GlobalAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut heap_guard = heap_mutex().lock();
        let mut heap = unsafe { Pin::new_unchecked(&mut *heap_guard) };

        heap.as_mut()
            .malloc_aligned(layout.size(), layout.align())
            .map_or(null_mut(), NonNull::as_ptr)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }
        let mut heap_guard = heap_mutex().lock();
        let mut heap = unsafe { Pin::new_unchecked(&mut *heap_guard) };
        unsafe { heap.as_mut().free(NonNull::new_unchecked(ptr)) };
    }
}

#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    use crate::klib::hal::Hardware;

    hal::Hardware::irq_disable();
    println!();
    println!();
    println!("FATAL SOFTWARE FAILURE -- SYSTEM NEEDS TO RESTART.");
    println!("{}", _info.message());
    if let Some(location) = _info.location() {
        println!(
            "at: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }

    hal::Hardware::stack_trace();
    loop {
        core::hint::spin_loop();
    }
}
