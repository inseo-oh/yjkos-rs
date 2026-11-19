use crate::{
    klib::{self, hal::Hardware},
    println,
};
use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

const THREAD_COUNT: usize = 10;
pub const DEFAULT_DURATION_SECS: u32 = 60;
const TICKS_PER_SEC: u32 = 1000;
const STALL_TIMEOUT_SECS: u8 = 10;

static RUN_STARTED: AtomicBool = AtomicBool::new(false);
static STOP_WORKERS: AtomicBool = AtomicBool::new(false);
static PROGRESS_0: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_1: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_2: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_3: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_4: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_5: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_6: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_7: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_8: AtomicUsize = AtomicUsize::new(0);
static PROGRESS_9: AtomicUsize = AtomicUsize::new(0);
static MUTEX_OWNER_READY: AtomicUsize = AtomicUsize::new(0);
static MUTEX_CONTENDER_WAITING: AtomicUsize = AtomicUsize::new(0);
static MUTEX_CONTENDER_ACQUIRED: AtomicUsize = AtomicUsize::new(0);

struct WorkerContext<'a, HW: Hardware> {
    kglobals: &'a klib::KGlobals<HW>,
    mutex: *const klib::tasks::mutex::Mutex<HW, usize>,
    thread_index: usize,
}

fn progress(thread_index: usize) -> &'static AtomicUsize {
    match thread_index {
        0 => &PROGRESS_0,
        1 => &PROGRESS_1,
        2 => &PROGRESS_2,
        3 => &PROGRESS_3,
        4 => &PROGRESS_4,
        5 => &PROGRESS_5,
        6 => &PROGRESS_6,
        7 => &PROGRESS_7,
        8 => &PROGRESS_8,
        9 => &PROGRESS_9,
        _ => unreachable!(),
    }
}

#[inline(never)]
fn stack_pressure(depth: usize, value: usize) -> usize {
    let frame = [value as u8; 512];
    let nested = if depth == 0 {
        frame[0] as usize
    } else {
        stack_pressure(depth - 1, value.wrapping_add(1))
    };
    core::hint::black_box(&frame);
    nested.wrapping_add(frame[depth % frame.len()] as usize)
}

fn stress_worker<HW: Hardware>(data: usize) {
    let context = unsafe { &*(data as *const WorkerContext<HW>) };
    let mutex = unsafe { &*context.mutex };

    if context.thread_index == 0 {
        let mut shared = crate::mutex_lock!(mutex, &context.kglobals.sched);
        MUTEX_OWNER_READY.store(1, Ordering::Release);
        let start_tick = klib::ticktime::load();
        while MUTEX_CONTENDER_WAITING.load(Ordering::Acquire) == 0
            || klib::ticktime::load().wrapping_sub(start_tick) < 5
        {
            core::hint::spin_loop();
        }
        *shared = shared.wrapping_add(1);
    } else if context.thread_index == 1 {
        while MUTEX_OWNER_READY.load(Ordering::Acquire) == 0 {
            core::hint::spin_loop();
        }
        MUTEX_CONTENDER_WAITING.store(1, Ordering::Release);
        let mut shared = crate::mutex_lock!(mutex, &context.kglobals.sched);
        *shared = shared.wrapping_add(1);
        MUTEX_CONTENDER_ACQUIRED.store(1, Ordering::Release);
    }

    loop {
        if STOP_WORKERS.load(Ordering::Acquire) {
            return;
        }
        let mut shared = crate::mutex_lock!(mutex, &context.kglobals.sched);
        *shared = shared.wrapping_add(1);
        drop(shared);

        let work = stack_pressure(4, context.thread_index);
        core::hint::black_box(work);
        progress(context.thread_index).fetch_add(1, Ordering::Relaxed);
        core::hint::spin_loop();
    }
}

fn return_worker(_data: usize) {}

