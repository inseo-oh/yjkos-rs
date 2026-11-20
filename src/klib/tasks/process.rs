use crate::klib;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};

pub const PROCESS_STACK_SIZE: usize = 1024 * 16;

// This is only for testing, for now.
static RETURNED_ENTRY_COUNT: AtomicUsize = AtomicUsize::new(0);

pub(in crate::klib) fn process_entry_returned_count() -> usize {
    RETURNED_ENTRY_COUNT.load(Ordering::Relaxed)
}

pub(crate) fn process_entry_returned() -> ! {
    RETURNED_ENTRY_COUNT.fetch_add(1, Ordering::Relaxed);
    loop {
        core::hint::spin_loop();
    }
}

pub struct Process<HW: klib::hal::Hardware> {
    pub(super) priority: i8,
    queued_threads: Vec<klib::tasks::thread::ThreadHandle<HW>>,
    ran_threads: Vec<klib::tasks::thread::ThreadHandle<HW>>,
}

pub type ProcessHandle<HW> = Rc<RefCell<Process<HW>>>;

impl<HW: klib::hal::Hardware> Process<HW> {
    pub fn new() -> ProcessHandle<HW> {
        Rc::new(RefCell::new(Self {
            priority: 0,
            queued_threads: Vec::new(),
            ran_threads: Vec::new(),
        }))
    }
    pub fn priority(&self) -> i8 {
        self.priority
    }
    pub fn threads(&self) -> impl Iterator<Item = klib::tasks::thread::ThreadHandle<HW>> {
        self.queued_threads
            .iter()
            .chain(self.ran_threads.iter())
            .cloned()
    }
    pub fn add_thread(&mut self, thr: &klib::tasks::thread::ThreadHandle<HW>) {
        self.queued_threads.push(thr.clone());
    }
    pub fn queued_threads_left(&self) -> bool {
        !self.queued_threads.is_empty()
    }
    pub fn pick_next_thread(&mut self) -> klib::tasks::thread::ThreadHandle<HW> {
        if self.queued_threads.is_empty() {
            for thr in self.ran_threads.drain(0..self.ran_threads.len()) {
                self.queued_threads.push(thr);
            }
        }
        let thr = self
            .queued_threads
            .pop()
            .expect("there should be at least one thread");
        self.ran_threads.push(thr.clone());
        thr
    }
    pub fn running_thread(&self) -> klib::tasks::thread::ThreadHandle<HW> {
        self.ran_threads
            .last()
            .expect("there's no running thread")
            .clone()
    }
}
