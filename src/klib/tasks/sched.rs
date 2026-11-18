use core::cell::RefCell;

use crate::klib;
use alloc::{rc::Rc, vec::Vec};

pub struct Queue<HW: klib::hal::Hardware> {
    // Lower value -> Higher priority (similar to UNIX niceness).
    priority: i8,
    opportunities: usize,
    threads: Vec<klib::tasks::thread::ThreadHandle<HW>>,
}
pub type QueueHandle<HW> = Rc<RefCell<Queue<HW>>>;

// NOTE: Most scheduler functions are considered as critical section, so it is safe to turn off interrupts before using any of these!
//       TODO: Just do that in sched itself :D

const BOOT_THREAD_PRIORITY: i8 = 20;

pub struct Scheduler<HW: klib::hal::Hardware> {
    // Each queue has associated priority, and when selecting the next thread we
    // start from highest priority, but every time a queue is selected, remaining
    // opportunities is decreased.
    // (Initial opportunities count is 1 for lowest priority, 2 for next lowest,
    // and so on...)
    //
    // When opportunities become zero, that queue is no longer selected, thus lower
    // priority queues are selected less than higher priority ones.
    queues: Vec<QueueHandle<HW>>,
    current_queue: Option<QueueHandle<HW>>,
    running_thread: Option<klib::tasks::thread::ThreadHandle<HW>>,
    mutex_wait_threads: Vec<klib::tasks::thread::ThreadHandle<HW>>,
}

