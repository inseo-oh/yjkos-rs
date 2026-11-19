use core::mem::{offset_of, size_of};

#[repr(C, align(4))]
struct TaskStateSegment {
    link: u16,
    _reserved0: u16,
    esp0: u32,
    ss0: u16,
    _reserved1: u16,
    esp1: u32,
    ss1: u16,
    _reserved2: u16,
    esp2: u32,
    ss2: u16,
    _reserved3: u16,
    cr3: u32,
    eip: u32,
    eflags: u32,
    eax: u32,
    ecx: u32,
    edx: u32,
    ebx: u32,
    esp: u32,
    ebp: u32,
    esi: u32,
    edi: u32,
    es: u16,
    _reserved4: u16,
    cs: u16,
    _reserved5: u16,
    ss: u16,
    _reserved6: u16,
    ds: u16,
    _reserved7: u16,
    fs: u16,
    _reserved8: u16,
    gs: u16,
    _reserved9: u16,
    ldtr: u16,
    _reserved10: u16,
    _reserved11: u16,
    iopb: u16,
    ssp: u32,
}

const _: () = assert!(size_of::<TaskStateSegment>() == 108);

impl TaskStateSegment {
    const fn zeroed() -> Self {
        Self {
            link: 0,
            _reserved0: 0,
            esp0: 0,
            ss0: 0,
            _reserved1: 0,
            esp1: 0,
            ss1: 0,
            _reserved2: 0,
            esp2: 0,
            ss2: 0,
            _reserved3: 0,
            cr3: 0,
            eip: 0,
            eflags: 0,
            eax: 0,
            ecx: 0,
            edx: 0,
            ebx: 0,
            esp: 0,
            ebp: 0,
            esi: 0,
            edi: 0,
            es: 0,
            _reserved4: 0,
            cs: 0,
            _reserved5: 0,
            ss: 0,
            _reserved6: 0,
            ds: 0,
            _reserved7: 0,
            fs: 0,
            _reserved8: 0,
            gs: 0,
            _reserved9: 0,
            ldtr: 0,
            _reserved10: 0,
            _reserved11: 0,
            iopb: 0,
            ssp: 0,
        }
    }

    fn for_kernel(esp0: u32) -> Self {
        Self {
            esp0,
            ss0: GDT_KERNEL_DS,
            iopb: size_of::<Self>() as u16,
            ..Self::zeroed()
        }
    }
}

#[repr(C, align(16))]
struct ESP0Stack([u8; 4096]);

static mut TSS: TaskStateSegment = TaskStateSegment::zeroed();
static mut ESP0_STACK: ESP0Stack = ESP0Stack([0; 4096]);

const GDT_FLAG_L: u8 = 1 << 1;
const GDT_FLAG_DB: u8 = 1 << 2;
const GDT_FLAG_G: u8 = 1 << 3;
const GDT_ACCESS_FLAG_S: u8 = 1 << 4; // Clear -> System segment descriptor
const fn gdt_access_flag_dpl(n: u8) -> u8 {
    n << 5
}
const GDT_ACCESS_FLAG_DPL0: u8 = gdt_access_flag_dpl(0);
const GDT_ACCESS_FLAG_DPL1: u8 = gdt_access_flag_dpl(1);
const GDT_ACCESS_FLAG_DPL2: u8 = gdt_access_flag_dpl(2);
const GDT_ACCESS_FLAG_DPL3: u8 = gdt_access_flag_dpl(3);
const GDT_ACCESS_FLAG_P: u8 = 1 << 7;

/* Below applies to non-system segment descriptors */
const GDT_ACCESS_FLAG_ACCESSED: u8 = 1 << 0;
const GDT_ACCESS_FLAG_RW: u8 = 1 << 1; // Data segments: Writable bit, Code segments: Readable bit
const GDT_ACCESS_FLAG_DC: u8 = 1 << 2;
const GDT_ACCESS_FLAG_E: u8 = 1 << 3;

/* Below applies to system segment descriptors */
const GDT_ACCESS_FLAG_TYPE_LDT: u8 = 0x2;
const GDT_ACCESS_FLAG_TYPE_TSS32_AVL: u8 = 0x9;
const GDT_ACCESS_FLAG_TYPE_BUSY: u8 = 0xb;

#[repr(C, align(4))]
struct SegmentDescriptor {
    limit_b15_to_b0: u16,
    base_b15_to_b0: u16,
    base_b23_to_b16: u8,
    access_byte: u8,
    limit_b19_to_b16_and_flags: u8,
    base_b31_to_b24: u8,
}

impl SegmentDescriptor {
    const fn null() -> Self {
        Self {
            limit_b15_to_b0: 0,
            base_b15_to_b0: 0,
            base_b23_to_b16: 0,
            access_byte: 0,
            limit_b19_to_b16_and_flags: 0,
            base_b31_to_b24: 0,
        }
    }
    const fn new(base: u32, limit: u32, flags: u8, access_byte: u8) -> Self {
        Self {
            limit_b15_to_b0: (limit & 0xffff) as u16,
            base_b15_to_b0: (base & 0xffff) as u16,
            base_b23_to_b16: ((base >> 16) & 0xffff) as u8,
            access_byte,
            limit_b19_to_b16_and_flags: ((flags & 0xf) << 4) | ((limit >> 16) & 0xf) as u8,
            base_b31_to_b24: ((base >> 24) & 0xff) as u8,
        }
    }
}
const _: () = assert!(size_of::<SegmentDescriptor>() == 8);

