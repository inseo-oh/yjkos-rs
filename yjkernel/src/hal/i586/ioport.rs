//! x86 port-mapped I/O primitives.

/// Writes one byte to an I/O port.
///
/// # Safety
/// Must run at a privilege level allowed to access `port`.
#[inline]
pub(super) unsafe fn out8(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Writes one 16-bit word to an I/O port.
///
/// # Safety
/// Must run at a privilege level allowed to access `port`.
#[inline]
pub(super) unsafe fn out16(port: u16, value: u16) {
    unsafe {
        core::arch::asm!(
            "out dx, ax",
            in("dx") port,
            in("ax") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Writes one 32-bit double word to an I/O port.
///
/// # Safety
/// Must run at a privilege level allowed to access `port`.
#[inline]
pub(super) unsafe fn out32(port: u16, value: u32) {
    unsafe {
        core::arch::asm!(
            "out dx, eax",
            in("dx") port,
            in("eax") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}

/// Reads one byte from an I/O port.
///
/// # Safety
/// Must run at a privilege level allowed to access `port`.
#[inline]
pub(super) unsafe fn in8(port: u16) -> u8 {
    let value: u8;
    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

/// Reads one 16-bit word from an I/O port.
///
/// # Safety
/// Must run at a privilege level allowed to access `port`.
#[inline]
pub(super) unsafe fn in16(port: u16) -> u16 {
    let value: u16;
    unsafe {
        core::arch::asm!(
            "in ax, dx",
            in("dx") port,
            out("ax") value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

/// Reads one 32-bit double word from an I/O port.
///
/// # Safety
/// Must run at a privilege level allowed to access `port`.
#[inline]
pub(super) unsafe fn in32(port: u16) -> u32 {
    let value: u32;
    unsafe {
        core::arch::asm!(
            "in eax, dx",
            in("dx") port,
            out("eax") value,
            options(nomem, nostack, preserves_flags),
        );
    }
    value
}

/// Reads `output.len()` 16-bit words from an I/O port into `output`.
///
/// # Safety
/// Must run at a privilege level allowed to access `port`. The port must
/// provide the requested number of words.
#[inline]
pub(super) unsafe fn in16_rep(port: u16, output: &mut [u16]) {
    #[cfg(target_arch = "x86")]
    unsafe {
        core::arch::asm!(
            "rep insw",
            in("dx") port,
            inout("edi") output.as_mut_ptr() as u32 => _,
            inout("ecx") output.len() as u32 => _,
            options(nostack, preserves_flags),
        );
    }

    #[cfg(target_arch = "x86_64")]
    unsafe {
        core::arch::asm!(
            "rep insw",
            in("dx") port,
            inout("rdi") output.as_mut_ptr() => _,
            inout("rcx") output.len() => _,
            options(nostack, preserves_flags),
        );
    }
}
