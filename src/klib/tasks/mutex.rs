use alloc::rc::Rc;

use crate::{klib, source_location};
use core::{
    cell::{Cell, UnsafeCell},
    marker::PhantomData,
    ops::{Deref, DerefMut},
    sync::atomic::{AtomicBool, Ordering},
};

pub(super) struct RawMutex<HW> {
    locked: AtomicBool,
    lock_source: Cell<klib::diag::SourceLocation>,
    _hw: PhantomData<HW>,
}
pub(super) type RawMutexHandle<HW> = Rc<RawMutex<HW>>;
impl<HW: klib::hal::Hardware> RawMutex<HW> {
    pub fn new() -> RawMutexHandle<HW> {
        Rc::new(Self {
            locked: AtomicBool::new(false),
            lock_source: Cell::new(source_location!()),
            _hw: PhantomData,
        })
    }
    pub fn __try_lock(&self, loc: klib::diag::SourceLocation) -> Result<(), &'static str> {
        if self
            .locked
            .compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            return Err("mutex already locked");
        }
        self.lock_source.set(loc);
        Ok(())
    }
    pub fn __lock_begin(
        self: &RawMutexHandle<HW>,
        sched: &mut klib::tasks::sched::Scheduler<HW>,
        loc: klib::diag::SourceLocation,
    ) -> Result<(), klib::tasks::sched::SwitchRequest<HW>> {
        let irq_disable = klib::irq::DisableGuard::<HW>::lock();
        if self.__try_lock(loc).is_ok() {
            drop(irq_disable);
            Ok(())
        } else {
            Err(sched.wait_mutex(self.clone(), loc, irq_disable))
        }
    }
    pub fn unlock(&self) {
        self.lock_source.set(source_location!());
        self.locked.store(false, Ordering::Release);
    }
}

pub struct Mutex<HW, T> {
    raw_mutex_h: RawMutexHandle<HW>,
    data: UnsafeCell<T>,
}
impl<HW: klib::hal::Hardware, T> Mutex<HW, T> {
    pub fn new(data: T) -> Self {
        Self {
            raw_mutex_h: RawMutex::new(),
            data: UnsafeCell::new(data),
        }
    }
    pub fn __try_lock<'a>(
        &'a self,
        loc: klib::diag::SourceLocation,
    ) -> Result<MutexGuard<'a, HW, T>, &'static str> {
        self.raw_mutex_h.__try_lock(loc)?;
        Ok(MutexGuard(self))
    }
    pub fn __lock_begin(
        &self,
        sched: &mut klib::tasks::sched::Scheduler<HW>,
        loc: klib::diag::SourceLocation,
    ) -> Result<(), klib::tasks::sched::SwitchRequest<HW>> {
        self.raw_mutex_h.__lock_begin(sched, loc)
    }
    #[doc(hidden)]
    pub fn __finish_lock(&self) -> MutexGuard<'_, HW, T> {
        assert!(self.raw_mutex_h.locked.load(Ordering::Relaxed));
        MutexGuard(self)
    }
}
unsafe impl<HW: klib::hal::Hardware, T: Send> Sync for Mutex<HW, T> {}

pub struct MutexGuard<'a, HW: klib::hal::Hardware, T>(&'a Mutex<HW, T>);
impl<'a, HW: klib::hal::Hardware, T> Deref for MutexGuard<'a, HW, T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        unsafe { self.0.data.get().as_ref().unwrap() }
    }
}
impl<'a, HW: klib::hal::Hardware, T> DerefMut for MutexGuard<'a, HW, T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        unsafe { self.0.data.get().as_mut().unwrap() }
    }
}
impl<'a, HW: klib::hal::Hardware, T> Drop for MutexGuard<'a, HW, T> {
    fn drop(&mut self) {
        self.0.raw_mutex_h.unlock()
    }
}

#[macro_export]
macro_rules! mutex_lock {
    ($mutex:expr, $scheduler:expr $(,)?) => {{
        let __mutex = &$mutex;
        match __mutex.__lock_begin($scheduler, $crate::source_location!()) {
            Ok(()) => __mutex.__finish_lock(),
            Err(__switch) => {
                __switch.switch();
                __mutex.__finish_lock()
            }
        }
    }};
}