#[repr(C, align(4))]
struct GlobalDescriptorTable {
    null_descriptor: SegmentDescriptor,
    kernel_code: SegmentDescriptor,
    kernel_data: SegmentDescriptor,
    user_code: SegmentDescriptor,
    user_data: SegmentDescriptor,
    tss: SegmentDescriptor,
}
impl GlobalDescriptorTable {
    const fn empty() -> Self {
        Self {
            null_descriptor: SegmentDescriptor::null(),
            kernel_code: SegmentDescriptor::null(),
            kernel_data: SegmentDescriptor::null(),
            user_code: SegmentDescriptor::null(),
            user_data: SegmentDescriptor::null(),
            tss: SegmentDescriptor::null(),
        }
    }

    fn new(tss_base: u32) -> Self {
        Self {
            null_descriptor: SegmentDescriptor::null(),
            kernel_code: SegmentDescriptor::new(
                0,
                0xfffff,
                GDT_FLAG_G | GDT_FLAG_DB,
                GDT_ACCESS_FLAG_P
                    | GDT_ACCESS_FLAG_S
                    | GDT_ACCESS_FLAG_RW
                    | GDT_ACCESS_FLAG_DPL0
                    | GDT_ACCESS_FLAG_E
                    | GDT_ACCESS_FLAG_ACCESSED,
            ),
            kernel_data: SegmentDescriptor::new(
                0,
                0xfffff,
                GDT_FLAG_G | GDT_FLAG_DB,
                GDT_ACCESS_FLAG_P
                    | GDT_ACCESS_FLAG_S
                    | GDT_ACCESS_FLAG_RW
                    | GDT_ACCESS_FLAG_DPL0
                    | GDT_ACCESS_FLAG_ACCESSED,
            ),
            user_code: SegmentDescriptor::new(
                0,
                0xfffff,
                GDT_FLAG_G | GDT_FLAG_DB,
                GDT_ACCESS_FLAG_P
                    | GDT_ACCESS_FLAG_S
                    | GDT_ACCESS_FLAG_RW
                    | GDT_ACCESS_FLAG_DPL3
                    | GDT_ACCESS_FLAG_E
                    | GDT_ACCESS_FLAG_ACCESSED,
            ),
            user_data: SegmentDescriptor::new(
                0,
                0xfffff,
                GDT_FLAG_G | GDT_FLAG_DB,
                GDT_ACCESS_FLAG_P
                    | GDT_ACCESS_FLAG_S
                    | GDT_ACCESS_FLAG_RW
                    | GDT_ACCESS_FLAG_DPL3
                    | GDT_ACCESS_FLAG_ACCESSED,
            ),
            tss: SegmentDescriptor::new(
                tss_base,
                size_of::<TaskStateSegment>() as u32 - 1,
                GDT_FLAG_DB, // TSS size is expressed as bytes, so we don't use G flag
                GDT_ACCESS_FLAG_P | GDT_ACCESS_FLAG_DPL0 | GDT_ACCESS_FLAG_TYPE_TSS32_AVL,
            ),
        }
    }
}

static mut GDT: GlobalDescriptorTable = GlobalDescriptorTable::empty();

#[repr(C, packed)]
struct DescriptorTablePointer {
    limit: u16,
    base: u32,
}

pub(super) fn init() {
    let stack_top = unsafe {
        core::ptr::addr_of_mut!(ESP0_STACK.0)
            .cast::<u8>()
            .add(size_of::<[u8; 4096]>())
    };

    // An I/O-map base at the end of the TSS disables the I/O bitmap.
    let tss = TaskStateSegment::for_kernel(stack_top as usize as u32);

    unsafe {
        let tss_ptr = core::ptr::addr_of_mut!(TSS);
        tss_ptr.write(tss);
        let gdt = GlobalDescriptorTable::new(tss_ptr as usize as u32);
        core::ptr::addr_of_mut!(GDT).write(gdt);
    }
}

/// # Safety
/// `init` must have completed, and the GDT must remain at its current address.
pub(super) unsafe fn load() {
    let gdtr = DescriptorTablePointer {
        limit: (size_of::<GlobalDescriptorTable>() - 1) as u16,
        base: core::ptr::addr_of!(GDT) as usize as u32,
    };
    let gdtr_ptr = core::ptr::addr_of!(gdtr);
    unsafe {
        core::arch::asm!(
            "lgdt [{gdtr}]",
            gdtr = in(reg) gdtr_ptr,
            options(readonly, nostack, preserves_flags),
        );
    }
}

/// # Safety
/// `load` must have completed, and the GDT descriptors at the kernel and TSS
/// selectors must be valid.
pub(super) unsafe fn reload_selectors() {
    unsafe {
        core::arch::asm!(
            "lea eax, [2f]",
            "push {cs:e}",
            "push eax",
            "retf",
            "2:",
            "mov ds, {ds:x}",
            "mov es, {ds:x}",
            "mov fs, {ds:x}",
            "mov gs, {ds:x}",
            "mov ss, {ds:x}",
            "ltr {tss:x}",
            cs = in(reg) GDT_KERNEL_CS as u32,
            ds = in(reg) GDT_KERNEL_DS,
            tss = in(reg) GDT_TSS,
            out("eax") _,
            options(preserves_flags),
        );
    }
}

pub const GDT_KERNEL_CS: u16 = offset_of!(GlobalDescriptorTable, kernel_code) as u16;
pub const GDT_KERNEL_DS: u16 = offset_of!(GlobalDescriptorTable, kernel_data) as u16;
pub const GDT_USER_CS: u16 = offset_of!(GlobalDescriptorTable, user_code) as u16 | 3;
pub const GDT_USER_DS: u16 = offset_of!(GlobalDescriptorTable, user_data) as u16 | 3;
pub const GDT_TSS: u16 = offset_of!(GlobalDescriptorTable, tss) as u16;
