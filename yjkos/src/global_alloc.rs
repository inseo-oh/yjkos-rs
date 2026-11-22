// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::{syscall, vmm};
use core::{
    alloc::{GlobalAlloc, Layout},
    pin::Pin,
    ptr::{NonNull, addr_of_mut, null_mut},
};

struct GlobalAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

static mut HEAP: toolbox::Heap = toolbox::Heap::new();

pub fn init() -> Result<(), &'static str> {
    let heap_size = 1024 * 1024 * 2;
    let object = {
        vmm::alloc_array::<u8>(
            heap_size,
            syscall::VMMapFlags {
                read: true,
                write: true,
                ..Default::default()
            },
        )?
    };
    let heap = unsafe { Pin::new_unchecked(&mut *addr_of_mut!(HEAP)) };
    unsafe { heap.add_mem(object, heap_size, 4096) }
}

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut heap = unsafe { Pin::new_unchecked(&mut *addr_of_mut!(HEAP)) };

        heap.as_mut()
            .malloc_aligned(layout.size(), layout.align())
            .map_or(null_mut(), NonNull::as_ptr)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }
        let mut heap = unsafe { Pin::new_unchecked(&mut *addr_of_mut!(HEAP)) };
        unsafe { heap.as_mut().free(NonNull::new_unchecked(ptr)) };
    }
}
