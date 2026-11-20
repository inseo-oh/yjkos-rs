use core::ptr;

use crate::{klib, print, println};
use alloc::{
    boxed::Box,
    rc::Rc,
    string::{String, ToString},
    vec::Vec,
};

struct RebootCommand;
impl<HW: klib::hal::Hardware> Command<HW> for RebootCommand {
    fn name(&self) -> &'static str {
        "reboot"
    }
    fn help(&self) -> &'static str {
        "reboots the system"
    }
    fn parameter(
        &self,
        _arguments: &[String],
        _argument_index: usize,
    ) -> Option<Box<dyn Completable>> {
        None
    }

    fn execute(&self, _arguments: &[String], _kglobals: &'static klib::KGlobals<HW>) {
        HW::reboot();
    }
}

struct ShowCommand;
impl<HW: klib::hal::Hardware> Command<HW> for ShowCommand {
    fn name(&self) -> &'static str {
        "show"
    }
    fn help(&self) -> &'static str {
        "shows information about kernel"
    }
    fn parameter(
        &self,
        _arguments: &[String],
        argument_index: usize,
    ) -> Option<Box<dyn Completable>> {
        match argument_index {
            0 => Some(KeywordParameter::new(&[
                Keyword::new("pmemory", "physical memory"),
                Keyword::new("vmemory", "virtual memory"),
                Keyword::new("hmemory", "kernel heap memory"),
                Keyword::new("scheduler", "task scheduler"),
                Keyword::new("ticktime", "ticktime value"),
                Keyword::new("traps", "exception/interrupt traps"),
            ])),
            _ => None,
        }
    }

    fn execute(&self, arguments: &[String], kglobals: &'static klib::KGlobals<HW>) {
        let typ = if let Some(a) = arguments.first() {
            a
        } else {
            println!("usage: show <type>");
            return;
        };
        if typ == "pmemory" {
            println!("# Physical memory information");
            println!();
            println!("| BASE       | SIZE       | PAGES   |");
            println!("+------------+------------+---------+");
            let pmm = kglobals.pmm.lock();
            for pool in pmm.pools() {
                let base = pool.base_addr().addr();
                println!(
                    "| {:#10x} | {:7}KiB | {:7} |",
                    base,
                    (pool.page_count() * HW::PAGE_SIZE) / 1024,
                    pool.page_count()
                );
            }
            println!();
            println!(
                "Total memory is around {}MiB",
                pmm.get_total_mem_size() / (1024 * 1024)
            );
            println!("(Not including memory used for kernel image or computer itself)");
        } else if typ == "vmemory" {
            let vmm = kglobals.vmm.lock();
            let kas_h = vmm.existing_kernel_address_space().unwrap();
            let kas = kas_h.borrow();
            println!("# Virtual memory information (Kernel address space)");
            println!();
            println!("1. Unallocated virtual memory regions");
            println!("| BASE       | SIZE       | PAGES   |");
            println!("+------------+------------+---------+");
            for obj_h in kas.objects() {
                let obj = obj_h.borrow();
                let base = obj.start.addr();
                println!(
                    "| {:#10x} | {:7}KiB | {:7} |",
                    base,
                    (obj.size) / 1024,
                    (obj.size / HW::PAGE_SIZE)
                );
            }
            println!();
            println!("2. Uncommited virtual memory regions");
            println!(
                "| PHYS-BASE  | VIRT-BASE  | SIZE       | PAGES   | UNCOMMITED PAGES    | FLAGS |"
            );
            println!(
                "+------------+------------+------------+---------+---------------------+-------+"
            );
            for uobj_h in kas.uncommitted_objects() {
                let uobj = uobj_h.borrow();
                let uncommited_count = uobj.uncommited_count();
                let obj = uobj.object().borrow();
                let vbase = obj.start.addr();
                if let Some(pbase) = obj.phys_base.map(klib::PhysicalPtr::addr) {
                    print!("| {:#10x} ", pbase,);
                } else {
                    print!("| N/A        ");
                }
                print!(
                    "| {:#10x} | {:7}KiB | {:7} | {:7}({:7}KiB) | ",
                    vbase,
                    obj.size / 1024,
                    (obj.size / HW::PAGE_SIZE),
                    uncommited_count,
                    (uncommited_count * HW::PAGE_SIZE) / 1024,
                );
                print!("{}", if obj.flags.read { "R" } else { "-" });
                print!("{}", if obj.flags.write { "W" } else { "-" });
                print!("{}", if obj.flags.no_cache { "I" } else { "-" });
                println!("   |");
            }
            println!();
            println!("FLAGS: R=Read, W=Write, I=(cache)Inhibit");
        } else if typ == "hmemory" {
            println!("# Heap memory information");
            let heap = kglobals.heap.lock();
            println!();
            println!("1. Pools");
            println!(
                "| NO. | TOTAL               | USED                      | PAGE COUNT           |"
            );
            println!(
                "+-----+---------------------+---------------------------+----------------------+"
            );
            for (n, pool) in heap.pools().enumerate() {
                println!(
                    "| {:3} | {:7} ({:6}KiB) | {:7} ({:6}KiB, {:3}%) | {:7} ({:7}KiB) |",
                    n,
                    pool.block_count(),
                    (pool.block_count() * klib::mem::heap::BLOCK_SIZE) / 1024,
                    pool.used_block_count(),
                    (pool.used_block_count() * klib::mem::heap::BLOCK_SIZE) / 1024,
                    (pool.used_block_count() * 100) / pool.block_count(),
                    pool.page_count(),
                    (pool.page_count() * HW::PAGE_SIZE) / 1024,
                );
            }
            println!();
            println!("2. Allocations");
            println!("| NO. | HEAP NO. | SIZE      | BLOCK COUNT         |");
            println!("+-----+----------+-----------+---------------------+");
            for (n, alloc) in heap.allocs().enumerate() {
                println!(
                    "| {:3} | {:8} | {:6}KiB | {:7} ({:6}KiB) |",
                    n,
                    heap.pools()
                        .position(|h| ptr::eq(h, &alloc.pool().data))
                        .expect("no heap contains this allocation"),
                    alloc.size() / 1024,
                    alloc.block_count(),
                    (alloc.block_count() * klib::mem::heap::BLOCK_SIZE) / 1024,
                );
            }
            println!();
            println!(
                "Total free memory is {} blocks ({}KiB)",
                heap.free_block_count(),
                (heap.free_block_count() * klib::mem::heap::BLOCK_SIZE) / 1024
            );
        } else if typ == "scheduler" {
            println!("# Scheduler information");
            let sched = kglobals.sched.lock();
            println!();
            println!("1. Scheduler queues");
            println!("| NO. | PRIORITY | OPPORTUNITIES | THREADS |");
            println!("+-----+----------+---------------+---------+");
            for (n, (priority, opportunities, thread_count)) in sched.queue_stats().enumerate() {
                println!(
                    "| {:3} | {:8} | {:3} | {:10} |",
                    n, priority, opportunities, thread_count,
                );
            }
            println!();
            println!("2. Processes");
            println!("| NO. | PRIORITY | THREADS |");
            println!("+-----+----------+---------+");
            for (n, proc_h) in sched.processes().enumerate() {
                let proc = proc_h.borrow();
                println!(
                    "| {:3} | {:8} | {:7} |",
                    n,
                    proc.priority(),
                    proc.threads().count(),
                );
            }
            println!();
            println!("3. Threads");
            println!("| NO. | PROC.NO. | STATE      |");
            println!("+-----+----------+------------+");
            for (n, (thr_h, thr_state)) in sched.threads().enumerate() {
                let thr = thr_h.borrow();
                println!(
                    "| {:3} | {:8} | {:10} |",
                    n,
                    sched
                        .processes()
                        .position(|proc_h| Rc::ptr_eq(&proc_h, &thr.process()))
                        .expect("thread's process could not be found"),
                    match thr_state {
                        klib::tasks::sched::ThreadState::Running => "RUNNING",
                        klib::tasks::sched::ThreadState::Queued => "QUEUED",
                        klib::tasks::sched::ThreadState::MutexWait => "MUTEX-WAIT",
                    },
                );
            }
            println!();
            println!(
                "Mutex waiting threads: {}",
                sched.mutex_waiting_thread_count()
            );
        } else if typ == "ticktime" {
            println!("Current ticktime: {}", klib::ticktime::load());
        } else if typ == "traps" {
            println!("# Traps information");
            let trap_manager = kglobals.trap_manager.lock();
            println!();
            println!("| NO. | HANDLERS   |");
            println!("+-----+------------+");
            for n in 0..klib::traps::TRAP_COUNT {
                println!("| {:3} | {:10} |", n, trap_manager.handler_count(n));
            }
        } else {
            unreachable!();
        }
    }
}

