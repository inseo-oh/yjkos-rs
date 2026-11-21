// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::klib;
use core::{
    cell::UnsafeCell,
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub struct DisableGuard<HW: klib::hal::Hardware> {
    irq_was_enabled: bool,
    hw: PhantomData<HW>,
}
impl<HW: klib::hal::Hardware> DisableGuard<HW> {
    pub fn lock() -> Self {
        let irq_was_enabled = HW::irq_enabled();
        HW::irq_disable();
        Self {
            irq_was_enabled,
            hw: PhantomData,
        }
    }
}
impl<HW: klib::hal::Hardware> Drop for DisableGuard<HW> {
    fn drop(&mut self) {
        if self.irq_was_enabled {
            HW::irq_enable();
        }
    }
}

pub struct Mutex<HW, T> {
    locked: AtomicBool,
    data: UnsafeCell<T>,
    _hw: PhantomData<HW>,
}
impl<HW: klib::hal::Hardware, T> Mutex<HW, T> {
    pub const fn new(data: T) -> Self {
        Self {
            locked: AtomicBool::new(false),
            data: UnsafeCell::new(data),
            _hw: PhantomData,
        }
    }
    pub fn lock<'a>(&'a self) -> MutexGuard<'a, HW, T> {
        let lock = DisableGuard::lock();
        self.locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .expect("mutex is already locked");
        MutexGuard { mutex: self, lock }
    }
    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}
unsafe impl<HW: klib::hal::Hardware, T: Send> Sync for Mutex<HW, T> {}

pub struct MutexGuard<'a, HW: klib::hal::Hardware, T> {
    mutex: &'a Mutex<HW, T>,
    lock: DisableGuard<HW>,
}
impl<'a, HW: klib::hal::Hardware, T> Deref for MutexGuard<'a, HW, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.mutex.data.get().as_ref().unwrap() }
    }
}
impl<'a, HW: klib::hal::Hardware, T> DerefMut for MutexGuard<'a, HW, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.mutex.data.get().as_mut().unwrap() }
    }
}
impl<'a, HW: klib::hal::Hardware, T> Drop for MutexGuard<'a, HW, T> {
    fn drop(&mut self) {
        self.mutex.unlock();
    }
}

#[cfg(test)]
mod tests {
    use klib::hal::{Hardware, TestHW};

    use super::*;

    #[test]
    fn irq_disable_guard_restores_the_previous_state_and_supports_nesting() {
        TestHW::irq_enable();
        {
            let _outer = DisableGuard::<TestHW>::lock();
            assert!(!TestHW::irq_enabled());
            {
                let _inner = DisableGuard::<TestHW>::lock();
                assert!(!TestHW::irq_enabled());
            }
            assert!(!TestHW::irq_enabled());
        }
        assert!(TestHW::irq_enabled());

        TestHW::irq_disable();
        {
            let _guard = DisableGuard::<TestHW>::lock();
            assert!(!TestHW::irq_enabled());
        }
        assert!(!TestHW::irq_enabled());
        TestHW::irq_enable();
    }

    #[test]
    fn mutex_lock_disables_irqs_and_restores_the_previous_state() {
        TestHW::irq_enable();
        let mutex = Mutex::<TestHW, _>::new(0);

        {
            let mut guard = mutex.lock();
            assert!(!TestHW::irq_enabled());
            *guard = 42;
        }

        assert!(TestHW::irq_enabled());
        assert_eq!(*mutex.lock(), 42);

        TestHW::irq_disable();
        {
            let _guard = mutex.lock();
            assert!(!TestHW::irq_enabled());
        }
        assert!(!TestHW::irq_enabled());
        TestHW::irq_enable();
    }

    #[test]
    fn nested_mutex_locks_restore_irq_state_in_order() {
        TestHW::irq_enable();
        let outer_mutex = Mutex::<TestHW, _>::new(());
        let inner_mutex = Mutex::<TestHW, _>::new(());

        {
            let _outer = outer_mutex.lock();
            assert!(!TestHW::irq_enabled());
            {
                let _inner = inner_mutex.lock();
                assert!(!TestHW::irq_enabled());
            }
            assert!(!TestHW::irq_enabled());
        }

        assert!(TestHW::irq_enabled());
    }
}
