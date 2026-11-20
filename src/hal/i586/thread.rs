use crate::{klib, println};
use core::slice;
pub struct Thread {
    saved_esp: *mut u32,
    restore_with_iret: bool,
    esp0_stack: super::gdt::ESP0Stack,
}

const STACK_IDX_EDI: usize = 0;
const STACK_IDX_ESI: usize = 1;
const STACK_IDX_EBX: usize = 2;
const STACK_IDX_EBP: usize = 3;
const STACK_IDX_IRET_SS: usize = 4;
const STACK_IDX_IRET_ESP: usize = 5;
const STACK_IDX_EFLAGS: usize = 6;
const STACK_IDX_IRET_CS: usize = 7;
const STACK_IDX_EIP: usize = 8;
const STACK_IDX_MAIN_RET_ADDR: usize = 9;
const STACK_IDX_MAIN_ARG0: usize = 10;
const STACK_ITEM_COUNT: usize = 11;

const STACK_MIN_SIZE: usize = STACK_ITEM_COUNT * size_of::<u32>();
const STACK_MAX_PADDING_SIZE: usize = 3 * size_of::<u32>();

fn init_saved_registers(stack: &mut [u32], esp: usize) {
    stack[esp + STACK_IDX_EDI] = 0;
    stack[esp + STACK_IDX_ESI] = 0;
    stack[esp + STACK_IDX_EBX] = 0;
    stack[esp + STACK_IDX_EBP] = 0;
}

fn exit_callback() {
    klib::tasks::thread::thread_entry_returned()
}

struct InitialStackValues {
    entry: u32,
    eflags: u32,
    code_segment: u32,
    stack_segment: u32,
    user_esp: Option<u32>,
    return_address: u32,
    argument: u32,
    padding_words: usize,
    restore_with_iret: bool,
}

fn init_stack(
    init_stack_base: *mut u32,
    init_stack_size: usize,
    start: klib::hal::ThreadStart,
) -> Thread {
    assert!(!init_stack_base.is_null(), "thread stack base is null");
    assert_eq!(
        init_stack_base.addr() % align_of::<u32>(),
        0,
        "thread stack base is misaligned"
    );
    assert!(
        init_stack_size.is_multiple_of(size_of::<u32>()),
        "thread stack size is not word-aligned"
    );
    assert!(
        init_stack_size <= isize::MAX as usize,
        "thread stack is too large"
    );
    let allocated_top = init_stack_base
        .addr()
        .checked_add(init_stack_size)
        .expect("thread stack address range overflow");
    let stack =
        unsafe { slice::from_raw_parts_mut(init_stack_base, init_stack_size / size_of::<u32>()) };

    let values = match start {
        klib::hal::ThreadStart::Kernel { entry, data } => {
            assert!(
                init_stack_size >= STACK_MIN_SIZE + STACK_MAX_PADDING_SIZE,
                "thread stack is too small for its initial context"
            );
            println!(
                "hal/thread: Creating thread with {}k stack and entry point {:#10x}",
                init_stack_size / 1024,
                entry as usize
            );
            // Reserve up to 12 bytes so the initial entry stack can meet the
            // i386 ABI's 16-byte alignment. `Vec<u32>` only guarantees 4-byte
            // alignment, so choose padding using the allocation's address.
            let top_padding_words = ((allocated_top - size_of::<u32>()) & 0xF) / size_of::<u32>();
            InitialStackValues {
                entry: entry as *const () as u32,
                eflags: super::EFLAGS_FLAG_IF,
                code_segment: super::gdt::GDT_KERNEL_CS as u32,
                stack_segment: super::gdt::GDT_KERNEL_DS as u32,
                user_esp: None,
                return_address: exit_callback as *const () as u32,
                argument: data as u32,
                padding_words: top_padding_words,
                restore_with_iret: false,
            }
        }
        klib::hal::ThreadStart::User { entry } => {
            assert!(
                init_stack_size >= STACK_MIN_SIZE,
                "thread stack is too small for its initial context"
            );
            InitialStackValues {
                entry: entry as u32,
                eflags: super::EFLAGS_FLAG_IF | 0x2,
                code_segment: super::gdt::GDT_USER_CS as u32,
                stack_segment: super::gdt::GDT_USER_DS as u32,
                user_esp: Some(allocated_top as u32 - 16),
                return_address: 0,
                argument: 0,
                padding_words: 0,
                restore_with_iret: true,
            }
        }
    };
    assert!(
        stack.len() >= STACK_ITEM_COUNT + values.padding_words,
        "thread stack is too small after alignment padding"
    );
    let esp = stack.len() - values.padding_words - STACK_ITEM_COUNT;
    init_saved_registers(stack, esp);
    let saved_esp = unsafe { stack.as_mut_ptr().add(esp) };
    stack[esp + STACK_IDX_IRET_SS] = values.stack_segment;
    stack[esp + STACK_IDX_IRET_ESP] = values.user_esp.unwrap_or(saved_esp as u32);
    stack[esp + STACK_IDX_EFLAGS] = values.eflags;
    stack[esp + STACK_IDX_IRET_CS] = values.code_segment;
    stack[esp + STACK_IDX_EIP] = values.entry;
    stack[esp + STACK_IDX_MAIN_RET_ADDR] = values.return_address;
    stack[esp + STACK_IDX_MAIN_ARG0] = values.argument;

    if !values.restore_with_iret {
        let entry_esp = unsafe { init_stack_base.add(esp + STACK_IDX_MAIN_RET_ADDR) } as usize;
        assert_eq!(entry_esp % 16, 12);
    }

    Thread {
        saved_esp,
        restore_with_iret: values.restore_with_iret,
        esp0_stack: super::gdt::ESP0Stack::new(),
    }
}