struct TestCommand;
impl<HW: klib::hal::Hardware> Command<HW> for TestCommand {
    fn name(&self) -> &'static str {
        "test"
    }
    fn help(&self) -> &'static str {
        "run a diagnostic test"
    }
    fn parameter(
        &self,
        _arguments: &[String],
        argument_index: usize,
    ) -> Option<Box<dyn Completable>> {
        match argument_index {
            0 => Some(KeywordParameter::new(&[
                Keyword::new("thread", "run the multithreading stress test"),
                Keyword::new(
                    "usermode",
                    "test switching between ring 3 processes (ends at the expected general protection fault)",
                ),
            ])),
            _ => None,
        }
    }

    fn execute(&self, arguments: &[String], kglobals: &'static klib::KGlobals<HW>) {
        if arguments.first().is_some_and(|arg| arg == "thread") {
            if let Err(error) =
                klib::diag::thread::run(kglobals, klib::diag::thread::DEFAULT_DURATION_SECS)
            {
                println!("[FAIL] {}", error);
            }
        } else if arguments.first().is_some_and(|arg| arg == "usermode") {
            test_user_mode::<HW>(kglobals);
        } else {
            println!("usage: test <thread|usermode>");
        }
    }
}

const USER_MODE_TEST_REGION_START: usize = 0x8000_0000;
const USER_MODE_TEST_TIMEOUT_TICKS: u32 = 5_000;

