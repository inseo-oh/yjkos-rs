use crate::klib;
use alloc::rc::Rc;
use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};

pub const THREAD_STACK_SIZE: usize = 1024 * 16;

// This is only for testing, for now.
static RETURNED_ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(crate) fn thread_entry_returned_count() -> usize {
    RETURNED_ENTRY_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn thread_entry_returned() -> ! {
    RETURNED_ENTRY_COUNT.fetch_add(1, Ordering::Relaxed);
    loop {
        core::hint::spin_loop();
    }
}

pub struct Thread<HW: klib::hal::Hardware> {
    arch_thread: HW::Thread,
    pub(super) waiting_mutex: Option<(
        klib::tasks::mutex::RawMutexHandle<HW>,
        klib::diag::SourceLocation,
    )>,
    pub(super) priority: i8,
    pub(super) shutdown: bool,
}

pub type ThreadHandle<HW> = Rc<RefCell<Thread<HW>>>;

impl<HW: klib::hal::Hardware> Thread<HW> {
    // Those init_ parameters are only valid for initial setup.
    // This of course applies to any new thread, but the boot thread is exception: It's thread for already running code.
    pub fn new(
        init_stack_size: usize,
        init_main_func: fn(data: usize),
        init_data: usize,
    ) -> ThreadHandle<HW> {
        Rc::new(RefCell::new(Self {
            arch_thread: HW::thread_create(init_stack_size, init_main_func, init_data),
            waiting_mutex: None,
            priority: 0,
            shutdown: false,
        }))
    }
    /// Switches from one thread context to another without keeping `RefCell`
    /// borrows alive on a suspended stack.
    ///
    /// # Safety
    /// The caller must prevent reentrant scheduler access while switching
    /// (normally by disabling interrupts), and both handles must remain alive
    /// until this call returns. The architecture contexts must not be accessed
    /// concurrently; later sequential switches may access them while this call
    /// is suspended.
    pub(crate) unsafe fn switch(from: &ThreadHandle<HW>, to: &ThreadHandle<HW>) {
        assert!(!Rc::ptr_eq(from, to), "cannot switch a thread to itself");

        let from_cell = unsafe { &*Rc::as_ptr(from) };
        let to_cell = unsafe { &*Rc::as_ptr(to) };
        let from_borrow = from_cell
            .try_borrow_mut()
            .expect("from thread is already borrowed");
        let to_borrow = to_cell
            .try_borrow_mut()
            .expect("to thread is already borrowed");

        let from_arch = unsafe { core::ptr::addr_of_mut!((*from_cell.as_ptr()).arch_thread) };
        let to_arch = unsafe { core::ptr::addr_of_mut!((*to_cell.as_ptr()).arch_thread) };

        // These borrows must be gone before the architecture switches stacks;
        // they would otherwise remain live on the suspended stack.
        drop(to_borrow);
        drop(from_borrow);

        unsafe { HW::thread_switch(from_arch, to_arch) };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(_data: usize) {}

    #[test]
    fn new_thread_starts_runnable_at_default_priority_without_a_mutex() {
        let thread = Thread::<klib::hal::TestHW>::new(1024, entry, 7);
        let thread = thread.borrow();

        assert_eq!(thread.priority, 0);
        assert!(!thread.shutdown);
        assert!(thread.waiting_mutex.is_none());
        assert_eq!(thread.arch_thread.init_stack_size, 1024);
        let expected_entry: fn(usize) = entry;
        assert!(core::ptr::fn_addr_eq(
            thread.arch_thread.init_main_func.unwrap(),
            expected_entry
        ));
        assert_eq!(thread.arch_thread.init_data, 7);
        assert_eq!(thread.arch_thread.switch_count, 0);
    }

    #[test]
    fn switching_test_threads_is_supported_by_test_arch() {
        let from = Thread::<klib::hal::TestHW>::new(0, entry, 0);
        let to = Thread::<klib::hal::TestHW>::new(0, entry, 0);

        unsafe { Thread::switch(&from, &to) };

        assert_eq!(from.borrow().arch_thread.switch_count, 1);
        assert_eq!(to.borrow().arch_thread.switch_count, 1);
    }
}
