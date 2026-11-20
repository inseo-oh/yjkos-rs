use core::mem::size_of;

use crate::klib::hal::Hardware;

const IDT_FLAG_TYPE_INT32: u8 = 0xe;
const IDT_FLAG_TYPE_TRAP32: u8 = 0xf;
const fn idt_flag_dpl(n: u8) -> u8 {
    n << 5
}
const IDT_FLAG_DPL0: u8 = idt_flag_dpl(0);
const IDT_FLAG_DPL1: u8 = idt_flag_dpl(1);
const IDT_FLAG_DPL2: u8 = idt_flag_dpl(2);
const IDT_FLAG_DPL3: u8 = idt_flag_dpl(3);
const IDT_FLAG_P: u8 = 1 << 7;

#[repr(C, align(4))]
struct GateDescriptor {
    offset_b15_to_b0: u16,
    segment_selector: u16,
    _reserved0: u8,
    flags: u8,
    offset_b31_to_b16: u16,
}

impl GateDescriptor {
    const fn new(offset: u32, flags: u8) -> Self {
        Self {
            offset_b15_to_b0: (offset & 0xffff) as u16,
            segment_selector: super::gdt::GDT_KERNEL_CS,
            _reserved0: 0,
            flags,
            offset_b31_to_b16: (offset >> 16) as u16,
        }
    }
}
const _: () = assert!(size_of::<GateDescriptor>() == 8);