fn make_user_mode_test_worker(counter_addr: usize, release_addr: usize) -> [u8; 20] {
    // mov dword ptr [counter_addr], 1
    // wait:
    //   mov eax, dword ptr [release_addr]
    //   test eax, eax
    //   jz wait
    //   cli
    let mut code = [
        0xc7, 0x05, 0, 0, 0, 0, 1, 0, 0, 0, 0xa1, 0, 0, 0, 0, 0x85, 0xc0, 0x74, 0xf7, 0xfa,
    ];
    code[2..6].copy_from_slice(&(counter_addr as u32).to_le_bytes());
    code[11..15].copy_from_slice(&(release_addr as u32).to_le_bytes());
    code
}

struct UserModeTestProcess<HW: klib::hal::Hardware> {
    // Keep the process alive; its threads hold only Weak process references.
    _process: klib::tasks::process::ProcessHandle<HW>,
    thread: klib::tasks::thread::ThreadHandle<HW>,
    data_page: klib::PhysicalPtr,
}

fn create_user_mode_test_process<HW: klib::hal::Hardware>(
    kglobals: &'static klib::KGlobals<HW>,
) -> Result<UserModeTestProcess<HW>, &'static str> {
    let user_address_space = klib::mem::vmm::AddressSpace::<HW>::new(
        USER_MODE_TEST_REGION_START as *mut u8,
        HW::KERNEL_SPACE_BASE_ADDRESS - USER_MODE_TEST_REGION_START,
        true,
    );
    let code_pages = match kglobals.pmm.lock().malloc(1) {
        Ok(pages) => pages,
        Err(error) => return Err(error),
    };
    let data_pages = match kglobals.pmm.lock().malloc(1) {
        Ok(pages) => pages,
        Err(error) => return Err(error),
    };

    // Both processes use the same virtual addresses, backed by distinct
    // physical pages in their private user page tables.
    let code_addr = USER_MODE_TEST_REGION_START as *mut u8;
    let data_addr = (USER_MODE_TEST_REGION_START + HW::PAGE_SIZE) as *mut u8;
    klib::mem::vmm::map_at(
        &user_address_space,
        code_addr,
        code_pages.base,
        HW::PAGE_SIZE,
        klib::mem::vmm::MapFlags {
            read: true,
            write: false,
            exec: true,
            no_cache: false,
        },
    )?;
    klib::mem::vmm::map_at(
        &user_address_space,
        data_addr,
        data_pages.base,
        HW::PAGE_SIZE,
        klib::mem::vmm::MapFlags {
            read: true,
            write: true,
            exec: false,
            no_cache: false,
        },
    )?;

    user_address_space.borrow_mut().with_active(|| {
        HW::mmu_map(
            code_addr,
            code_pages.base,
            1,
            klib::mem::vmm::MapFlags {
                read: true,
                write: false,
                exec: true,
                no_cache: false,
            },
            true,
        )?;
        HW::mmu_map(
            data_addr,
            data_pages.base,
            1,
            klib::mem::vmm::MapFlags {
                read: true,
                write: true,
                exec: false,
                no_cache: false,
            },
            true,
        )?;
        Ok::<(), &'static str>(())
    })?;

    let worker_code = make_user_mode_test_worker(data_addr.addr(), data_addr.addr() + 4);
    // The user process writes a progress word then waits for the shell to
    // release it. CLI is privileged at CPL 3 with IOPL 0, so the released
    // process must raise #GP. The trap frame should show CS with RPL 3.
    {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        HW::mmu_scratch_map(code_pages.base, false);
        unsafe {
            core::ptr::copy_nonoverlapping(
                worker_code.as_ptr(),
                HW::SCRATCH_MAP_BASE,
                worker_code.len(),
            )
        }
        HW::mmu_scratch_map(data_pages.base, false);
        unsafe { core::ptr::write_bytes(HW::SCRATCH_MAP_BASE.cast::<u32>(), 0, 2) };
    }

    // Keep the process alive while its thread is queued. Thread stores only a
    // Weak reference to its owning process.
    let user_process = klib::tasks::process::Process::new(user_address_space);
    let thread = klib::tasks::thread::Thread::new(
        kglobals,
        &user_process,
        klib::tasks::thread::THREAD_STACK_SIZE,
        klib::hal::ThreadStart::User {
            entry: code_addr.addr(),
        },
    )?;
    Ok(UserModeTestProcess {
        _process: user_process,
        thread,
        data_page: data_pages.base,
    })
}

