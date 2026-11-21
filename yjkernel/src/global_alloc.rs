use crate::{KGLOBALS, hal, klib};
use core::{
    alloc::{GlobalAlloc, Layout},
    pin::Pin,
    ptr::{NonNull, null_mut},
};

fn heap_mutex() -> &'static klib::irq::Mutex<hal::Hardware, klib::mem::Heap<hal::Hardware>> {
    &KGLOBALS.heap
}

struct GlobalAllocator;

#[global_allocator]
static GLOBAL_ALLOCATOR: GlobalAllocator = GlobalAllocator;

unsafe impl GlobalAlloc for GlobalAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let mut heap_guard = heap_mutex().lock();
        let mut heap = unsafe { Pin::new_unchecked(&mut *heap_guard) };

        heap.as_mut()
            .malloc_aligned(layout.size(), layout.align())
            .map_or(null_mut(), NonNull::as_ptr)
    }

    unsafe fn dealloc(&self, ptr: *mut u8, _layout: Layout) {
        if ptr.is_null() {
            return;
        }
        let mut heap_guard = heap_mutex().lock();
        let mut heap = unsafe { Pin::new_unchecked(&mut *heap_guard) };
        unsafe { heap.as_mut().free(NonNull::new_unchecked(ptr)) };
    }
}
