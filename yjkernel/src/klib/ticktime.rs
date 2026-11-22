// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use core::sync::atomic::{AtomicU32, Ordering};

pub(in crate::klib) type TickTime = AtomicU32;

static TICKTIME: TickTime = AtomicU32::new(0);

#[cfg(test)]
static TEST_TICKTIME_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub(in crate::klib) fn load() -> u32 {
    TICKTIME.load(Ordering::Relaxed)
}
pub(in crate::klib) fn store(t: u32) {
    TICKTIME.store(t, Ordering::Relaxed);
}
pub(crate) fn add(t: u32) {
    TICKTIME.fetch_add(t, Ordering::Relaxed);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn store_ticktime_updates_the_value_returned_by_load() {
        let _lock = TEST_TICKTIME_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = load();
        store(1234);
        assert_eq!(load(), 1234);
        store(previous);
    }

    #[test]
    fn add_ticktime_increments_the_value_returned_by_load() {
        let _lock = TEST_TICKTIME_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = load();

        store(17);
        add(25);

        assert_eq!(load(), 42);
        store(previous);
    }

    #[test]
    fn add_ticktime_wraps_on_overflow() {
        let _lock = TEST_TICKTIME_LOCK
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let previous = load();

        store(u32::MAX);
        add(1);

        assert_eq!(load(), 0);
        store(previous);
    }
}
