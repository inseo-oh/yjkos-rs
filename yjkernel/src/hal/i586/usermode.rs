// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use core::arch::asm;

pub unsafe fn enter_user_mode(entry: usize, user_stack: usize) -> ! {
    unsafe {
        asm!(
            "cli",
            "push {user_ds:e}",
            "pop ds",
            "push {user_ds:e}",
            "pop es",
            "push {user_ds:e}",
            "pop fs",
            "push {user_ds:e}",
            "pop gs",
            "push {user_ds:e}",
            "push {user_stack:e}",
            // Start user code with reserved EFLAGS bit 1 and interrupts
            // enabled, while keeping IOPL and DF clear.
            "push 0x202",
            "push {user_cs:e}",
            "push {entry:e}",
            "iretd",
            user_ds = in(reg) super::gdt::GDT_USER_DS as u32,
            user_cs = in(reg) super::gdt::GDT_USER_CS as u32,
            user_stack = in(reg) user_stack as u32,
            entry = in(reg) entry as u32,
            options(noreturn),
        );
    }
}