pub fn create(
    init_stack_base: *mut u32,
    init_stack_size: usize,
    start: klib::hal::ThreadStart,
) -> Thread {
    if init_stack_size == 0 {
        assert!(init_stack_base.is_null());
        assert!(matches!(start, klib::hal::ThreadStart::Kernel { .. }));
        // The boot thread already has a live stack. Its saved_esp is set the
        // first time it switches out, so it has no initial context frame.
        return Thread {
            saved_esp: core::ptr::null_mut(),
            restore_with_iret: false,
            esp0_stack: super::gdt::ESP0Stack::new(),
        };
    }
    init_stack(init_stack_base, init_stack_size, start)
}

pub unsafe fn switch(from: *mut Thread, to: *mut Thread, reload_address_space: bool) {
    assert!(!from.is_null());
    assert!(!to.is_null());
    super::gdt::set_esp0(unsafe { &mut (*to).esp0_stack });
    let from_saved_esp = unsafe { core::ptr::addr_of_mut!((*from).saved_esp) };
    let to_saved_esp = unsafe { (*to).saved_esp };
    let restore_with_iret = unsafe { (*to).restore_with_iret };
    if restore_with_iret {
        unsafe { (*to).restore_with_iret = false };
    }
    // SAFETY: The caller serializes access to both thread contexts and ensures
    // that the current stack is not otherwise borrowed across the switch.
    if restore_with_iret {
        unsafe {
            hal_i586_context_switch_iret(from_saved_esp, to_saved_esp, reload_address_space as u32)
        };
    } else {
        unsafe {
            hal_i586_context_switch(from_saved_esp, to_saved_esp, reload_address_space as u32)
        };
    }
}

unsafe extern "C" {
    fn hal_i586_context_switch(
        old_stack_ptr: *mut *mut u32,
        new_stack_ptr: *mut u32,
        reload_address_space: u32,
    );
    fn hal_i586_context_switch_iret(
        old_stack_ptr: *mut *mut u32,
        new_stack_ptr: *mut u32,
        reload_address_space: u32,
    );
}