impl<HW: klib::hal::Hardware> Scheduler<HW> {
    pub const fn new() -> Self {
        Self {
            queues: Vec::new(),
            current_queue: None,
            running_thread: None,
            mutex_wait_threads: Vec::new(),
        }
    }
    fn alloc_queue(
        &mut self,
        insert_after: Option<QueueHandle<HW>>,
        priority: i8,
    ) -> QueueHandle<HW> {
        let insert_idx = insert_after
            .map(|q1| {
                self.queues
                    .iter()
                    .position(|q2| Rc::ptr_eq(&q1, q2))
                    .expect("no such queue found")
                    + 1
            })
            .unwrap_or(0);
        let result_queue = Rc::new(RefCell::new(Queue::<HW> {
            priority,
            opportunities: 0,
            threads: Vec::new(),
        }));
        self.queues.insert(insert_idx, result_queue.clone());
        result_queue
    }
    fn get_queue(&mut self, priority: i8) -> QueueHandle<HW> {
        let mut insert_after = None;
        let mut result_queue = None;
        let mut should_insert_front = false;
        let queue = self.queues.first();
        if let Some(queue) = queue
            && priority < queue.borrow().priority
        {
            should_insert_front = true;
        }
        if !should_insert_front {
            for (idx, queue_h) in self.queues.iter().enumerate() {
                let next_queue_h = if idx + 1 != self.queues.len() {
                    Some(self.queues[idx + 1].clone())
                } else {
                    None
                };
                // Found the queue
                if queue_h.borrow().priority == priority {
                    result_queue = Some(queue_h);
                    break;
                }
                // No more queue
                if next_queue_h.is_none() {
                    insert_after = Some(queue_h);
                    break;
                }
                // Given priority is between current and next queue's priority.
                if (queue_h.borrow().priority < priority)
                    && (priority < next_queue_h.unwrap().borrow().priority)
                {
                    insert_after = Some(queue_h);
                    break;
                }
            }
        }
        if let Some(q) = result_queue {
            return q.clone();
        }
        self.alloc_queue(insert_after.cloned(), priority)
    }
    fn reset_queues(&mut self) {
        // We have to reset the scheduler either because we are scheduling for the first time, or every queue ran out of opportunities.

        for (opportunities, queue_h) in (1..=self.queues.len()).rev().zip(self.queues.iter()) {
            queue_h.borrow_mut().opportunities = opportunities;
        }
    }
    fn find_next_queue_index(
        &self,
        start_index: usize,
        current_queue_index: Option<usize>,
    ) -> Option<usize> {
        let queue_count = self.queues.len();
        (0..queue_count).find_map(|offset| {
            let index = (start_index + offset) % queue_count;
            if current_queue_index == Some(index) {
                return None;
            }
            let queue = self.queues[index].borrow();
            (queue.opportunities != 0 && !queue.threads.is_empty()).then_some(index)
        })
    }
    fn pick_next_queue(&mut self) -> Option<QueueHandle<HW>> {
        let current_queue_index = self
            .current_queue
            .as_ref()
            .map(|cq| self.queues.iter().position(|q| Rc::ptr_eq(q, cq)).unwrap());
        let queue_count = self.queues.len();
        if queue_count == 0 {
            return None;
        }
        let start_index = current_queue_index
            .map(|index| (index + 1) % queue_count)
            .unwrap_or(0);
        let result_queue_index =
            if let Some(index) = self.find_next_queue_index(start_index, current_queue_index) {
                index
            } else {
                self.reset_queues();
                self.find_next_queue_index(start_index, current_queue_index)?
            };
        let queue = self.queues[result_queue_index].clone();
        queue.borrow_mut().opportunities -= 1;
        self.current_queue = Some(queue.clone());
        Some(queue)
    }
    fn pick_next_task_from_mutex_wait_list(
        &mut self,
    ) -> Option<klib::tasks::thread::ThreadHandle<HW>> {
        let result_idx = self.mutex_wait_threads.iter().position(|thread_h| {
            let thread = thread_h.borrow_mut();
            let (waiting_mutex, source_loc) = thread.waiting_mutex.as_ref().unwrap();
            if waiting_mutex.__try_lock(*source_loc).is_ok() {
                return true;
            }
            false
        });
        if let Some(idx) = result_idx {
            let result = self.mutex_wait_threads[idx].clone();
            self.mutex_wait_threads.remove(idx);
            if result.borrow().shutdown {
                // thread is about to shutdown - unlock the mutex
                result.borrow().waiting_mutex.as_ref().unwrap().0.unlock();
            }
            result.borrow_mut().waiting_mutex = None;
            return Some(result);
        }
        None
    }
    fn pick_next_task(&mut self) -> Option<klib::tasks::thread::ThreadHandle<HW>> {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        let mut result;
        loop {
            result = self.pick_next_task_from_mutex_wait_list().or_else(|| {
                self.pick_next_queue()
                    .and_then(|queue| queue.borrow_mut().threads.pop())
            });
            if result.as_ref().is_none_or(|t| !t.borrow().shutdown) {
                break;
            }
            // The thread will be shutdown at this point.
            drop(result);
        }
        result
    }
    pub(super) fn wait_mutex(
        &mut self,
        mutex: klib::tasks::mutex::RawMutexHandle<HW>,
        lock_source: klib::diag::SourceLocation,
    ) {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        self.running_thread
            .as_mut()
            .unwrap()
            .borrow_mut()
            .waiting_mutex = Some((mutex.clone(), lock_source));
        let next_thread = self
            .pick_next_task()
            .expect("there is no other thread to wait for mutex");
        self.mutex_wait_threads
            .insert(0, self.running_thread.as_mut().unwrap().clone());
        let old_thread = self.running_thread.as_mut().unwrap().clone();
        assert!(!Rc::ptr_eq(&old_thread, &next_thread));
        self.running_thread = Some(next_thread.clone());
        unsafe { klib::tasks::thread::Thread::switch(&old_thread, &next_thread) };
        // ... And we should return here when mutex is released.
    }
    pub fn queue(&mut self, thread: klib::tasks::thread::ThreadHandle<HW>) {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        let queue = self.get_queue(thread.borrow().priority);
        queue.borrow_mut().threads.insert(0, thread);
    }
    pub fn schedule(&mut self) {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        if let Some(next_thread) = self.pick_next_task() {
            // Note that it is safe to call schedule() before calling init_boot_thread(), as long as it's called before creating
            // any other threads.
            // If there are no other threads to switch, we will never reach here,
            // so it doesn't trip below assertion.
            let running_thread = self.running_thread.clone().unwrap();
            self.queue(running_thread.clone());
            let old_thread = &running_thread;
            assert!(!Rc::ptr_eq(old_thread, &next_thread));
            self.running_thread = Some(next_thread.clone());
            unsafe { klib::tasks::thread::Thread::switch(old_thread, &next_thread) };
        }
    }
    pub fn init_boot_thread(&mut self) {
        assert!(self.running_thread.is_none());
        // Init values for thread is not used, as those are only used for fresh new
        // threads, and boot thread is what we are running now.
        self.running_thread = Some(klib::tasks::thread::Thread::new(0, dummy_entry_fn, 0));
        self.running_thread.as_mut().unwrap().borrow_mut().priority = BOOT_THREAD_PRIORITY;
    }
}

