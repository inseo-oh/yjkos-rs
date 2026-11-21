// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

#![no_std]
#![no_main]
extern crate alloc;

mod global_alloc;
mod hal;
mod klib;
mod panic;

static KGLOBALS: klib::KGlobals<hal::Hardware> = klib::KGlobals::new();
fn kglobals() -> &'static klib::KGlobals<hal::Hardware> {
    &KGLOBALS
}

#[unsafe(no_mangle)]
extern "C" fn trap_manager_trap(
    trap_num: usize,
    trap_frame: *mut <hal::Hardware as klib::hal::Hardware>::TrapFrame,
) {
    let trap_manager = &kglobals().trap_manager;
    let handler_count = { trap_manager.lock().handler_count(trap_num) };
    for index in 0..handler_count {
        let callback = { trap_manager.lock().handler_at(trap_num, index) };
        if let Some(callback) = callback {
            callback(trap_num, trap_frame);
        }
    }
}
