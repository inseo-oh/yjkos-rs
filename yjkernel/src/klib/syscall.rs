// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::{klib, print, println};
use core::sync::atomic::{AtomicPtr, Ordering};

pub(crate) const SYSCALL_MAX_PARAM_COUNT: usize = 5;

pub(in crate::klib) const SYSCALL_WRITE_CHAR: u32 = 0x00;
pub(in crate::klib) const SYSCALL_VM_ALLOC_OBJECT: u32 = 0x01;
pub(in crate::klib) const SYSCALL_VM_ALLOC_OBJECT_AT: u32 = 0x02;
pub(in crate::klib) const SYSCALL_VM_FREE_OBJECT: u32 = 0x03;

pub(in crate::klib) const SYSCALL_RESULT_OK: u32 = 0x00000000;
pub(in crate::klib) const SYSCALL_RESULT_BAD_CALL_NUM: u32 = 0xC0000001;
pub(in crate::klib) const SYSCALL_RESULT_INVALID_ARGUMENT: u32 = 0xC0000002;
pub(in crate::klib) const SYSCALL_RESULT_OPERATION_FAILED: u32 = 0xC0000003;
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

static SYSCALL_KGLOBALS: AtomicPtr<()> = AtomicPtr::new(core::ptr::null_mut());

fn trap_handler<HW: klib::hal::Hardware>(
    _trap_num: usize,
    trap_frame: *mut <HW as klib::hal::Hardware>::TrapFrame,
) {
    // SAFETY: init installs the single global KGlobals before syscall traps are
    // enabled, and the kernel has one instance.
    let kglobals = SYSCALL_KGLOBALS.load(Ordering::Relaxed);
    assert!(!kglobals.is_null(), "syscall handler is not initialized");
    // SAFETY: init stores a reference to the static KGlobals before syscalls are
    // enabled; that reference remains valid forever.
    let kglobals = unsafe { &*kglobals.cast::<klib::KGlobals<HW>>() };

    let raw_args = HW::decode_syscall_args(trap_frame);
    let call = raw_args[0];
    let args = &raw_args[1..];
    let mut call_ret = SYSCALL_RESULT_OK;
    match call {
        //======================================================================
        // SYSCALL_WRITE_CHAR
        //======================================================================
        SYSCALL_WRITE_CHAR => print!("{}", args[0] as u8 as char),
        //======================================================================
        // SYSCALL_VM_ALLOC_OBJECT
        // args[0]: physical base (ignored when VM_ALLOC_FLAG_ALLOC_PHYS is set)
        // args[1]: size
        // args[2]: flags
        //======================================================================
        SYSCALL_VM_ALLOC_OBJECT => {
            let phys_base = args[0] as usize;
            let size = args[1] as usize;
            let raw_flags = args[2];
            if size == 0 || raw_flags & !VM_ALLOC_VALID_FLAGS != 0 {
                call_ret = SYSCALL_RESULT_INVALID_ARGUMENT;
            } else {
                let address_space = kglobals
                    .sched
                    .lock()
                    .running_thread()
                    .borrow()
                    .process()
                    .borrow()
                    .address_space();
                let flags = klib::mem::MapFlags {
                    read: raw_flags & VM_ALLOC_FLAG_READ != 0,
                    write: raw_flags & VM_ALLOC_FLAG_WRITE != 0,
                    exec: raw_flags & VM_ALLOC_FLAG_EXEC != 0,
                    no_cache: raw_flags & VM_ALLOC_FLAG_NO_CACHE != 0,
                };
                let phys_base = if raw_flags & VM_ALLOC_FLAG_ALLOC_PHYS != 0 {
                    None
                } else {
                    Some(klib::PhysicalPtr::new(phys_base))
                };
                match klib::mem::vmm::alloc_object::<HW>(&address_space, phys_base, size, flags) {
                    Ok(object) => call_ret = object.borrow().start.addr() as u32,
                    Err(e) => {
                        println!("syscall[SYSCALL_VM_ALLOC_OBJECT]: {}", e);
                        call_ret = SYSCALL_RESULT_OPERATION_FAILED;
                    }
                }
            }
        }
        //======================================================================
        // SYSCALL_VM_ALLOC_OBJECT_AT
        // args[0]: virtual base
        // args[2]: physical base (ignored when VM_ALLOC_FLAG_ALLOC_PHYS is set)
        // args[3]: size
        // args[3]: flags
        //======================================================================
        SYSCALL_VM_ALLOC_OBJECT_AT => {
            let virt_base = args[0] as *mut u8;
            let phys_base = args[1] as usize;
            let size = args[2] as usize;
            let raw_flags = args[3];
            if size == 0 || raw_flags & !VM_ALLOC_VALID_FLAGS != 0 {
                call_ret = SYSCALL_RESULT_INVALID_ARGUMENT;
            } else {
                let address_space = kglobals
                    .sched
                    .lock()
                    .running_thread()
                    .borrow()
                    .process()
                    .borrow()
                    .address_space();
                let flags = klib::mem::MapFlags {
                    read: raw_flags & VM_ALLOC_FLAG_READ != 0,
                    write: raw_flags & VM_ALLOC_FLAG_WRITE != 0,
                    exec: raw_flags & VM_ALLOC_FLAG_EXEC != 0,
                    no_cache: raw_flags & VM_ALLOC_FLAG_NO_CACHE != 0,
                };
                let phys_base = if raw_flags & VM_ALLOC_FLAG_ALLOC_PHYS != 0 {
                    None
                } else {
                    Some(klib::PhysicalPtr::new(phys_base))
                };
                match klib::mem::vmm::alloc_object_at::<HW>(
                    &address_space,
                    virt_base,
                    phys_base,
                    size,
                    flags,
                ) {
                    Ok(object) => call_ret = object.borrow().start.addr() as u32,
                    Err(e) => {
                        println!("syscall[SYSCALL_VM_ALLOC_OBJECT]: {}", e);
                        call_ret = SYSCALL_RESULT_OPERATION_FAILED;
                    }
                }
            }
        }
        //======================================================================
        // SYSCALL_VM_FREE_OBJECT
        // args[0]: virtual base returned by an allocation syscall
        //======================================================================
        SYSCALL_VM_FREE_OBJECT => {
            let virt_base = args[0] as usize;
            let address_space = kglobals
                .sched
                .lock()
                .running_thread()
                .borrow()
                .process()
                .borrow()
                .address_space();
            let object = address_space
                .borrow()
                .allocated_objects()
                .iter()
                .find(|object| object.borrow().start.addr() == virt_base)
                .cloned();
            if let Some(object) = object {
                klib::mem::vmm::free(&object, &mut kglobals.pmm.lock());
            } else {
                call_ret = SYSCALL_RESULT_INVALID_ARGUMENT;
            }
        }

        _ => {
            call_ret = SYSCALL_RESULT_BAD_CALL_NUM;
        }
    }
    HW::prepare_syscall_return(call_ret, trap_frame);
}

pub(in crate::klib) fn init<HW: klib::hal::Hardware>(
    trap_manager: &mut klib::traps::TrapManager<HW>,
    kglobals: &'static klib::KGlobals<HW>,
) {
    SYSCALL_KGLOBALS.store(kglobals as *const _ as *mut (), Ordering::Relaxed);
    trap_manager
        .register_trap(HW::SYSCALL_TRAP_NUMBER, trap_handler::<HW>)
        .unwrap();
}
