// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::klib;
use core::{fmt, marker::PhantomData};

const REG_DATA: u8 = 0; // When LCR.DLAB=0
const REG_IER: u8 = 1; // When LCR.DLAB=0
const REG_DLL: u8 = 0; // When LCR.DLAB=1
const REG_DLH: u8 = 1; // When LCR.DLAB=1
const REG_IIR: u8 = 2;
const REG_LCR: u8 = 3;
const REG_MCR: u8 = 4;
const REG_LSR: u8 = 5;
const REG_MSR: u8 = 6;

// IER (Interrupt enable) ======================================================
const IER_FLAG_RX_AVAIL: u8 = 1 << 0;
const IER_FLAG_TX_EMPTY: u8 = 1 << 1;
const IER_FLAG_RX_STATUS: u8 = 1 << 2;
const IER_FLAG_MODEM_STATUS: u8 = 1 << 3;

// IIR (Interrupt identification) ==============================================
const IIR_FLAG_NO_INT_PENDING: u8 = 1 << 0;
const IIR_FLAG_MODEM_STATUS: u8 = 0 << 1;
const IIR_FLAG_TX_EMPTY: u8 = 1 << 1;
const IIR_FLAG_RX_AVAIL: u8 = 2 << 1;
const IIR_FLAG_RX_STATUS: u8 = 3 << 1;

// LSR (Line status) ===========================================================
const LSR_FLAG_DATA_READY: u8 = 1 << 0;
const LSR_FLAG_OVERRUN_ERR: u8 = 1 << 1;
const LSR_FLAG_PARITY_ERR: u8 = 1 << 2;
const LSR_FLAG_FRAMING_ERR: u8 = 1 << 3;
const LSR_FLAG_RECVED_BREAK: u8 = 1 << 4;
const LSR_FLAG_TX_HOLDING_REG_EMPTY: u8 = 1 << 5;
const LSR_FLAG_TX_SHIFT_REG_EMPTY: u8 = 1 << 6;

// MSR (Modem status) ==========================================================
const MSR_FLAG_CTS_DELTA: u8 = 1 << 0;
const MSR_FLAG_DSR_DELTA: u8 = 1 << 1;
const MSR_FLAG_RI_TRAILING_EDGE: u8 = 1 << 2;
const MSR_FLAG_DCD_DELTA: u8 = 1 << 3;
const MSR_FLAG_CTS: u8 = 1 << 4;
const MSR_FLAG_DSR: u8 = 1 << 5;
const MSR_FLAG_RI: u8 = 1 << 6;
const MSR_FLAG_DCD: u8 = 1 << 7;

// LCR (Line control) ==========================================================
const LCR_FLAG_WORD_LEN_FIVE: u8 = 0;
const LCR_FLAG_WORD_LEN_SIX: u8 = 1 << 0;
const LCR_FLAG_WORD_LEN_SEVEN: u8 = 2;
const LCR_FLAG_WORD_LEN_EIGHT: u8 = 3;

// When enabled: 5-bit -> 1.5 stop-bit, otherwise -> 2 stop-bit ================
const LCR_FLAG_MULTI_STOP_BITS: u8 = 1 << 2;
const LCR_FLAG_PARITY_ENABLE: u8 = 1 << 3;
const LCR_FLAG_PARITY_ODD: u8 = 0 << 4;
const LCR_FLAG_PARITY_EVEN: u8 = 1 << 4;

// Even parity . Parity is always 1, Odd parity . Parity is always 0 ===========
const LCR_FLAG_STICKY_PARITY: u8 = 1 << 5;

// Enter break condition by pulling Tx low
// (Receiving UART will see this as long stream of zeros)

const LCR_FLAG_SET_BREAK: u8 = 1 << 6;
const LCR_FLAG_DLAB: u8 = 1 << 7; // Divisor Latch Access Bit

// MCR (Modem control) =========================================================
const MCR_FLAG_DTR: u8 = 1 << 0;
const MCR_FLAG_RTS: u8 = 1 << 1;
const MCR_FLAG_OUT1: u8 = 1 << 2;
const MCR_FLAG_OUT2: u8 = 1 << 3;
const MCR_FLAG_LOOPBACK: u8 = 1 << 4;

pub struct SerialI8250<HWPIO: klib::devlib::HardwarePortIO> {
    master_clock: u32,
    base_addr: u16,
    _hwpio: PhantomData<HWPIO>,
}

