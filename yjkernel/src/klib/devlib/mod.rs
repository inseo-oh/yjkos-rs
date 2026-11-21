// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

pub mod serial_i8250;

pub trait HardwarePortIO {
    fn in8(port: u16) -> u8;
    fn out8(port: u16, value: u8);
}
