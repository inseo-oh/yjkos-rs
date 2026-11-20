use crate::klib::{self, KGlobals};
use alloc::rc::{Rc, Weak};
use core::cell::RefCell;
use core::ptr;
use core::sync::atomic::{AtomicUsize, Ordering};

pub const THREAD_STACK_SIZE: usize = 1024 * 16;

// This is only for testing, for now.
static RETURNED_ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(in crate::klib) fn thread_entry_returned_count() -> usize {
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
    process: Weak<RefCell<klib::tasks::process::Process<HW>>>,
    pub(super) waiting_mutex: Option<(
        klib::tasks::mutex::RawMutexHandle<HW>,
        klib::diag::SourceLocation,
    )>,
    pub(super) shutdown: bool,
    pub stack_object: Option<klib::mem::vmm::ObjectHandle<HW>>,
    pub kglobals: &'static KGlobals<HW>,
}

pub type ThreadHandle<HW> = Rc<RefCell<Thread<HW>>>;

impl<HW: klib::hal::Hardware> Thread<HW> {
    // Those init_ parameters are only valid for initial setup.
    // This of course applies to any new thread, but the boot thread is exception: It's thread for already running code.
    pub fn new(
        kglobals: &'static KGlobals<HW>,
        process: &klib::tasks::process::ProcessHandle<HW>,
        init_stack_size: usize,
        start: klib::hal::ThreadStart,
    ) -> Result<ThreadHandle<HW>, &'static str> {
        let (address_space, is_user_process) = {
            let process = process.borrow();
            (process.address_space.clone(), process.is_user())
        };
        if is_user_process != matches!(start, klib::hal::ThreadStart::User { .. }) {
            return Err("thread start mode does not match process privilege level");
        }
        if is_user_process && init_stack_size == 0 {
            return Err("user thread requires a stack");
        }
        let stack_object = if init_stack_size == 0 {
            None
        } else {
            let object = klib::mem::vmm::alloc(
                &address_space,
                init_stack_size,
                klib::mem::vmm::MapFlags {
                    write: true,
                    read: true,
                    ..Default::default()
                },
            )?;

            if is_user_process {
                let page_count = klib::miscmath::size_to_blocks(init_stack_size, HW::PAGE_SIZE);
                let allocation = match kglobals.pmm.lock().malloc(page_count) {
                    Ok(allocation) => allocation,
                    Err(error) => {
                        klib::mem::vmm::free(&object, &mut kglobals.pmm.lock());
                        return Err(error);
                    }
                };
                let mapping_result = address_space.borrow_mut().with_active(|| {
                    HW::mmu_map(
                        object.borrow().start,
                        allocation.base,
                        page_count,
                        klib::mem::vmm::MapFlags {
                            write: true,
                            read: true,
                            ..Default::default()
                        },
                        true,
                    )
                });
                if let Err(error) = mapping_result {
                    {
                        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
                        let _ = HW::mmu_unmap(object.borrow().start, page_count);
                    }
                    klib::mem::vmm::free(&object, &mut kglobals.pmm.lock());
                    // SAFETY: The allocation has not been installed in the
                    // address space when mmu_map reports failure.
                    unsafe { kglobals.pmm.lock().free(allocation) };
                    return Err(error);
                }
            }
            Some(object)
        };

        let init_stack_base = stack_object
            .as_ref()
            .map(|o| o.borrow().start.cast::<u32>())
            .unwrap_or(ptr::null_mut());
        let arch_thread = if is_user_process {
            address_space
                .borrow_mut()
                .with_active(|| HW::thread_create(init_stack_base, init_stack_size, start))
        } else {
            HW::thread_create(init_stack_base, init_stack_size, start)
        };
        let thr = Rc::new(RefCell::new(Self {
            kglobals,
            stack_object,
            process: Rc::downgrade(process),
            arch_thread,
            waiting_mutex: None,
            shutdown: false,
        }));
        process.borrow_mut().add_thread(&thr);
        Ok(thr)
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
    pub(super) unsafe fn switch(from: &ThreadHandle<HW>, to: &ThreadHandle<HW>) {
        assert!(!Rc::ptr_eq(from, to), "cannot switch a thread to itself");

        let from_cell = unsafe { &*Rc::as_ptr(from) };
        let to_cell = unsafe { &*Rc::as_ptr(to) };

        // Keep these strong process references alive across the switch. Their
        // destructors must run only after this thread resumes in its own
        // address space.
        let from_proc_h = from.borrow().process();
        let to_proc_h = to.borrow().process();
        let reload_address_space = {
            let from_proc = from_proc_h.borrow();
            let to_proc = to_proc_h.borrow();
            let address_spaces_differ =
                !Rc::ptr_eq(&from_proc.address_space, &to_proc.address_space);
            if address_spaces_differ {
                from_proc.address_space.borrow_mut().snapshot_current();
                to_proc.address_space.borrow().load();
            }
            address_spaces_differ
        };
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

        // The architecture saves the outgoing ESP before reloading CR3, then
        // switches stacks without touching these heap-resident thread objects.
        unsafe { HW::thread_switch(from_arch, to_arch, reload_address_space) };
    }
    pub fn process(&self) -> klib::tasks::process::ProcessHandle<HW> {
        self.process.upgrade().expect("process no longer exists")
    }
}

impl<HW: klib::hal::Hardware> Drop for Thread<HW> {
    fn drop(&mut self) {
        if let Some(stack_object) = &self.stack_object {
            klib::mem::vmm::free(stack_object, &mut self.kglobals.pmm.lock());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    static TEST_KGLOBALS: KGlobals<klib::hal::TestHW> = KGlobals::new();
    static TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    fn entry(_data: usize) {}

    fn test_kglobals() -> &'static KGlobals<klib::hal::TestHW> {
        &TEST_KGLOBALS
    }

    fn test_process() -> klib::tasks::process::ProcessHandle<klib::hal::TestHW> {
        let address_space = klib::mem::vmm::AddressSpace::new(0x1000 as *mut u8, 16 * 4096, false);
        klib::tasks::process::Process::new(address_space)
    }

    #[test]
    fn new_thread_starts_runnable_at_default_priority_without_a_mutex() {
        let _test_lock = TEST_LOCK.lock().unwrap();
        let kglobals = test_kglobals();
        let process = test_process();
        let thread = Thread::<klib::hal::TestHW>::new(
            kglobals,
            &process,
            1024,
            klib::hal::ThreadStart::Kernel { entry, data: 7 },
        )
        .unwrap();
        let thread = thread.borrow();

        assert!(!thread.shutdown);
        assert!(thread.waiting_mutex.is_none());
        assert_eq!(thread.arch_thread.init_stack_size, 1024);
        assert_eq!(
            thread.stack_object.as_ref().unwrap().borrow().size,
            <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE
        );
        assert!(core::ptr::eq(thread.kglobals, kglobals));
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
        let _test_lock = TEST_LOCK.lock().unwrap();
        let kglobals = test_kglobals();
        let process = test_process();
        let from = Thread::<klib::hal::TestHW>::new(
            kglobals,
            &process,
            1024,
            klib::hal::ThreadStart::Kernel { entry, data: 0 },
        )
        .unwrap();
        let to = Thread::<klib::hal::TestHW>::new(
            kglobals,
            &process,
            1024,
            klib::hal::ThreadStart::Kernel { entry, data: 0 },
        )
        .unwrap();

        unsafe { Thread::switch(&from, &to) };

        assert_eq!(from.borrow().arch_thread.switch_count, 1);
        assert_eq!(to.borrow().arch_thread.switch_count, 1);
    }
}
