// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use alloc::vec::Vec;

use crate::{
    klib::{self, devlib::HardwarePortIO},
    println,
};

const CMD_PORT_MASTER: u16 = 0x20;
const DATA_PORT_MASTER: u16 = 0x21;
const CMD_PORT_SLAVE: u16 = 0xa0;
const DATA_PORT_SLAVE: u16 = 0xa1;

const CMD_READ_IRR: u8 = 0x0a;
const CMD_READ_ISR: u8 = 0x0b;
const CMD_EOI: u8 = 0x20;

const SLAVEPIN_ON_MASTER: u8 = 2;
const IRQS_PER_PIC: u8 = 8;
const IRQS_TOTAL: u8 = IRQS_PER_PIC * 2;
const PIC_VECTOR_BASE: u8 = 0x20;

const PIC_ICW1_FLAG_ICW4: u8 = 1 << 0;
const PIC_ICW1_FLAG_INIT: u8 = 1 << 4;
const PIC_ICW4_FLAG_8086MODE: u8 = 1 << 0;

fn read_irq_reg(read_cmd: u8) -> u16 {
    super::HardwarePortIO::out8(CMD_PORT_MASTER, read_cmd);
    super::HardwarePortIO::out8(CMD_PORT_SLAVE, read_cmd);
    ((super::HardwarePortIO::in8(CMD_PORT_SLAVE) as u16) << 8)
        | super::HardwarePortIO::in8(CMD_PORT_MASTER) as u16
}

fn read_irr() -> u16 {
    read_irq_reg(CMD_READ_IRR)
}

fn read_isr() -> u16 {
    read_irq_reg(CMD_READ_ISR)
}

fn get_irq_mask() -> u16 {
    let master_mask = super::HardwarePortIO::in8(DATA_PORT_MASTER);
    let slave_mask = super::HardwarePortIO::in8(DATA_PORT_SLAVE);
    ((slave_mask as u16) << 8) | master_mask as u16
}

fn set_irq_mask(mask: u16) {
    super::HardwarePortIO::out8(DATA_PORT_MASTER, mask as u8);
    super::HardwarePortIO::out8(DATA_PORT_SLAVE, (mask >> 8) as u8);
}

fn check_spurious_irq(irq: u8) -> bool {
    let is_real = (read_isr() & (1 << irq)) != 0;
    if irq == 7 {
        if !is_real {
            println!("pic: spurious IRQ {} received", irq);
        }
        return !is_real;
    }
    if irq == 15 {
        if !is_real {
            println!("pic: spurious IRQ {} received", irq);
            // If spurious IRQ occured on the slave PIC, master PIC has no idea that it is spurious at all.
            // So we must send EOI to the master.
            send_eoi(SLAVEPIN_ON_MASTER);
        }
        return !is_real;
    }
    false
}

pub(in crate::hal) fn send_eoi(irq: u8) {
    if irq >= IRQS_PER_PIC {
        super::HardwarePortIO::out8(CMD_PORT_SLAVE, CMD_EOI);
    }
    super::HardwarePortIO::out8(CMD_PORT_MASTER, CMD_EOI);
}

pub(in crate::hal) fn is_irq_masked(irq: u8) -> bool {
    (get_irq_mask() & (1 << irq)) != 0
}

pub(in crate::hal) fn mask_irq(irq: u8) {
    let _irq_disable = klib::irq::DisableGuard::<super::Hardware>::lock();
    set_irq_mask(get_irq_mask() | (1 << irq));
}

pub(in crate::hal) fn unmask_irq(irq: u8) {
    let _irq_disable = klib::irq::DisableGuard::<super::Hardware>::lock();
    set_irq_mask(get_irq_mask() & !(1 << irq));
}

// Each IRQ entry is a list of IRQ handlers.
static IRQS: [klib::irq::Mutex<super::Hardware, Vec<fn(irq_num: u8)>>; IRQS_TOTAL as usize] =
    [const { klib::irq::Mutex::new(Vec::new()) }; IRQS_TOTAL as usize];

// Default handler that just EOIs given IRQ
fn default_irq_handler(trap_num: usize, _trap_frame: *mut super::traps::TrapFrame) {
    let irq_num = (trap_num - PIC_VECTOR_BASE as usize) as u8;
    assert!(irq_num < IRQS_TOTAL);
    let is_suprious_irq = check_spurious_irq(irq_num);
    if !is_suprious_irq {
        send_eoi(irq_num);
    }
    if IRQS[irq_num as usize].lock().is_empty() {
        println!("no IRQ handler registered for IRQ {}", irq_num);
    } else {
        let handlers = IRQS[irq_num as usize].lock().clone();
        for handler in &handlers {
            handler(irq_num);
        }
    }
}

pub(in crate::hal) fn init(trap_manager: &mut klib::traps::TrapManager<super::Hardware>) {
    // ICW1 ====================================================================
    super::HardwarePortIO::out8(CMD_PORT_MASTER, PIC_ICW1_FLAG_INIT | PIC_ICW1_FLAG_ICW4);
    super::HardwarePortIO::out8(CMD_PORT_SLAVE, PIC_ICW1_FLAG_INIT | PIC_ICW1_FLAG_ICW4);
    // ICW2 ====================================================================
    super::HardwarePortIO::out8(DATA_PORT_MASTER, PIC_VECTOR_BASE);
    super::HardwarePortIO::out8(DATA_PORT_SLAVE, PIC_VECTOR_BASE + IRQS_PER_PIC);
    // ICW3 ====================================================================
    super::HardwarePortIO::out8(DATA_PORT_MASTER, 1 << SLAVEPIN_ON_MASTER);
    super::HardwarePortIO::out8(DATA_PORT_SLAVE, SLAVEPIN_ON_MASTER);
    // ICW4 ====================================================================
    super::HardwarePortIO::out8(DATA_PORT_MASTER, PIC_ICW4_FLAG_8086MODE);
    super::HardwarePortIO::out8(DATA_PORT_SLAVE, PIC_ICW4_FLAG_8086MODE);
    // Setup default PIC handler ===============================================
    for i in 0..8 {
        trap_manager
            .register_trap((PIC_VECTOR_BASE + i) as usize, default_irq_handler)
            .unwrap();
        trap_manager
            .register_trap(
                (PIC_VECTOR_BASE + IRQS_PER_PIC + i) as usize,
                default_irq_handler,
            )
            .unwrap();
    }
    // Disable IRQs except for IRQ2(which is connected to slave PIC) ===========
    set_irq_mask(!(1 << 2));
}

pub(in crate::hal) fn register_handler(
    irq_num: u8,
    callback: fn(irq_num: u8),
) -> Result<(), &'static str> {
    if IRQS_TOTAL <= irq_num {
        return Err("IRQ number is outside of range");
    }
    IRQS[irq_num as usize].lock().push(callback);
    Ok(())
}
