const SYSCALL_TRAP_NUMBER: u8 = 0xa1;

const SYSCALL_WRITE_CHAR: u32 = 0x00;
const SYSCALL_VM_ALLOC_OBJECT: u32 = 0x01;
const SYSCALL_VM_ALLOC_OBJECT_AT: u32 = 0x02;
const SYSCALL_VM_FREE_OBJECT: u32 = 0x03;

const SYSCALL_RESULT_OK: u32 = 0x00000000;
const SYSCALL_RESULT_BAD_CALL_NUM: u32 = 0xC0000001;
const SYSCALL_RESULT_INVALID_ARGUMENT: u32 = 0xC0000002;
const SYSCALL_RESULT_OPERATION_FAILED: u32 = 0xC0000003;
const VM_ALLOC_FLAG_READ: u32 = 1 << 0;
const VM_ALLOC_FLAG_WRITE: u32 = 1 << 1;
const VM_ALLOC_FLAG_EXEC: u32 = 1 << 2;
const VM_ALLOC_FLAG_NO_CACHE: u32 = 1 << 3;
const VM_ALLOC_FLAG_ALLOC_PHYS: u32 = 1 << 4;
const VM_ALLOC_VALID_FLAGS: u32 = VM_ALLOC_FLAG_READ
    | VM_ALLOC_FLAG_WRITE
    | VM_ALLOC_FLAG_EXEC
    | VM_ALLOC_FLAG_NO_CACHE
    | VM_ALLOC_FLAG_ALLOC_PHYS;

unsafe fn syscall4(
    mut syscall_num: u32,
    arg1: u32,
    arg2: u32,
    arg3: u32,
    arg4: u32,
) -> Result<u32, &'static str> {
    unsafe {
        core::arch::asm!(
            "int {vector}",
            vector = const SYSCALL_TRAP_NUMBER,
            inlateout("eax") syscall_num,
            in("ebx") arg1 as u32,
            in("ecx") arg2 as u32,
            in("edx") arg3 as u32,
            in("edi") arg4 as u32,
            options(nostack),
        );
    }
    let res = syscall_num; // This is where result is returned to
    match res {
        SYSCALL_RESULT_BAD_CALL_NUM => Err("bad call number"),
        SYSCALL_RESULT_INVALID_ARGUMENT => Err("invalid argument"),
        SYSCALL_RESULT_OPERATION_FAILED => Err("operation failed"),
        _ => Ok(res),
    }
}
unsafe fn syscall3(syscall_num: u32, arg1: u32, arg2: u32, arg3: u32) -> Result<u32, &'static str> {
    unsafe { syscall4(syscall_num, arg1, arg2, arg3, 0) }
}
unsafe fn syscall2(syscall_num: u32, arg1: u32, arg2: u32) -> Result<u32, &'static str> {
    unsafe { syscall3(syscall_num, arg1, arg2, 0) }
}
unsafe fn syscall1(syscall_num: u32, arg1: u32) -> Result<u32, &'static str> {
    unsafe { syscall2(syscall_num, arg1, 0) }
}
unsafe fn syscall0(syscall_num: u32) -> Result<u32, &'static str> {
    unsafe { syscall1(syscall_num, 0) }
}

//==============================================================================
// Printing
//==============================================================================

pub fn write_char(chr: u8) {
    _ = unsafe { syscall1(SYSCALL_WRITE_CHAR, chr as u32) }
}

//==============================================================================
// VM control
//==============================================================================

#[derive(Default, Clone, Copy)]
pub struct VMMapFlags {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub no_cache: bool,
}
pub fn vm_alloc_object(
    phys_base: Option<usize>,
    size: usize,
    flags: VMMapFlags,
) -> Result<*mut u8, &'static str> {
    let mut flags_val = 0;
    if flags.write {
        flags_val |= VM_ALLOC_FLAG_WRITE;
    }
    if flags.read {
        flags_val |= VM_ALLOC_FLAG_READ;
    }
    if flags.exec {
        flags_val |= VM_ALLOC_FLAG_EXEC;
    }
    if phys_base.is_none() {
        flags_val |= VM_ALLOC_FLAG_ALLOC_PHYS;
    }
    unsafe {
        syscall3(
            SYSCALL_VM_ALLOC_OBJECT,
            phys_base.unwrap_or(0) as u32,
            size as u32,
            flags_val,
        )
    }
    .map(|r| r as *mut u8)
}
pub fn vm_alloc_object_at(
    virt_base: *mut u8,
    phys_base: Option<usize>,
    size: usize,
    flags: VMMapFlags,
) -> Result<*mut u8, &'static str> {
    let mut flags_val = 0;
    if flags.write {
        flags_val |= VM_ALLOC_FLAG_WRITE;
    }
    if flags.read {
        flags_val |= VM_ALLOC_FLAG_READ;
    }
    if flags.exec {
        flags_val |= VM_ALLOC_FLAG_EXEC;
    }
    if phys_base.is_none() {
        flags_val |= VM_ALLOC_FLAG_ALLOC_PHYS;
    }
    unsafe {
        syscall4(
            SYSCALL_VM_ALLOC_OBJECT_AT,
            virt_base as u32,
            phys_base.unwrap_or(0) as u32,
            size as u32,
            flags_val,
        )
    }
    .map(|r| r as *mut u8)
}
pub fn vm_free_object(virt_base: *mut u8) -> Result<(), &'static str> {
    unsafe { syscall1(SYSCALL_VM_FREE_OBJECT, virt_base as u32) }?;
    Ok(())
}
