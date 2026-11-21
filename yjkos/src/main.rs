#![no_std]
#![no_main]

const SYSCALL_TRAP_NUMBER: u8 = 0xa1;
const SYSCALL_WRITE_CHAR: u32 = 0;

unsafe fn syscall_write_char(chr: u8) {
    let mut syscall_number = SYSCALL_WRITE_CHAR;
    unsafe {
        core::arch::asm!(
            "int {vector}",
            vector = const SYSCALL_TRAP_NUMBER,
            inlateout("eax") syscall_number,
            in("ebx") chr as u32,
            options(nostack),
        );
    }
    let _ = syscall_number;
}

#[unsafe(no_mangle)]
pub extern "C" fn _start() -> ! {
    unsafe {
        syscall_write_char(b'H');
        syscall_write_char(b'i');
        syscall_write_char(b'!');
        syscall_write_char(b'\n');
    }

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
