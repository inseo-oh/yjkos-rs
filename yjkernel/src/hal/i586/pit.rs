// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::{
    kglobals,
    klib::{self, devlib::HardwarePortIO as _},
};

const PIT_CH0_DATA_PORT: u16 = 0x40;
const PIT_MODE_PORT: u16 = 0x43;

const PIT_FREQ: u32 = 1193182;

const PIT_MODE_FLAG_SELECT_CH0: u8 = 0 << 6; //Channel select (Bit 7:6)
const PIT_MODE_FLAG_ACCESS_LSB_MSB: u8 = 3 << 4; // Access mode (Bit 5:4)
const PIT_MODE_FLAG_OP_RATEGEN: u8 = 2 << 1; // Operation mode (Bit 3:1)
const PIT_MODE_FLAG_BINMODE: u8 = 0; // Binary/BCD mode (Bit 0)

const PIT_IRQ: u8 = 0;
const FREQ_MILLIS: u32 = 1;

const fn counter_value_from_hz(hz: u32) -> u16 {
    (PIT_FREQ / hz) as u16
}

const fn hz_from_millis(millis: u32) -> u32 {
    1000 / millis
}

fn counter_from_millis(millis: u32) -> u16 {
    counter_value_from_hz(hz_from_millis(millis))
}

fn short_internal_delay() {
    super::HardwarePortIO::in8(PIT_MODE_PORT);
}

fn irq_handler(_irq_num: u8) {
    klib::ticktime::add(1);
    let switch = { kglobals().sched.lock().prepare_schedule() };
    if let Some(switch) = switch {
        switch.switch();
    }
}

pub fn init() {
    super::pic::mask_irq(PIT_IRQ);
    let initial_counter = counter_from_millis(FREQ_MILLIS);
    super::HardwarePortIO::out8(
        PIT_MODE_PORT,
        PIT_MODE_FLAG_SELECT_CH0
            | PIT_MODE_FLAG_ACCESS_LSB_MSB
            | PIT_MODE_FLAG_OP_RATEGEN
            | PIT_MODE_FLAG_BINMODE,
    );
    super::HardwarePortIO::out8(PIT_CH0_DATA_PORT, (initial_counter) as u8);
    short_internal_delay();
    super::HardwarePortIO::out8(PIT_CH0_DATA_PORT, (initial_counter >> 8) as u8);
    super::pic::register_handler(PIT_IRQ, irq_handler).unwrap();
    super::pic::unmask_irq(PIT_IRQ);
}