fn dummy_entry_fn(_unused: usize) {
    unreachable!();
}

#[cfg(test)]
mod tests {
    use super::*;

    fn entry(_unused: usize) {}

    fn make_thread(priority: i8) -> klib::tasks::thread::ThreadHandle<klib::hal::TestHW> {
        let thread = klib::tasks::thread::Thread::new(0, entry, 0);
        thread.borrow_mut().priority = priority;
        thread
    }

    fn selected_priority(queue: Option<QueueHandle<klib::hal::TestHW>>) -> Option<i8> {
        queue.map(|queue| queue.borrow().priority)
    }

    #[test]
    fn new_scheduler_has_no_queues_or_running_thread() {
        let scheduler = Scheduler::<klib::hal::TestHW>::new();

        assert!(scheduler.queues.is_empty());
        assert!(scheduler.current_queue.is_none());
        assert!(scheduler.running_thread.is_none());
        assert!(scheduler.mutex_wait_threads.is_empty());
    }

    #[test]
    fn boot_thread_initialization_sets_boot_priority() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        scheduler.init_boot_thread();

        let running = scheduler.running_thread.as_ref().unwrap().borrow();
        assert_eq!(running.priority, BOOT_THREAD_PRIORITY);
        assert!(!running.shutdown);
        assert!(running.waiting_mutex.is_none());
    }

    #[test]
    fn get_queue_keeps_priorities_sorted_and_reuses_matching_queues() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        scheduler.get_queue(20);
        let high = scheduler.get_queue(0);
        let middle = scheduler.get_queue(10);
        let middle_again = scheduler.get_queue(10);

        let priorities: Vec<_> = scheduler
            .queues
            .iter()
            .map(|queue| queue.borrow().priority)
            .collect();
        assert_eq!(priorities, [0, 10, 20]);
        assert!(Rc::ptr_eq(&middle, &middle_again));
        assert!(Rc::ptr_eq(&high, &scheduler.queues[0]));
    }

    #[test]
    fn priority_queues_receive_more_opportunities_for_higher_priority() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        scheduler.get_queue(0);
        scheduler.get_queue(10);
        scheduler.get_queue(20);

        scheduler.reset_queues();

        let opportunities: Vec<_> = scheduler
            .queues
            .iter()
            .map(|queue| queue.borrow().opportunities)
            .collect();
        assert_eq!(opportunities, [3, 2, 1]);
    }

    #[test]
    fn queue_selection_respects_opportunity_weights_and_skips_current_queue() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        for priority in [0, 10, 20] {
            scheduler.queue(make_thread(priority));
        }

        let selected: Vec<_> = (0..6)
            .map(|_| selected_priority(scheduler.pick_next_queue()))
            .collect();
        assert_eq!(
            selected,
            [Some(0), Some(10), Some(20), Some(0), Some(10), Some(0)]
        );
    }

    #[test]
    fn queue_selection_returns_none_when_there_are_no_queues() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();

        assert!(scheduler.pick_next_queue().is_none());
    }

    #[test]
    fn queue_selection_returns_none_when_only_the_current_queue_is_runnable() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        scheduler.queue(make_thread(0));
        assert_eq!(selected_priority(scheduler.pick_next_queue()), Some(0));

        assert!(scheduler.pick_next_queue().is_none());
        assert_eq!(scheduler.queues[0].borrow().opportunities, 1);
    }

    #[test]
    fn queue_selection_ignores_empty_queues_and_resets_opportunities() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        scheduler.get_queue(0);
        scheduler.queue(make_thread(10));
        scheduler.get_queue(20);

        assert_eq!(selected_priority(scheduler.pick_next_queue()), Some(10));
    }

    #[test]
    fn picking_a_task_discards_shutdown_tasks_and_continues_to_another_queue() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        let shutdown_thread = make_thread(0);
        shutdown_thread.borrow_mut().shutdown = true;
        scheduler.queue(shutdown_thread);
        let live_thread = make_thread(10);
        scheduler.queue(live_thread.clone());

        let selected = scheduler.pick_next_task().unwrap();

        assert!(Rc::ptr_eq(&selected, &live_thread));
        assert!(scheduler.queues[0].borrow().threads.is_empty());
    }

    #[test]
    fn schedule_without_runnable_tasks_is_a_no_op() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();

        scheduler.schedule();

        assert!(scheduler.running_thread.is_none());
        assert!(scheduler.current_queue.is_none());
    }

    #[test]
    fn schedule_switches_threads_and_requeues_the_previous_runner() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        scheduler.init_boot_thread();
        let boot_thread = scheduler.running_thread.as_ref().unwrap().clone();
        let worker = make_thread(0);
        scheduler.queue(worker.clone());

        scheduler.schedule();

        assert!(Rc::ptr_eq(
            scheduler.running_thread.as_ref().unwrap(),
            &worker
        ));
        assert!(Rc::ptr_eq(
            &scheduler.current_queue.as_ref().unwrap(),
            &scheduler.queues[0]
        ));
        assert!(scheduler.queues[0].borrow().threads.is_empty());
        assert!(Rc::ptr_eq(
            &scheduler.queues[1].borrow().threads[0],
            &boot_thread
        ));

        scheduler.schedule();

        assert!(Rc::ptr_eq(
            scheduler.running_thread.as_ref().unwrap(),
            &boot_thread
        ));
        assert!(Rc::ptr_eq(
            &scheduler.queues[0].borrow().threads[0],
            &worker
        ));
    }

    #[test]
    fn waiting_thread_is_released_and_requeued_when_mutex_becomes_available() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        scheduler.init_boot_thread();
        let boot_thread = scheduler.running_thread.as_ref().unwrap().clone();
        let next_thread = make_thread(0);
        scheduler.queue(next_thread.clone());
        let mutex = klib::tasks::mutex::RawMutex::<klib::hal::TestHW>::new();
        let lock_source = klib::diag::SourceLocation {
            file_name: "sched_test.rs",
            module_path: "sched::tests",
            line: 1,
        };
        assert_eq!(mutex.__try_lock(lock_source), Ok(()));

        scheduler.wait_mutex(mutex.clone(), lock_source);

        assert!(Rc::ptr_eq(
            scheduler.running_thread.as_ref().unwrap(),
            &next_thread
        ));
        assert_eq!(scheduler.mutex_wait_threads.len(), 1);
        assert!(boot_thread.borrow().waiting_mutex.is_some());

        mutex.unlock();
        scheduler.schedule();

        assert!(Rc::ptr_eq(
            scheduler.running_thread.as_ref().unwrap(),
            &boot_thread
        ));
        assert!(boot_thread.borrow().waiting_mutex.is_none());
        assert_eq!(mutex.__try_lock(lock_source), Err("mutex already locked"));
        assert!(scheduler.mutex_wait_threads.is_empty());
    }

    #[test]
    fn shutdown_mutex_waiter_unlocks_mutex_and_is_discarded() {
        let mut scheduler = Scheduler::<klib::hal::TestHW>::new();
        scheduler.init_boot_thread();
        let boot_thread = scheduler.running_thread.as_ref().unwrap().clone();
        let next_thread = make_thread(0);
        scheduler.queue(next_thread.clone());
        let mutex = klib::tasks::mutex::RawMutex::<klib::hal::TestHW>::new();
        let lock_source = klib::diag::SourceLocation {
            file_name: "sched_test.rs",
            module_path: "sched::tests",
            line: 2,
        };
        assert_eq!(mutex.__try_lock(lock_source), Ok(()));
        scheduler.wait_mutex(mutex.clone(), lock_source);
        boot_thread.borrow_mut().shutdown = true;
        mutex.unlock();

        scheduler.schedule();

        assert!(Rc::ptr_eq(
            scheduler.running_thread.as_ref().unwrap(),
            &next_thread
        ));
        assert!(scheduler.mutex_wait_threads.is_empty());
        assert!(boot_thread.borrow().waiting_mutex.is_none());
        assert_eq!(mutex.__try_lock(lock_source), Ok(()));
        mutex.unlock();
    }
}
