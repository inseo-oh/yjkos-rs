use crate::{kglobals, klib::hal::Hardware, println};

#[repr(C, align(4))]
pub struct TrapFrame {
    pub(super) gs: u32,
    pub(super) fs: u32,
    pub(super) es: u32,
    pub(super) ds: u32,
    pub(super) edi: u32,
    pub(super) esi: u32,
    pub(super) ebp: u32,
    // This is only included as part of PUSHAD doing its thing.
    // It does not actually represent the original ESP value before servicing the interrupt/exception.
    _esp_useless: u32,
    pub(super) ebx: u32,
    pub(super) edx: u32,
    pub(super) ecx: u32,
    pub(super) eax: u32,
    pub(super) err_code: u32,
    pub(super) eip: u32,
    pub(super) cs: u32,
    pub(super) eflags: u32,
    /* Only valid if CS != 0. Do NOT access when CS == 0 */
    pub(super) esp_usermode: u32,
    pub(super) ss_usermode: u32,
}
impl TrapFrame {
    pub fn dump(&self) {
        println!(
            "eax={:08x} ebx={:08x} ecx={:08x} edx={:08x} esi={:08x} edi={:08x}",
            self.eax, self.ebx, self.ecx, self.edx, self.esi, self.edi
        );
        println!(
            "ebp={:08x} eip={:08x} efl={:08x} cs ={:08x} ds ={:08x} es ={:08x}",
            self.ebp, self.eip, self.eflags, self.cs, self.ds, self.es
        );
        println!("fs ={:08x} gs ={:08x}", self.fs, self.gs);
    }
}

fn default_handler(trap_num: usize, trap_frame: *mut TrapFrame) {
    let frame = unsafe { trap_frame.as_mut().unwrap() };
    println!(
        "fatal exception {} occured (error code {:#x})",
        trap_num, frame.err_code
    );
    unsafe { trap_frame.as_ref().unwrap().dump() };
    super::Hardware::hcf();
}

const PF_FLAG_P: u32 = 1 << 0; // Present
const PF_FLAG_W: u32 = 1 << 1; // Write
const PF_FLAG_U: u32 = 1 << 2; // User

fn page_fault_handler(_trap_num: usize, trap_frame: *mut TrapFrame) {
    let frame = unsafe { trap_frame.as_mut().unwrap() };
    let fault_addr = super::read_cr2();
    let globals = kglobals();
    let mut vmm = globals.vmm.lock();
    vmm.page_fault(
        |page_count| globals.pmm.lock().malloc(page_count),
        fault_addr as *mut u8,
        (frame.err_code & PF_FLAG_P) != 0,
        (frame.err_code & PF_FLAG_W) != 0,
        (frame.err_code & PF_FLAG_U) != 0,
        frame,
        &mut globals.sched.lock(),
    );
}

pub fn init() {
    let globals = kglobals();
    let mut trap_manager = globals.trap_manager.lock();
    for i in 0..32 {
        match i {
            14 => trap_manager.register_trap(i, page_fault_handler).unwrap(),
            _ => trap_manager.register_trap(i, default_handler).unwrap(),
        }
    }
}
