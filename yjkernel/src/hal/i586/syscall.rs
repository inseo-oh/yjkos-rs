// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::klib;

pub fn decode_syscall_args(
    trap_frame: *mut super::traps::TrapFrame,
) -> [u32; klib::syscall::SYSCALL_MAX_PARAM_COUNT] {
    let tfr = unsafe { trap_frame.as_ref().unwrap() };
    [
        tfr.eax, tfr.ebx, tfr.ecx, tfr.edx, tfr.esi, tfr.edi, tfr.ebp,
    ]
}

pub fn prepare_syscall_return(result: u32, trap_frame: *mut super::traps::TrapFrame) {
    let tfr = unsafe { trap_frame.as_mut().unwrap() };
    tfr.eax = result;
}