/// Run the single-CPU threading stress diagnostic.
///
/// This diagnostic can run only once per boot because the scheduler does not
/// yet reclaim finished thread stacks.
pub fn run<HW: Hardware>(
    kglobals: &klib::KGlobals<HW>,
    duration_secs: u32,
) -> Result<(), &'static str> {
    if RUN_STARTED
        .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
        .is_err()
    {
        return Err("thread diagnostic can only run once per boot");
    }
    assert!(duration_secs >= 5, "diagnostic needs at least 5 seconds");
    println!(
        "> Multithreading stress test ({} workers, {}s)",
        THREAD_COUNT, duration_secs
    );
    for thread_index in 0..THREAD_COUNT {
        progress(thread_index).store(0, Ordering::Relaxed);
    }
    STOP_WORKERS.store(false, Ordering::Relaxed);
    MUTEX_OWNER_READY.store(0, Ordering::Relaxed);
    MUTEX_CONTENDER_WAITING.store(0, Ordering::Relaxed);
    MUTEX_CONTENDER_ACQUIRED.store(0, Ordering::Relaxed);

    let test_mutex = klib::tasks::mutex::Mutex::<HW, usize>::new(0);
    let contexts: [WorkerContext<'_, HW>; THREAD_COUNT] =
        core::array::from_fn(|thread_index| WorkerContext {
            kglobals,
            mutex: &test_mutex,
            thread_index,
        });
    let return_target = klib::tasks::thread::thread_entry_returned_count().wrapping_add(1);
    {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        let mut scheduler = kglobals.sched.lock();
        for thread_index in 0..THREAD_COUNT {
            let context = core::ptr::addr_of!(contexts[thread_index]);
            let thread = klib::tasks::thread::Thread::new(
                &scheduler.kernel_process(),
                klib::tasks::thread::THREAD_STACK_SIZE,
                stress_worker::<HW>,
                context as usize,
            );
            scheduler.queue(thread);
        }
        let returning_thread = klib::tasks::thread::Thread::new(
            &scheduler.kernel_process(),
            klib::tasks::thread::THREAD_STACK_SIZE,
            return_worker,
            0,
        );
        scheduler.queue(returning_thread);
    }

    let mut test_passed = true;
    let mut stalled_seconds = [0u8; THREAD_COUNT];
    for second in 1..=duration_secs {
        let start_tick = klib::ticktime::load();
        let before = core::array::from_fn::<_, THREAD_COUNT, _>(|index| {
            progress(index).load(Ordering::Relaxed)
        });
        while klib::ticktime::load().wrapping_sub(start_tick) < TICKS_PER_SEC {
            core::hint::spin_loop();
        }
        for (thread_index, previous_count) in before.iter().enumerate() {
            let current_count = progress(thread_index).load(Ordering::Relaxed);
            if current_count == *previous_count {
                stalled_seconds[thread_index] += 1;
                if stalled_seconds[thread_index] >= STALL_TIMEOUT_SECS {
                    println!(
                        "[FAIL] Worker thread {} made no progress for {}s",
                        thread_index, STALL_TIMEOUT_SECS
                    );
                    test_passed = false;
                    break;
                }
            } else {
                stalled_seconds[thread_index] = 0;
            }
        }
        if !test_passed {
            break;
        }
        if second == 5 {
            if MUTEX_CONTENDER_ACQUIRED.load(Ordering::Acquire) == 0 {
                println!("[FAIL] Mutex contender did not acquire the released lock");
                test_passed = false;
                break;
            }
            if klib::tasks::thread::thread_entry_returned_count() < return_target {
                println!("[FAIL] Returning thread did not reach its exit trampoline");
                test_passed = false;
                break;
            }
            println!("[ OK ] Mutex contention and thread return path");
        }
        if second == 1 || second % 10 == 0 {
            println!("  workers made progress ({}/{}s)", second, duration_secs);
        }
    }
    if test_passed {
        println!(
            "[ OK ] All {} workers ran for {}s",
            THREAD_COUNT, duration_secs
        );
    }

    STOP_WORKERS.store(true, Ordering::Release);
    let expected_returned_count = return_target.wrapping_add(THREAD_COUNT);
    while klib::tasks::thread::thread_entry_returned_count() < expected_returned_count {
        core::hint::spin_loop();
    }
    println!("[ OK ] Stress workers stopped");

    if test_passed {
        Ok(())
    } else {
        Err("thread stress test failed")
    }
}
