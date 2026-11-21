// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::{klib::hal::Hardware as _, println};
use core::arch::asm;

#[repr(C)]
struct FunctionStackFrame {
    next: u32,
    eip: u32,
}

fn with_frame(mut frame_address: u32) {
    println!("stack trace:");

    while frame_address != 0 {
        let frame_ptr = frame_address as *const FunctionStackFrame;
        if let Err(error) = super::Hardware::mmu_virtual_to_physical(frame_ptr.cast_mut().cast()) {
            println!(
                "  stackframe at {:#x} inaccessible ({}) - STOP.",
                frame_address, error
            );
            break;
        }
        let frame = unsafe { core::ptr::read_volatile(frame_ptr) };
        println!("  {:#x}", frame.eip);
        frame_address = frame.next;
    }
}

pub(super) fn for_trap_frame(trap_frame: &super::traps::TrapFrame) {
    println!("pc: {:#x}", trap_frame.eip);
    with_frame(trap_frame.ebp);
}

#[inline(never)]
pub(super) fn current() {
    let frame: u32;
    unsafe { asm!("mov {frame:e}, ebp", frame = out(reg) frame) };
    with_frame(frame);
}