fn test_user_mode<HW: klib::hal::Hardware>(kglobals: &'static klib::KGlobals<HW>) {
    if USER_MODE_TEST_REGION_START >= HW::KERNEL_SPACE_BASE_ADDRESS
        || HW::KERNEL_SPACE_BASE_ADDRESS - USER_MODE_TEST_REGION_START
            < klib::tasks::thread::THREAD_STACK_SIZE + HW::PAGE_SIZE * 2
    {
        println!("[FAIL] Hardware has no usable userspace test region");
        return;
    }

    let mut user_processes = Vec::new();
    for index in 0..2 {
        match create_user_mode_test_process(kglobals) {
            Ok(process) => user_processes.push(process),
            Err(error) => {
                println!(
                    "[FAIL] Could not create user-mode process {}: {}",
                    index, error
                );
                return;
            }
        }
    }

    println!("Starting two user processes; expecting both to run before #GP from CLI.");
    {
        // Keep interrupts disabled until the owned switch request has taken
        // over the guard, then restore them as soon as this shell thread runs
        // again so the workers can be observed while the timer keeps ticking.
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        let switch = {
            let mut scheduler = kglobals.sched.lock();
            for process in &user_processes {
                scheduler.queue(process.thread.clone());
            }
            scheduler.prepare_schedule()
        };
        if let Some(switch) = switch {
            switch.switch();
        } else {
            println!("[FAIL] Scheduler did not select a user-mode test thread");
            return;
        }
    }

    let start_tick = klib::ticktime::load();
    let read_progress = || {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        core::array::from_fn(|index| {
            let process = &user_processes[index];
            HW::mmu_scratch_map(process.data_page, false);
            unsafe { core::ptr::read_volatile(HW::SCRATCH_MAP_BASE.cast::<u32>()) }
        })
    };
    let progress = loop {
        let progress = read_progress();
        if progress == [1, 1]
            || klib::ticktime::load().wrapping_sub(start_tick) >= USER_MODE_TEST_TIMEOUT_TICKS
        {
            break progress;
        }
        core::hint::spin_loop();
    };

    {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        for process in &user_processes {
            HW::mmu_scratch_map(process.data_page, false);
            unsafe { core::ptr::write_volatile(HW::SCRATCH_MAP_BASE.cast::<u32>().add(1), 1) };
        }
    }
    if progress == [1, 1] {
        println!("[ OK ] Both user processes ran and were preempted across address-space switches");
    } else {
        println!(
            "[FAIL] User-process progress was {}/{}",
            progress[0], progress[1]
        );
    }

    // Hand off once more. A released user process reaches CLI, producing the
    // expected #GP with a ring-3 code selector; that exception halts the test.
    let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
    let switch = {
        let mut scheduler = kglobals.sched.lock();
        scheduler.prepare_schedule()
    };
    if let Some(switch) = switch {
        switch.switch();
    } else {
        println!("[FAIL] Scheduler did not resume a user-mode test process");
    }
}