impl<HWPIO: klib::devlib::HardwarePortIO> SerialI8250<HWPIO> {
    pub const fn new(base_addr: u16, master_clock: u32) -> Self {
        Self {
            base_addr,
            master_clock,
            _hwpio: PhantomData,
        }
    }
    pub fn init(&self, baud_rate: u32) -> Result<(), &'static str> {
        let divisor = self.get_divisor(baud_rate)?;
        self.write_ier(0);
        self.write_reg(
            REG_MCR,
            MCR_FLAG_DTR | MCR_FLAG_RTS | MCR_FLAG_OUT1 | MCR_FLAG_OUT2,
        );
        self.write_dl(divisor);
        self.write_reg(REG_LCR, LCR_FLAG_WORD_LEN_EIGHT);
        Ok(())
    }
    pub fn write_byte(&self, byte: u8) {
        self.wait_lsr_ready_to_send();
        self.write_data(byte);
    }
    pub fn read_byte(&self) -> u8 {
        self.wait_lsr_ready_to_recv();
        self.read_data()
    }
    pub fn try_read_byte(&self) -> Option<u8> {
        self.is_ready_to_recv().then(|| self.read_data())
    }
    fn get_divisor(&self, baudrate: u32) -> Result<u16, &'static str> {
        if baudrate == 0 || !self.master_clock.is_multiple_of(baudrate) {
            return Err("bad baudrate");
        }
        let divisor = self.master_clock / baudrate;
        if divisor == 0 || divisor > u16::MAX as u32 {
            return Err("bad baudrate");
        }
        Ok(divisor as u16)
    }
    fn write_reg(&self, reg_idx: u8, value: u8) {
        HWPIO::out8(self.base_addr + reg_idx as u16, value);
    }
    fn read_reg(&self, reg_idx: u8) -> u8 {
        HWPIO::in8(self.base_addr + reg_idx as u16)
    }
    fn set_dlab(&self) {
        let mut val = self.read_reg(REG_LCR);
        val |= LCR_FLAG_DLAB;
        self.write_reg(REG_LCR, val);
    }
    fn clear_dlab(&self) {
        let mut val = self.read_reg(REG_LCR);
        val &= !LCR_FLAG_DLAB;
        self.write_reg(REG_LCR, val);
    }
    fn write_ier(&self, value: u8) {
        self.clear_dlab();
        self.write_reg(REG_IER, value);
    }
    fn write_dl(&self, dl: u16) {
        self.set_dlab();
        self.write_reg(REG_DLL, dl as u8);
        self.write_reg(REG_DLH, (dl >> 8) as u8);
    }
    fn write_data(&self, value: u8) {
        self.clear_dlab();
        self.write_reg(REG_DATA, value);
    }
    fn read_data(&self) -> u8 {
        self.clear_dlab();
        self.read_reg(REG_DATA)
    }
    fn is_ready_to_send(&self) -> bool {
        (self.read_reg(REG_LSR) & LSR_FLAG_TX_HOLDING_REG_EMPTY) != 0
    }
    fn wait_lsr_ready_to_send(&self) {
        while !self.is_ready_to_send() {}
    }
    fn wait_lsr_ready_to_send_timeout(&self, max_attempts: u32) -> bool {
        for _ in 0..max_attempts {
            if self.is_ready_to_send() {
                return true;
            }
        }
        false
    }
    fn is_ready_to_recv(&self) -> bool {
        (self.read_reg(REG_LSR) & LSR_FLAG_DATA_READY) != 0
    }
    fn wait_lsr_ready_to_recv(&self) {
        while !self.is_ready_to_recv() {}
    }
    pub fn run_loopback_test(&self) -> Result<(), &'static str> {
        let old_mcr = self.read_reg(REG_MCR);
        let result = (|| {
            self.write_reg(REG_MCR, old_mcr | MCR_FLAG_LOOPBACK);
            if self.read_reg(REG_MCR) & MCR_FLAG_LOOPBACK == 0 {
                return Err("failed to write to MCR");
            }
            if !self.wait_lsr_ready_to_send_timeout(500_000) {
                return Err("loopback transmit timeout");
            }
            if self.is_ready_to_recv() {
                let _ = self.read_data();
            }
            self.write_data(0x69);
            let mut waited_counter = 0;
            while !self.is_ready_to_recv() {
                if waited_counter >= 500_000 {
                    return Err("loopback response timeout");
                }
                waited_counter += 1;
            }
            if self.read_data() != 0x69 {
                return Err("loopback data mismatch");
            }
            Ok(())
        })();

        self.write_reg(REG_MCR, old_mcr);
        result
    }
}
impl<HWPIO: klib::devlib::HardwarePortIO> fmt::Write for SerialI8250<HWPIO> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut last_byte: Option<u8> = None;
        for &byte in s.as_bytes() {
            if byte == b'\n' && last_byte != Some(b'\r') {
                // Turn it into CRLF
                self.write_byte(b'\r');
            }
            self.write_byte(byte);
            last_byte = Some(byte);
        }
        Ok(())
    }
}
