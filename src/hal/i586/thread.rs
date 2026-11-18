use alloc::vec::Vec;

use crate::{klib, println};

pub struct Thread {
    saved_esp: *mut u32,
    stack: Vec<u32>,
}

const STACK_IDX_EDI: usize = 0;
const STACK_IDX_ESI: usize = 1;
const STACK_IDX_EBX: usize = 2;
const STACK_IDX_EFLAGS: usize = 3;
const STACK_IDX_EBP: usize = 4;
const STACK_IDX_EIP: usize = 5;
const STACK_IDX_MAIN_RET_ADDR: usize = 6;
const STACK_IDX_MAIN_ARG0: usize = 7;
const STACK_ITEM_COUNT: usize = 8;

const STACK_MIN_SIZE: usize = STACK_ITEM_COUNT * size_of::<u32>();

fn exit_callback() {
    loop {
        core::hint::spin_loop();
    }
}

pub fn create(init_stack_size: usize, init_main_func: fn(data: usize), init_data: usize) -> Thread {
    let mut stack_size = init_stack_size;
    if stack_size < STACK_MIN_SIZE {
        stack_size = STACK_MIN_SIZE;
    }
    if !stack_size.is_multiple_of(size_of::<u32>()) {
        stack_size = klib::miscmath::align_usize_up(stack_size, size_of::<u32>());
    }
    assert!(stack_size.is_multiple_of(size_of::<u32>()));
    println!(
        "hal/thread: Creating thread with {}k stack and entry point {:#10x}",
        stack_size / 1024,
        init_main_func as usize
    );
    // Reserve up to 12 bytes so the initial entry stack can meet the i386 ABI's
    // 16-byte alignment. `Vec<u32>` only guarantees 4-byte alignment, so choose
    // the padding using the allocation's actual address.
    let mut stack = alloc::vec![0; stack_size / size_of::<u32>() + 3];
    let stack_base = stack.as_mut_ptr();
    let allocated_top = unsafe { stack_base.add(stack.len()) } as usize;
    let top_padding_words = ((allocated_top - size_of::<u32>()) & 0xF) / size_of::<u32>();
    let stack_top = stack.len() - top_padding_words;
    let esp = stack_top - STACK_ITEM_COUNT;
    stack[esp + STACK_IDX_MAIN_RET_ADDR] = exit_callback as *const () as u32;
    stack[esp + STACK_IDX_MAIN_ARG0] = init_data as u32;
    stack[esp + STACK_IDX_EIP] = init_main_func as *const () as u32;
    stack[esp + STACK_IDX_EBP] = 0;
    stack[esp + STACK_IDX_EFLAGS] = super::EFLAGS_FLAG_IF;
    stack[esp + STACK_IDX_EBX] = 0;
    stack[esp + STACK_IDX_ESI] = 0;
    stack[esp + STACK_IDX_EDI] = 0;
    let entry_esp = unsafe { stack_base.add(esp + STACK_IDX_MAIN_RET_ADDR) } as usize;
    assert_eq!(entry_esp % 16, 12);
    let saved_esp = unsafe { stack.as_mut_ptr().add(esp) };
    Thread { saved_esp, stack }
}

pub unsafe fn switch(from: *mut Thread, to: *mut Thread) {
    assert!(!from.is_null());
    assert!(!to.is_null());
    let from_saved_esp = unsafe { core::ptr::addr_of_mut!((*from).saved_esp) };
    let to_saved_esp = unsafe { (*to).saved_esp };
    // SAFETY: The caller serializes access to both thread contexts and ensures
    // that the current stack is not otherwise borrowed across the switch.
    unsafe { hal_i586_context_switch(from_saved_esp, to_saved_esp) };
}

unsafe extern "C" {
    fn hal_i586_context_switch(old_stack_ptr: *mut *mut u32, new_stack_ptr: *mut u32);
}

core::arch::global_asm!(
    r#"
    .global hal_i586_context_switch
    .type hal_i586_context_switch, @function
hal_i586_context_switch:
    push ebp
    mov ebp, esp
    pushfd
    push ebx
    push esi
    push edi

    mov ecx, [ebp + 8]
    mov [ecx], esp
    mov esp, [ebp + 12]

    pop edi
    pop esi
    pop ebx
    popfd
    pop ebp
    ret
    .size hal_i586_context_switch, .-hal_i586_context_switch
    "#
);
