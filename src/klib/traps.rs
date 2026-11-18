use crate::klib;
use alloc::vec::Vec;
use core::marker::PhantomData;

#[derive(Clone, Copy)]
pub struct TrapHandler<HW: klib::hal::Hardware> {
    callback: fn(trap_num: usize, trap_frame: *mut HW::TrapFrame),
}

pub const TRAP_COUNT: usize = 256;

pub struct TrapManager<HW: klib::hal::Hardware> {
    traps: [Vec<TrapHandler<HW>>; TRAP_COUNT],
    _hw: PhantomData<HW>,
}

impl<HW: klib::hal::Hardware> TrapManager<HW> {
    pub const fn new() -> Self {
        Self {
            traps: [const { Vec::new() }; TRAP_COUNT],
            _hw: PhantomData,
        }
    }

    pub fn register_trap(
        &mut self,
        trap_num: usize,
        callback: fn(trap_num: usize, trap_frame: *mut HW::TrapFrame),
    ) -> Result<(), &'static str> {
        if trap_num >= TRAP_COUNT {
            return Err("invalid trap number");
        }
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        let handlers = &mut self.traps[trap_num];
        handlers.push(TrapHandler { callback });
        Ok(())
    }
    pub fn trap(&self, trap_num: usize, trapframe: *mut HW::TrapFrame) {
        assert!(!HW::irq_enabled());
        if trap_num >= TRAP_COUNT {
            return;
        }
        for trap in &self.traps[trap_num] {
            (trap.callback)(trap_num, trapframe)
        }
    }
}

impl<HW: klib::hal::Hardware> Default for TrapManager<HW> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    static CALLBACKS: Mutex<Vec<(u8, usize, usize)>> = Mutex::new(Vec::new());

    fn first_callback(
        trap_num: usize,
        trap_frame: *mut <klib::hal::TestHW as klib::hal::Hardware>::TrapFrame,
    ) {
        CALLBACKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((1, trap_num, trap_frame.addr()));
    }

    fn second_callback(
        trap_num: usize,
        trap_frame: *mut <klib::hal::TestHW as klib::hal::Hardware>::TrapFrame,
    ) {
        CALLBACKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push((2, trap_num, trap_frame.addr()));
    }

    #[test]
    fn new_manager_has_no_handlers() {
        let manager = TrapManager::<klib::hal::TestHW>::new();

        assert!(manager.traps.iter().all(Vec::is_empty));
    }

    #[test]
    fn registration_rejects_out_of_range_trap_numbers() {
        let mut manager = TrapManager::<klib::hal::TestHW>::new();

        assert_eq!(
            manager.register_trap(TRAP_COUNT, first_callback),
            Err("invalid trap number")
        );
        assert!(manager.traps.iter().all(Vec::is_empty));
        assert!(<klib::hal::TestHW as klib::hal::Hardware>::irq_enabled());
    }

    #[test]
    fn registration_restores_the_previous_interrupt_state() {
        let mut manager = TrapManager::<klib::hal::TestHW>::new();
        <klib::hal::TestHW as klib::hal::Hardware>::irq_enable();

        assert_eq!(manager.register_trap(7, first_callback), Ok(()));
        assert!(<klib::hal::TestHW as klib::hal::Hardware>::irq_enabled());

        <klib::hal::TestHW as klib::hal::Hardware>::irq_disable();
        assert_eq!(manager.register_trap(7, second_callback), Ok(()));
        assert!(!<klib::hal::TestHW as klib::hal::Hardware>::irq_enabled());
        <klib::hal::TestHW as klib::hal::Hardware>::irq_enable();
    }

    #[test]
    fn dispatch_calls_handlers_in_registration_order_for_the_selected_trap() {
        let mut manager = TrapManager::<klib::hal::TestHW>::new();
        manager.register_trap(3, first_callback).unwrap();
        manager.register_trap(3, second_callback).unwrap();
        manager.register_trap(4, second_callback).unwrap();
        CALLBACKS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clear();

        let _irq_guard = klib::irq::DisableGuard::<klib::hal::TestHW>::lock();
        manager.trap(3, 0x1234 as *mut ());

        assert_eq!(
            *CALLBACKS
                .lock()
                .unwrap_or_else(std::sync::PoisonError::into_inner),
            [(1, 3, 0x1234), (2, 3, 0x1234)]
        );
    }

    #[test]
    fn dispatch_ignores_out_of_range_trap_numbers() {
        let manager = TrapManager::<klib::hal::TestHW>::new();
        let _irq_guard = klib::irq::DisableGuard::<klib::hal::TestHW>::lock();

        manager.trap(TRAP_COUNT, core::ptr::null_mut());
    }
}
