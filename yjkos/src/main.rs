// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

#![no_std]
#![no_main]

mod syscall;

fn write_str(s: &str) {
    for byte in s.bytes() {
        syscall::write_char(byte);
    }
}

fn report_result(name: &str, result: Result<(), &'static str>) {
    write_str(name);
    match result {
        Ok(()) => write_str(": PASS\n"),
        Err(message) => {
            write_str(": FAIL (");
            write_str(message);
            write_str(")\n");
        }
    }
}

fn test_vm_alloc_and_free() -> Result<(), &'static str> {
    write_str("syscall::vm_alloc_object\n");
    let ptr = syscall::vm_alloc_object(
        None,
        4096,
        syscall::VMMapFlags {
            read: true,
            write: true,
            ..Default::default()
        },
    )?;

    if ptr.is_null() {
        return Err("allocation returned null");
    }

    // Exercise the permissions requested from the VM allocation syscall.
    unsafe {
        ptr.write_volatile(0x5a);
        if ptr.read_volatile() != 0x5a {
            let _ = syscall::vm_free_object(ptr);
            return Err("allocated memory did not retain written byte");
        }
    }

    write_str("syscall::vm_free_object\n");
    syscall::vm_free_object(ptr)
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    write_str("syscall tests\n");
    report_result("vm_alloc_object + vm_free_object", test_vm_alloc_and_free());

    loop {
        core::hint::spin_loop();
    }
}

#[panic_handler]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    loop {
        core::hint::spin_loop();
    }
}
