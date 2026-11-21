// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::klib::{devlib::HardwarePortIO, hal::Hardware};

// https://wiki.osdev.org/Reboot
pub fn reboot() {
    // First we try 8042 KBC reset =============================================

    const KBC_DATA: u16 = 0x60;
    const KBC_CTRL: u16 = 0x64; // Control register(OUT) / Status port(IN)

    const KBC_OUTPUT_BUFFER_FULL: u8 = 1 << 0;
    const KBC_INPUT_BUFFER_FULL: u8 = 1 << 1;
    const KBC_RESET_SYSTEM: u8 = 0xfe; // Reset command

    super::Hardware::irq_disable();
    loop {
        if (super::HardwarePortIO::in8(KBC_CTRL) & KBC_OUTPUT_BUFFER_FULL) != 0 {
            super::HardwarePortIO::in8(KBC_DATA);
        }
        if (super::HardwarePortIO::in8(KBC_CTRL) & KBC_INPUT_BUFFER_FULL) == 0 {
            break;
        }
    }
    super::HardwarePortIO::out8(KBC_CTRL, KBC_RESET_SYSTEM);

    // Halt CPU if it didn't work ==============================================
    loop {
        super::Hardware::hcf();
    }
}
