// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::{klib, print};

pub const SYSCALL_MAX_PARAM_COUNT: usize = 7;

pub const SYSCALL_WRITE_CHAR: u32 = 0x00;
pub const SYSCALL_RESULT_OK: u32 = 0x00000000;
pub const SYSCALL_RESULT_BAD_CALL_NUM: u32 = 0x80000001;

fn trap_handler<HW: klib::hal::Hardware>(
    _trap_num: usize,
    trap_frame: *mut <HW as klib::hal::Hardware>::TrapFrame,
) {
    let raw_args = HW::decode_syscall_args(trap_frame);
    let call = raw_args[0];
    let args = &raw_args[1..];
    let mut call_ret = SYSCALL_RESULT_OK;
    match call {
        SYSCALL_WRITE_CHAR => print!("{}", args[0] as u8 as char),
        _ => {
            call_ret = SYSCALL_RESULT_BAD_CALL_NUM;
        }
    }
    HW::prepare_syscall_return(call_ret, trap_frame);
}

pub fn init<HW: klib::hal::Hardware>(trap_manager: &mut klib::traps::TrapManager<HW>) {
    trap_manager
        .register_trap(HW::SYSCALL_TRAP_NUMBER, trap_handler::<HW>)
        .unwrap();
}