type Handler = unsafe extern "C" fn();
static KERNEL_TRAPS: [Handler; 32] = [
    super::isr_entry::hal_i586_isr_exception0_entry,
    super::isr_entry::hal_i586_isr_exception1_entry,
    super::isr_entry::hal_i586_isr_exception2_entry,
    super::isr_entry::hal_i586_isr_exception3_entry,
    super::isr_entry::hal_i586_isr_exception4_entry,
    super::isr_entry::hal_i586_isr_exception5_entry,
    super::isr_entry::hal_i586_isr_exception6_entry,
    super::isr_entry::hal_i586_isr_exception7_entry,
    super::isr_entry::hal_i586_isr_exception8_entry,
    super::isr_entry::hal_i586_isr_exception9_entry,
    super::isr_entry::hal_i586_isr_exception10_entry,
    super::isr_entry::hal_i586_isr_exception11_entry,
    super::isr_entry::hal_i586_isr_exception12_entry,
    super::isr_entry::hal_i586_isr_exception13_entry,
    super::isr_entry::hal_i586_isr_exception14_entry,
    super::isr_entry::hal_i586_isr_exception15_entry,
    super::isr_entry::hal_i586_isr_exception16_entry,
    super::isr_entry::hal_i586_isr_exception17_entry,
    super::isr_entry::hal_i586_isr_exception18_entry,
    super::isr_entry::hal_i586_isr_exception19_entry,
    super::isr_entry::hal_i586_isr_exception20_entry,
    super::isr_entry::hal_i586_isr_exception21_entry,
    super::isr_entry::hal_i586_isr_exception22_entry,
    super::isr_entry::hal_i586_isr_exception23_entry,
    super::isr_entry::hal_i586_isr_exception24_entry,
    super::isr_entry::hal_i586_isr_exception25_entry,
    super::isr_entry::hal_i586_isr_exception26_entry,
    super::isr_entry::hal_i586_isr_exception27_entry,
    super::isr_entry::hal_i586_isr_exception28_entry,
    super::isr_entry::hal_i586_isr_exception29_entry,
    super::isr_entry::hal_i586_isr_exception30_entry,
    super::isr_entry::hal_i586_isr_exception31_entry,
];
static KERNEL_INTERRUPT_HANDLERS: [Handler; 224] = [
    super::isr_entry::hal_i586_isr_interrupt32_entry,
    super::isr_entry::hal_i586_isr_interrupt33_entry,
    super::isr_entry::hal_i586_isr_interrupt34_entry,
    super::isr_entry::hal_i586_isr_interrupt35_entry,
    super::isr_entry::hal_i586_isr_interrupt36_entry,
    super::isr_entry::hal_i586_isr_interrupt37_entry,
    super::isr_entry::hal_i586_isr_interrupt38_entry,
    super::isr_entry::hal_i586_isr_interrupt39_entry,
    super::isr_entry::hal_i586_isr_interrupt40_entry,
    super::isr_entry::hal_i586_isr_interrupt41_entry,
    super::isr_entry::hal_i586_isr_interrupt42_entry,
    super::isr_entry::hal_i586_isr_interrupt43_entry,
    super::isr_entry::hal_i586_isr_interrupt44_entry,
    super::isr_entry::hal_i586_isr_interrupt45_entry,
    super::isr_entry::hal_i586_isr_interrupt46_entry,
    super::isr_entry::hal_i586_isr_interrupt47_entry,
    super::isr_entry::hal_i586_isr_interrupt48_entry,
    super::isr_entry::hal_i586_isr_interrupt49_entry,
    super::isr_entry::hal_i586_isr_interrupt50_entry,
    super::isr_entry::hal_i586_isr_interrupt51_entry,
    super::isr_entry::hal_i586_isr_interrupt52_entry,
    super::isr_entry::hal_i586_isr_interrupt53_entry,
    super::isr_entry::hal_i586_isr_interrupt54_entry,
    super::isr_entry::hal_i586_isr_interrupt55_entry,
    super::isr_entry::hal_i586_isr_interrupt56_entry,
    super::isr_entry::hal_i586_isr_interrupt57_entry,
    super::isr_entry::hal_i586_isr_interrupt58_entry,
    super::isr_entry::hal_i586_isr_interrupt59_entry,
    super::isr_entry::hal_i586_isr_interrupt60_entry,
    super::isr_entry::hal_i586_isr_interrupt61_entry,
    super::isr_entry::hal_i586_isr_interrupt62_entry,
    super::isr_entry::hal_i586_isr_interrupt63_entry,
    super::isr_entry::hal_i586_isr_interrupt64_entry,
    super::isr_entry::hal_i586_isr_interrupt65_entry,
    super::isr_entry::hal_i586_isr_interrupt66_entry,
    super::isr_entry::hal_i586_isr_interrupt67_entry,
    super::isr_entry::hal_i586_isr_interrupt68_entry,
    super::isr_entry::hal_i586_isr_interrupt69_entry,
    super::isr_entry::hal_i586_isr_interrupt70_entry,
    super::isr_entry::hal_i586_isr_interrupt71_entry,
    super::isr_entry::hal_i586_isr_interrupt72_entry,
    super::isr_entry::hal_i586_isr_interrupt73_entry,
    super::isr_entry::hal_i586_isr_interrupt74_entry,
    super::isr_entry::hal_i586_isr_interrupt75_entry,
    super::isr_entry::hal_i586_isr_interrupt76_entry,
    super::isr_entry::hal_i586_isr_interrupt77_entry,
    super::isr_entry::hal_i586_isr_interrupt78_entry,
    super::isr_entry::hal_i586_isr_interrupt79_entry,
    super::isr_entry::hal_i586_isr_interrupt80_entry,
    super::isr_entry::hal_i586_isr_interrupt81_entry,
    super::isr_entry::hal_i586_isr_interrupt82_entry,
    super::isr_entry::hal_i586_isr_interrupt83_entry,
    super::isr_entry::hal_i586_isr_interrupt84_entry,
    super::isr_entry::hal_i586_isr_interrupt85_entry,
    super::isr_entry::hal_i586_isr_interrupt86_entry,
    super::isr_entry::hal_i586_isr_interrupt87_entry,
    super::isr_entry::hal_i586_isr_interrupt88_entry,
    super::isr_entry::hal_i586_isr_interrupt89_entry,
    super::isr_entry::hal_i586_isr_interrupt90_entry,
    super::isr_entry::hal_i586_isr_interrupt91_entry,
    super::isr_entry::hal_i586_isr_interrupt92_entry,
    super::isr_entry::hal_i586_isr_interrupt93_entry,
    super::isr_entry::hal_i586_isr_interrupt94_entry,
    super::isr_entry::hal_i586_isr_interrupt95_entry,
    super::isr_entry::hal_i586_isr_interrupt96_entry,
    super::isr_entry::hal_i586_isr_interrupt97_entry,
    super::isr_entry::hal_i586_isr_interrupt98_entry,
    super::isr_entry::hal_i586_isr_interrupt99_entry,
    super::isr_entry::hal_i586_isr_interrupt100_entry,
    super::isr_entry::hal_i586_isr_interrupt101_entry,
    super::isr_entry::hal_i586_isr_interrupt102_entry,
    super::isr_entry::hal_i586_isr_interrupt103_entry,
    super::isr_entry::hal_i586_isr_interrupt104_entry,
    super::isr_entry::hal_i586_isr_interrupt105_entry,
    super::isr_entry::hal_i586_isr_interrupt106_entry,
    super::isr_entry::hal_i586_isr_interrupt107_entry,
    super::isr_entry::hal_i586_isr_interrupt108_entry,
    super::isr_entry::hal_i586_isr_interrupt109_entry,
    super::isr_entry::hal_i586_isr_interrupt110_entry,
    super::isr_entry::hal_i586_isr_interrupt111_entry,
    super::isr_entry::hal_i586_isr_interrupt112_entry,
    super::isr_entry::hal_i586_isr_interrupt113_entry,
    super::isr_entry::hal_i586_isr_interrupt114_entry,
    super::isr_entry::hal_i586_isr_interrupt115_entry,
    super::isr_entry::hal_i586_isr_interrupt116_entry,
    super::isr_entry::hal_i586_isr_interrupt117_entry,
    super::isr_entry::hal_i586_isr_interrupt118_entry,
    super::isr_entry::hal_i586_isr_interrupt119_entry,
    super::isr_entry::hal_i586_isr_interrupt120_entry,
    super::isr_entry::hal_i586_isr_interrupt121_entry,
    super::isr_entry::hal_i586_isr_interrupt122_entry,
    super::isr_entry::hal_i586_isr_interrupt123_entry,
    super::isr_entry::hal_i586_isr_interrupt124_entry,
    super::isr_entry::hal_i586_isr_interrupt125_entry,
    super::isr_entry::hal_i586_isr_interrupt126_entry,
    super::isr_entry::hal_i586_isr_interrupt127_entry,
    super::isr_entry::hal_i586_isr_interrupt128_entry,
    super::isr_entry::hal_i586_isr_interrupt129_entry,
    super::isr_entry::hal_i586_isr_interrupt130_entry,
    super::isr_entry::hal_i586_isr_interrupt131_entry,
    super::isr_entry::hal_i586_isr_interrupt132_entry,
    super::isr_entry::hal_i586_isr_interrupt133_entry,
    super::isr_entry::hal_i586_isr_interrupt134_entry,
    super::isr_entry::hal_i586_isr_interrupt135_entry,
    super::isr_entry::hal_i586_isr_interrupt136_entry,
    super::isr_entry::hal_i586_isr_interrupt137_entry,
    super::isr_entry::hal_i586_isr_interrupt138_entry,
    super::isr_entry::hal_i586_isr_interrupt139_entry,
    super::isr_entry::hal_i586_isr_interrupt140_entry,
    super::isr_entry::hal_i586_isr_interrupt141_entry,
    super::isr_entry::hal_i586_isr_interrupt142_entry,
    super::isr_entry::hal_i586_isr_interrupt143_entry,
    super::isr_entry::hal_i586_isr_interrupt144_entry,
    super::isr_entry::hal_i586_isr_interrupt145_entry,
    super::isr_entry::hal_i586_isr_interrupt146_entry,
    super::isr_entry::hal_i586_isr_interrupt147_entry,
    super::isr_entry::hal_i586_isr_interrupt148_entry,
    super::isr_entry::hal_i586_isr_interrupt149_entry,
    super::isr_entry::hal_i586_isr_interrupt150_entry,
    super::isr_entry::hal_i586_isr_interrupt151_entry,
    super::isr_entry::hal_i586_isr_interrupt152_entry,
    super::isr_entry::hal_i586_isr_interrupt153_entry,
    super::isr_entry::hal_i586_isr_interrupt154_entry,
    super::isr_entry::hal_i586_isr_interrupt155_entry,
    super::isr_entry::hal_i586_isr_interrupt156_entry,
    super::isr_entry::hal_i586_isr_interrupt157_entry,
    super::isr_entry::hal_i586_isr_interrupt158_entry,
    super::isr_entry::hal_i586_isr_interrupt159_entry,
    super::isr_entry::hal_i586_isr_interrupt160_entry,
    super::isr_entry::hal_i586_isr_interrupt161_entry,
    super::isr_entry::hal_i586_isr_interrupt162_entry,
    super::isr_entry::hal_i586_isr_interrupt163_entry,
    super::isr_entry::hal_i586_isr_interrupt164_entry,
    super::isr_entry::hal_i586_isr_interrupt165_entry,
    super::isr_entry::hal_i586_isr_interrupt166_entry,
    super::isr_entry::hal_i586_isr_interrupt167_entry,
    super::isr_entry::hal_i586_isr_interrupt168_entry,
    super::isr_entry::hal_i586_isr_interrupt169_entry,
    super::isr_entry::hal_i586_isr_interrupt170_entry,
    super::isr_entry::hal_i586_isr_interrupt171_entry,
    super::isr_entry::hal_i586_isr_interrupt172_entry,
    super::isr_entry::hal_i586_isr_interrupt173_entry,
    super::isr_entry::hal_i586_isr_interrupt174_entry,
    super::isr_entry::hal_i586_isr_interrupt175_entry,
    super::isr_entry::hal_i586_isr_interrupt176_entry,
    super::isr_entry::hal_i586_isr_interrupt177_entry,
    super::isr_entry::hal_i586_isr_interrupt178_entry,
    super::isr_entry::hal_i586_isr_interrupt179_entry,
    super::isr_entry::hal_i586_isr_interrupt180_entry,
    super::isr_entry::hal_i586_isr_interrupt181_entry,
    super::isr_entry::hal_i586_isr_interrupt182_entry,
    super::isr_entry::hal_i586_isr_interrupt183_entry,
    super::isr_entry::hal_i586_isr_interrupt184_entry,
    super::isr_entry::hal_i586_isr_interrupt185_entry,
    super::isr_entry::hal_i586_isr_interrupt186_entry,
    super::isr_entry::hal_i586_isr_interrupt187_entry,
    super::isr_entry::hal_i586_isr_interrupt188_entry,
    super::isr_entry::hal_i586_isr_interrupt189_entry,
    super::isr_entry::hal_i586_isr_interrupt190_entry,
    super::isr_entry::hal_i586_isr_interrupt191_entry,
    super::isr_entry::hal_i586_isr_interrupt192_entry,
    super::isr_entry::hal_i586_isr_interrupt193_entry,
    super::isr_entry::hal_i586_isr_interrupt194_entry,
    super::isr_entry::hal_i586_isr_interrupt195_entry,
    super::isr_entry::hal_i586_isr_interrupt196_entry,
    super::isr_entry::hal_i586_isr_interrupt197_entry,
    super::isr_entry::hal_i586_isr_interrupt198_entry,
    super::isr_entry::hal_i586_isr_interrupt199_entry,
    super::isr_entry::hal_i586_isr_interrupt200_entry,
    super::isr_entry::hal_i586_isr_interrupt201_entry,
    super::isr_entry::hal_i586_isr_interrupt202_entry,
    super::isr_entry::hal_i586_isr_interrupt203_entry,
    super::isr_entry::hal_i586_isr_interrupt204_entry,
    super::isr_entry::hal_i586_isr_interrupt205_entry,
    super::isr_entry::hal_i586_isr_interrupt206_entry,
    super::isr_entry::hal_i586_isr_interrupt207_entry,
    super::isr_entry::hal_i586_isr_interrupt208_entry,
    super::isr_entry::hal_i586_isr_interrupt209_entry,
    super::isr_entry::hal_i586_isr_interrupt210_entry,
    super::isr_entry::hal_i586_isr_interrupt211_entry,
    super::isr_entry::hal_i586_isr_interrupt212_entry,
    super::isr_entry::hal_i586_isr_interrupt213_entry,
    super::isr_entry::hal_i586_isr_interrupt214_entry,
    super::isr_entry::hal_i586_isr_interrupt215_entry,
    super::isr_entry::hal_i586_isr_interrupt216_entry,
    super::isr_entry::hal_i586_isr_interrupt217_entry,
    super::isr_entry::hal_i586_isr_interrupt218_entry,
    super::isr_entry::hal_i586_isr_interrupt219_entry,
    super::isr_entry::hal_i586_isr_interrupt220_entry,
    super::isr_entry::hal_i586_isr_interrupt221_entry,
    super::isr_entry::hal_i586_isr_interrupt222_entry,
    super::isr_entry::hal_i586_isr_interrupt223_entry,
    super::isr_entry::hal_i586_isr_interrupt224_entry,
    super::isr_entry::hal_i586_isr_interrupt225_entry,
    super::isr_entry::hal_i586_isr_interrupt226_entry,
    super::isr_entry::hal_i586_isr_interrupt227_entry,
    super::isr_entry::hal_i586_isr_interrupt228_entry,
    super::isr_entry::hal_i586_isr_interrupt229_entry,
    super::isr_entry::hal_i586_isr_interrupt230_entry,
    super::isr_entry::hal_i586_isr_interrupt231_entry,
    super::isr_entry::hal_i586_isr_interrupt232_entry,
    super::isr_entry::hal_i586_isr_interrupt233_entry,
    super::isr_entry::hal_i586_isr_interrupt234_entry,
    super::isr_entry::hal_i586_isr_interrupt235_entry,
    super::isr_entry::hal_i586_isr_interrupt236_entry,
    super::isr_entry::hal_i586_isr_interrupt237_entry,
    super::isr_entry::hal_i586_isr_interrupt238_entry,
    super::isr_entry::hal_i586_isr_interrupt239_entry,
    super::isr_entry::hal_i586_isr_interrupt240_entry,
    super::isr_entry::hal_i586_isr_interrupt241_entry,
    super::isr_entry::hal_i586_isr_interrupt242_entry,
    super::isr_entry::hal_i586_isr_interrupt243_entry,
    super::isr_entry::hal_i586_isr_interrupt244_entry,
    super::isr_entry::hal_i586_isr_interrupt245_entry,
    super::isr_entry::hal_i586_isr_interrupt246_entry,
    super::isr_entry::hal_i586_isr_interrupt247_entry,
    super::isr_entry::hal_i586_isr_interrupt248_entry,
    super::isr_entry::hal_i586_isr_interrupt249_entry,
    super::isr_entry::hal_i586_isr_interrupt250_entry,
    super::isr_entry::hal_i586_isr_interrupt251_entry,
    super::isr_entry::hal_i586_isr_interrupt252_entry,
    super::isr_entry::hal_i586_isr_interrupt253_entry,
    super::isr_entry::hal_i586_isr_interrupt254_entry,
    super::isr_entry::hal_i586_isr_interrupt255_entry,
];