#[macro_export]
macro_rules! mutex_try_lock {
    ($mutex:expr $(,)?) => {
        ($mutex).__try_lock($crate::source_location!())
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(line: u32) -> klib::diag::SourceLocation {
        klib::diag::SourceLocation {
            file_name: "mutex_test.rs",
            module_path: "mutex::tests",
            line,
        }
    }

    #[test]
    fn try_lock_acquires_an_unlocked_mutex_and_records_the_source() {
        let mutex = RawMutex::<klib::hal::TestHW>::new();
        let loc = source(10);

        assert_eq!(mutex.__try_lock(loc), Ok(()));
        assert!(mutex.locked.load(Ordering::Relaxed));
        assert_eq!(mutex.lock_source.get().line, 10);
    }

    #[test]
    fn failed_try_lock_preserves_the_original_owner_location() {
        let mutex = RawMutex::<klib::hal::TestHW>::new();
        assert_eq!(mutex.__try_lock(source(11)), Ok(()));

        assert_eq!(mutex.__try_lock(source(22)), Err("mutex already locked"));
        assert_eq!(mutex.lock_source.get().line, 11);
    }

    #[test]
    fn unlock_releases_mutex_and_records_unlock_location() {
        let mutex = RawMutex::<klib::hal::TestHW>::new();
        assert_eq!(mutex.__try_lock(source(11)), Ok(()));

        mutex.unlock();

        assert!(!mutex.locked.load(Ordering::Relaxed));
        let unlock_location = mutex.lock_source.get();
        assert_eq!(
            unlock_location.module_path,
            module_path!().trim_end_matches("::tests")
        );
        assert_eq!(unlock_location.file_name, file!());
        assert_ne!(unlock_location.line, 0);
    }

    #[test]
    fn mutex_guard_mutates_data_and_unlocks_when_dropped() {
        let mutex = Mutex::<klib::hal::TestHW, _>::new(10);
        {
            let mut guard = mutex.__try_lock(source(30)).unwrap();
            *guard += 5;
            assert_eq!(*guard, 15);
            assert_eq!(
                mutex.__try_lock(source(31)).err(),
                Some("mutex already locked")
            );
        }

        let guard = mutex
            .__try_lock(source(32))
            .expect("drop should unlock the mutex");
        assert_eq!(*guard, 15);
    }

    #[test]
    fn blocking_mutex_lock_macro_records_its_call_site_and_unlocks_on_drop() {
        let mutex = Mutex::<klib::hal::TestHW, _>::new(1);
        let mut scheduler = klib::tasks::sched::Scheduler::<klib::hal::TestHW>::new();
        let expected_line = line!() + 1;
        let guard = crate::mutex_lock!(mutex, &mut scheduler);

        assert_eq!(*guard, 1);
        let lock_source = mutex.raw_mutex_h.lock_source.get();
        assert_eq!(lock_source.file_name, file!());
        assert_eq!(lock_source.module_path, module_path!());
        assert_eq!(lock_source.line, expected_line);

        drop(guard);
        assert!(!mutex.raw_mutex_h.locked.load(Ordering::Relaxed));
    }

    #[test]
    fn try_lock_macro_records_its_call_site() {
        let mutex = Mutex::<klib::hal::TestHW, _>::new(2);
        let expected_line = line!() + 1;
        let guard = crate::mutex_try_lock!(mutex).unwrap();

        assert_eq!(*guard, 2);
        let lock_source = mutex.raw_mutex_h.lock_source.get();
        assert_eq!(lock_source.file_name, file!());
        assert_eq!(lock_source.module_path, module_path!());
        assert_eq!(lock_source.line, expected_line);
    }

    #[test]
    fn failed_typed_try_lock_keeps_the_first_guard_active() {
        let mutex = Mutex::<klib::hal::TestHW, _>::new(0);
        let first_guard = mutex.__try_lock(source(40)).unwrap();

        assert_eq!(
            mutex.__try_lock(source(41)).err(),
            Some("mutex already locked")
        );
        assert!(mutex.raw_mutex_h.locked.load(Ordering::Relaxed));
        drop(first_guard);
        assert!(!mutex.raw_mutex_h.locked.load(Ordering::Relaxed));
    }
}
