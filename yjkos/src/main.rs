// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

#![no_std]
#![no_main]

mod syscall;
mod vmm;

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

fn error_matches(actual: &str, expected: &str) -> bool {
    actual.len() == expected.len()
        && unsafe {
            toolbox::cstring::memcmp(actual.as_ptr(), expected.as_ptr(), actual.len()) == 0
        }
}

fn test_vmm_alloc_and_free() -> Result<(), &'static str> {
    let ptr = vmm::alloc::<u32>(syscall::VMMapFlags {
        read: true,
        write: true,
        ..Default::default()
    })?;
    if ptr.is_null() {
        return Err("allocation returned null");
    }

    unsafe {
        ptr.write_volatile(0x5a5a_c3c3);
        if ptr.read_volatile() != 0x5a5a_c3c3 {
            let _ = vmm::free(ptr);
            return Err("allocated memory did not retain written byte");
        }
    }

    vmm::free(ptr)
}

fn test_vmm_alloc_array_and_free() -> Result<(), &'static str> {
    const COUNT: usize = 4;
    let ptr = vmm::alloc_array::<u32>(
        COUNT,
        syscall::VMMapFlags {
            read: true,
            write: true,
            ..Default::default()
        },
    )?;
    if ptr.is_null() {
        return Err("array allocation returned null");
    }

    unsafe {
        for index in 0..COUNT {
            ptr.add(index).write_volatile((index as u32) + 10);
        }
        for index in 0..COUNT {
            if ptr.add(index).read_volatile() != (index as u32) + 10 {
                let _ = vmm::free(ptr);
                return Err("array memory did not retain written values");
            }
        }
    }

    vmm::free(ptr)
}

fn test_vmm_rejects_oversized_array() -> Result<(), &'static str> {
    match vmm::alloc_objects::<u32>(None, usize::MAX, syscall::VMMapFlags::default()) {
        Err(error) if error_matches(error, "count is too large") => Ok(()),
        Err(_) => Err("oversized array returned an unexpected error"),
        Ok(ptr) => {
            let _ = vmm::free(ptr);
            Err("oversized array allocation unexpectedly succeeded")
        }
    }
}

fn test_vmm_rejects_empty_array() -> Result<(), &'static str> {
    match vmm::alloc_array::<u32>(0, syscall::VMMapFlags::default()) {
        Err(error) if error_matches(error, "invalid argument") => Ok(()),
        Err(_) => Err("empty array returned an unexpected error"),
        Ok(ptr) => {
            let _ = vmm::free(ptr);
            Err("empty array allocation unexpectedly succeeded")
        }
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    write_str("vmm tests\n");
    report_result("vmm::alloc + vmm::free", test_vmm_alloc_and_free());
    report_result(
        "vmm::alloc_array + vmm::free",
        test_vmm_alloc_array_and_free(),
    );
    report_result(
        "vmm oversized array check",
        test_vmm_rejects_oversized_array(),
    );
    report_result("vmm empty array check", test_vmm_rejects_empty_array());

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