#[repr(C, align(4))]
struct InterruptDescriptorTable([GateDescriptor; 256]);
impl InterruptDescriptorTable {
    const fn empty() -> Self {
        Self([const { GateDescriptor::new(0, 0) }; 256])
    }
    fn new() -> Self {
        let mut idt = [const { GateDescriptor::new(0, 0) }; 256];
        // Traps ===============================================================
        for i in 0..KERNEL_TRAPS.len() {
            idt[i] = GateDescriptor::new(
                KERNEL_TRAPS[i] as usize as u32,
                IDT_FLAG_P | IDT_FLAG_TYPE_TRAP32 | IDT_FLAG_DPL0,
            );
        }
        // Interrupt handlers ==================================================
        for i in 0..KERNEL_INTERRUPT_HANDLERS.len() {
            let trap_number = i + KERNEL_TRAPS.len();
            let dpl_flag = if trap_number == super::Hardware::SYSCALL_TRAP_NUMBER {
                IDT_FLAG_DPL3
            } else {
                IDT_FLAG_DPL0
            };
            idt[i + KERNEL_TRAPS.len()] = GateDescriptor::new(
                KERNEL_INTERRUPT_HANDLERS[i] as usize as u32,
                IDT_FLAG_P | IDT_FLAG_TYPE_INT32 | dpl_flag,
            );
        }
        Self(idt)
    }
}

#[unsafe(link_section = ".data.ro_after_early_init")]
static mut IDT: InterruptDescriptorTable = InterruptDescriptorTable::empty();

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u32,
}

pub(super) fn init() {
    unsafe {
        let idt = InterruptDescriptorTable::new();
        core::ptr::addr_of_mut!(IDT).write(idt);
    }
}

/// # Safety
/// `init` must have completed, and the IDT must remain at its current address.
pub(super) unsafe fn load() {
    let idtr = DescriptorTablePointer {
        limit: (size_of::<InterruptDescriptorTable>() - 1) as u16,
        base: core::ptr::addr_of!(IDT) as usize as u32,
    };
    let idtr_ptr = core::ptr::addr_of!(idtr);
    unsafe {
        core::arch::asm!(
            "lidt [{idtr}]",
            idtr = in(reg) idtr_ptr,
            options(readonly, nostack, preserves_flags),
        );
    }
}
