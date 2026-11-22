// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

pub(crate) mod serial_i8250;

pub(crate) trait HardwarePortIO {
    fn in8(port: u16) -> u8;
    fn out8(port: u16, value: u8);
}