fn commands<HW: klib::hal::Hardware>() -> Vec<Box<dyn Command<HW>>> {
    let mut commands: Vec<Box<dyn Command<HW>>> = Vec::new();
    commands.push(Box::new(ShowCommand));
    commands.push(Box::new(TestCommand));
    commands.push(Box::new(RebootCommand));
    commands
}

////////////////////////////////////////////////////////////////////////////////

#[derive(Clone, Copy)]
struct Keyword {
    name: &'static str,
    help: &'static str,
}
impl Keyword {
    pub fn new(name: &'static str, help: &'static str) -> Self {
        Self { name, help }
    }
}
struct KeywordParameter {
    keywords: Vec<Keyword>,
}
impl KeywordParameter {
    pub fn new(keywords: &[Keyword]) -> Box<Self> {
        Box::new(Self {
            keywords: keywords.to_vec(),
        })
    }
}
impl Completable for KeywordParameter {
    fn choices(&self) -> Vec<Suggestion> {
        self.keywords
            .iter()
            .map(|s| Suggestion {
                value: s.name.to_string(),
                help: s.help,
            })
            .collect()
    }
}

////////////////////////////////////////////////////////////////////////////////

trait Command<HW: klib::hal::Hardware> {
    fn name(&self) -> &'static str;
    fn help(&self) -> &'static str;
    fn parameter(
        &self,
        arguments: &[String],
        argument_index: usize,
    ) -> Option<Box<dyn Completable>>;
    fn execute(&self, arguments: &[String], kglobals: &'static klib::KGlobals<HW>);
}

struct CommandNameCompletion<'a, HW: klib::hal::Hardware>(&'a [Box<dyn Command<HW>>]);
impl<HW: klib::hal::Hardware> Completable for CommandNameCompletion<'_, HW> {
    fn choices(&self) -> Vec<Suggestion> {
        self.0
            .iter()
            .map(|c| Suggestion {
                value: c.name().to_string(),
                help: c.help(),
            })
            .collect()
    }
}

#[derive(Clone)]
struct Suggestion {
    value: String,
    help: &'static str,
}
trait Completable {
    fn choices(&self) -> Vec<Suggestion>;
    fn candidates_for(&self, input: &str) -> Vec<Suggestion> {
        self.choices()
            .iter()
            .filter(|&s| s.value.starts_with(input))
            .cloned()
            .collect()
    }
    fn try_autocomplete(&self, input: &str) -> Result<Suggestion, &'static str> {
        let cand = self.candidates_for(input);
        if cand.is_empty() {
            Err("invalid command name or arguments")
        } else if cand.len() != 1 {
            Err("ambiguous command name or arguments")
        } else {
            Ok(cand[0].clone())
        }
    }
}

pub struct Shell<HW: klib::hal::Hardware> {
    command_buf: String,
    commands: Vec<Box<dyn Command<HW>>>,
    kglobals: &'static klib::KGlobals<HW>,
}

