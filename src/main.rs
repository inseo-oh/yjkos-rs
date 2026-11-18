#![no_std]
#![no_main]
extern crate alloc;

mod global_alloc;
mod hal;
mod klib;
mod panic;

use core::ptr::addr_of_mut;

static mut KGLOBALS: klib::KGlobals<hal::Hardware> = klib::KGlobals::new();
fn kglobals_mut() -> &'static mut klib::KGlobals<hal::Hardware> {
    unsafe { &mut *addr_of_mut!(KGLOBALS) }
}

#[unsafe(no_mangle)]
extern "C" fn trap_manager_trap(
    trap_num: usize,
    trap_frame: *mut <hal::Hardware as klib::hal::Hardware>::TrapFrame,
) {
    kglobals_mut().trap_manager.trap(trap_num, trap_frame);
}
