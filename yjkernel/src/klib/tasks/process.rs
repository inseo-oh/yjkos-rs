// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::klib;
use alloc::rc::Rc;
use alloc::vec::Vec;
use core::cell::RefCell;
use core::sync::atomic::{AtomicUsize, Ordering};

pub(in crate::klib) const PROCESS_STACK_SIZE: usize = 1024 * 16;

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

pub(in crate::klib) struct Process<HW: klib::hal::Hardware> {
    address_space: klib::mem::vmm::AddressSpaceHandle<HW>,
    pub(super) priority: i8,
    queued_threads: Vec<klib::tasks::thread::ThreadHandle<HW>>,
    ran_threads: Vec<klib::tasks::thread::ThreadHandle<HW>>,
}

pub(in crate::klib) type ProcessHandle<HW> = Rc<RefCell<Process<HW>>>;

impl<HW: klib::hal::Hardware> Process<HW> {
    pub(in crate::klib) fn new(
        address_space: klib::mem::vmm::AddressSpaceHandle<HW>,
    ) -> ProcessHandle<HW> {
        Rc::new(RefCell::new(Self {
            address_space,
            priority: 0,
            queued_threads: Vec::new(),
            ran_threads: Vec::new(),
        }))
    }
    pub(in crate::klib) fn address_space(&self) -> klib::mem::vmm::AddressSpaceHandle<HW> {
        self.address_space.clone()
    }
    pub(in crate::klib) fn is_user(&self) -> bool {
        self.address_space.borrow().is_user()
    }
    pub(in crate::klib) fn priority(&self) -> i8 {
        self.priority
    }
    pub(in crate::klib) fn threads(
        &self,
    ) -> impl Iterator<Item = klib::tasks::thread::ThreadHandle<HW>> {
        self.queued_threads
            .iter()
            .chain(self.ran_threads.iter())
            .cloned()
    }
    pub(in crate::klib) fn add_thread(&mut self, thr: &klib::tasks::thread::ThreadHandle<HW>) {
        self.queued_threads.push(thr.clone());
    }
    pub(in crate::klib) fn queued_threads_left(&self) -> bool {
        !self.queued_threads.is_empty()
    }
    pub(in crate::klib) fn pick_next_thread(&mut self) -> klib::tasks::thread::ThreadHandle<HW> {
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
    pub(in crate::klib) fn running_thread(&self) -> klib::tasks::thread::ThreadHandle<HW> {
        self.ran_threads
            .last()
            .expect("there's no running thread")
            .clone()
    }
}