impl<HW: klib::hal::Hardware> Shell<HW> {
    pub fn new(kglobals: &'static klib::KGlobals<HW>) -> Self {
        Self {
            command_buf: String::new(),
            commands: commands(),
            kglobals,
        }
    }
    fn command_components(&self) -> Vec<&str> {
        self.command_buf.split(" ").collect()
    }
    fn candidates(&self) -> Vec<Suggestion> {
        let components = self.command_components();
        if components.len() < 2 {
            // Return command suggestions
            return CommandNameCompletion(&self.commands).candidates_for(components[0]);
        }
        let cmd_sugg = if let Ok(name) =
            CommandNameCompletion(&self.commands).try_autocomplete(components[0])
        {
            name
        } else {
            return Vec::new();
        };
        let cmd = self
            .commands
            .iter()
            .find(|c| c.name() == cmd_sugg.value)
            .unwrap();
        let mut args = Vec::new();
        // Autocomplete command except the last component
        let mut param_idx = 0;
        while param_idx + 1 < components.len() - 1 {
            let param = if let Some(arg) = cmd.parameter(&args, param_idx) {
                arg
            } else {
                break;
            };
            args.push(
                if let Ok(arg_sugg) = param.try_autocomplete(components[param_idx + 1]) {
                    arg_sugg.value
                } else {
                    return Vec::new();
                },
            );
            param_idx += 1;
        }
        // Finally we return suggestions for next argument
        cmd.parameter(&args, param_idx)
            .unwrap()
            .candidates_for(components.last().unwrap())
    }
    fn try_autocomplete(&self) -> Result<String, &'static str> {
        let components = self.command_components();
        let cmd_sugg = CommandNameCompletion(&self.commands).try_autocomplete(components[0])?;
        let cmd = self
            .commands
            .iter()
            .find(|c| c.name() == cmd_sugg.value)
            .unwrap();
        let mut args = Vec::new();
        let mut param_idx = 0;
        while param_idx + 1 < components.len() {
            let param = if let Some(arg) = cmd.parameter(&args, param_idx) {
                arg
            } else {
                break;
            };
            args.push(param.try_autocomplete(components[param_idx + 1])?.value);
            param_idx += 1;
        }
        let mut new_comps = Vec::with_capacity(args.len() + 1);
        new_comps.push(cmd_sugg.value);
        for arg in args {
            new_comps.push(arg);
        }
        Ok(new_comps.join(" "))
    }
    fn execute(&self) -> Result<(), &'static str> {
        let components = self.command_components();
        let cmd_sugg = CommandNameCompletion(&self.commands).try_autocomplete(components[0])?;
        let cmd = self
            .commands
            .iter()
            .find(|c| c.name() == cmd_sugg.value)
            .unwrap();
        cmd.execute(
            &components[1..]
                .iter()
                .map(|s| s.to_string())
                .collect::<Vec<String>>(),
            self.kglobals,
        );
        Ok(())
    }
    fn prompt(&mut self) {
        print!("diag> {}", self.command_buf);
        'input: loop {
            let c = klib::console::read().unwrap();
            match c {
                '\r' => {
                    println!();
                    if !self.command_buf.trim().is_empty() {
                        match self.try_autocomplete() {
                            Ok(v) => {
                                self.command_buf = v;
                                if let Err(e) = self.execute() {
                                    println!("bad command: {}: {}", &self.command_buf, e)
                                }
                            }
                            Err(e) => println!("bad command: {}: {}", &self.command_buf, e),
                        }
                        self.command_buf.clear();
                    }
                    break 'input;
                }
                '\t' => {
                    let candidates = self.candidates();
                    if candidates.len() == 1 {
                        let remaining =
                            &candidates[0].value[self.command_components().last().unwrap().len()..];
                        self.command_buf.push_str(remaining);
                        print!("{}", remaining);
                    } else if !candidates.is_empty() {
                        println!();
                        let max_len = candidates.iter().map(|s| s.value.len()).max().unwrap_or(0);
                        for candidate in &candidates {
                            let name_padded = candidate.value.clone()
                                + (&" ".repeat(max_len - candidate.value.len()));
                            println!(" {} - {}", &name_padded, candidate.help);
                        }
                        break 'input;
                    }
                }
                '\x7f' => {
                    if !self.command_buf.is_empty() {
                        self.command_buf.pop();
                        print!("\x08 \x08");
                    }
                }
                _ => {
                    print!("{}", c);
                    self.command_buf.push(c);
                }
            }
        }
    }
    pub fn run(&mut self) -> ! {
        loop {
            self.prompt()
        }
    }
}