// After saving registers, the stack is laid out as below, with higher addresses at the top:
//
//       Higher address
//       +44 | arg 3: reload_address_space
//       +40 | arg 2: new_stack_ptr       (new thread: init_data)
//       +36 | arg 1: old_stack_ptr       (new thread: exit_callback)
//       +32 | return EIP                 (new thread: init_main_func)
// EBP ->+28 | IRET_CS                    (new thread: initial CS value)
//       +24 | EFLAGS
//       +20 | IRET_ESP                   <-- ignored during normal RET
//       +16 | IRET_SS                    <-- ignored during normal RET
//       +12 | EBP                        <-- Saved EBP
//        +8 | EBX
//        +4 | ESI
// ESP -> +0 | EDI                        <-- where saved ESP points to
//       Lower address
core::arch::global_asm!(
    r#"
    .global hal_i586_context_switch
    .type hal_i586_context_switch, @function
hal_i586_context_switch:
    # Save EBP, EFLAGS, and reserve the IRET_ESP and IRET_SS slots.
    push ebp
    mov ebp, esp
    pushfd
    sub esp, 8 # Skip IRET_ESP, IRET_SS

    # Copy the original EBP into its saved-context slot.
    mov ecx, [esp + 12]
    push ecx
    # Replace the prologue's saved EBP with the current CS.
    xor ecx, ecx
    mov ecx, cs
    mov [ebp], ecx

    # Save EBX, ESI, EDI
    push ebx
    push esi
    push edi

    # At this point we are ready to save current ESP to current thread, and load new ESP.
    mov ecx, [ebp + 8]
    mov [ecx], esp
    # Cache the new stack pointer before switching address spaces. The outgoing
    # kernel stack may not be mapped in the incoming address space.
    mov edx, [ebp + 12]
    cmp dword ptr [ebp + 16], 0
    je 1f
    mov ecx, cr3
    mov cr3, ecx
1:
    mov esp, edx

    # Restore EDI, ESI, EBX, EBP
    pop edi
    pop esi
    pop ebx
    pop ebp

    # Skip IRET_SS, IRET_ESP
    add esp, 8

    # Restore EFLAGS
    popfd

    # Skip IRET_CS
    add esp, 4

    # Finally we return
    ret
    .size hal_i586_context_switch, .-hal_i586_context_switch
    "#
);

// Similar to above, but for entering user mode.
core::arch::global_asm!(
    r#"
    .global hal_i586_context_switch_iret
    .type hal_i586_context_switch_iret, @function
hal_i586_context_switch_iret:
    # Save EBP, EFLAGS, and reserve the IRET_ESP and IRET_SS slots.
    push ebp
    mov ebp, esp
    pushfd
    sub esp, 8 # Skip IRET_ESP, IRET_SS

    # Copy the original EBP into its saved-context slot.
    mov ecx, [esp + 12]
    push ecx
    # Replace the prologue's saved EBP with the current CS.
    xor ecx, ecx
    mov ecx, cs
    mov [ebp], ecx

    # Save EBX, ESI, EDI
    push ebx
    push esi
    push edi

    # At this point we are ready to save current ESP to current thread, and load new ESP.
    mov ecx, [ebp + 8]
    mov [ecx], esp
    # Cache the new stack pointer before switching address spaces. The outgoing
    # kernel stack may not be mapped in the incoming address space.
    mov edx, [ebp + 12]
    cmp dword ptr [ebp + 16], 0
    je 1f
    mov ecx, cr3
    mov cr3, ecx
1:
    mov esp, edx

    # Restore EDI, ESI, EBX, EBP
    pop edi
    pop esi
    pop ebx
    pop ebp

    # The shared saved-context layout stores SS, ESP, EFLAGS, CS, EIP.
    # Reverse those five words into the order IRET consumes: EIP, CS,
    # EFLAGS, ESP, SS.
    mov eax, [esp]
    xchg eax, [esp + 16]
    mov [esp], eax
    mov eax, [esp + 4]
    xchg eax, [esp + 12]
    mov [esp + 4], eax

    # User data selectors must be loaded before returning to CPL 3.
    mov ax, {user_ds}
    mov ds, ax
    mov es, ax
    mov fs, ax
    mov gs, ax

    # Restore EIP, CS, EFLAGS, user ESP, and SS.
    iretd
    .size hal_i586_context_switch_iret, .-hal_i586_context_switch_iret
    "#,
    user_ds = const super::gdt::GDT_USER_DS,
);
