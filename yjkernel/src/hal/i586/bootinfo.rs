use alloc::vec;
use alloc::vec::Vec;

use crate::{
    klib::{self, hal::Hardware as _},
    println,
};

// https://www.gnu.org/software/grub/manual/multiboot/multiboot.html#Boot-information-format
#[repr(C)]
struct MultibootInfo {
    flags: u32,

    // present if flags[0] is set
    mem_lower: u32,
    mem_upper: u32,

    // present if flags[1] is set
    boot_device: u32,

    // present if flags[2] is set
    cmdline: u32,

    // present if flags[3] is set
    mods_count: u32,
    mods_addr: u32,

    // present if flags[4] or flags[5] is set
    syms: [u32; 4],

    // present if flags[6] is set
    mmap_length: u32,
    mmap_addr: u32,

    // present if flags[7] is set
    drives_length: u32,
    drives_addr: u32,

    // present if flags[8] is set
    config_table: u32,

    // present if flags[9] is set
    boot_loader_name: u32,

    // present if flags[10] is set
    apm_table: u32,

    // present if flags[11] is set
    vbe_control_info: u32,
    vbe_mode_info: u32,
    vbe_mode: u16,
    vbe_interface_seg: u16,
    vbe_interface_off: u16,
    vbe_interface_len: u16,

    // present if flags[12] is set
    framebuffer_addr: u64,
    framebuffer_pitch: u32,
    framebuffer_width: u32,
    framebuffer_height: u32,
    framebuffer_bpp: u8,
    framebuffer_type: u8, // 0: Indexed, 1: RGB, 2: EGA Text
    color_info: [u8; 6],
}
#[repr(C, packed(4))]
struct MultibootMmapEntry {
    size: u32,
    addr: u64,
    len: u64,
    typ: u32,
}
pub const MULTIBOOT_BOOTLOADER_MAGIC: u32 = 0x2BADB002;

const MULTIBOOT_MEMORY_AVAILABLE: u32 = 1;
const MULTIBOOT_MEMORY_RESERVED: u32 = 2;
const MULTIBOOT_MEMORY_ACPI_RECLAIMABLE: u32 = 3;
const MULTIBOOT_MEMORY_NVS: u32 = 4;
const MULTIBOOT_MEMORY_BADRAM: u32 = 5;

const MULTIBOOT_INFO_MEMORY: u32 = 1 << 0;
const MULTIBOOT_INFO_BOOTDEV: u32 = 1 << 1;
const MULTIBOOT_INFO_CMDLINE: u32 = 1 << 2;
const MULTIBOOT_INFO_MODS: u32 = 1 << 3;
const MULTIBOOT_INFO_AOUT_SYMS: u32 = 1 << 4;
const MULTIBOOT_INFO_ELF_SHDR: u32 = 1 << 5;
const MULTIBOOT_INFO_MEM_MAP: u32 = 1 << 6;
const MULTIBOOT_INFO_DRIVE_INFO: u32 = 1 << 7;
const MULTIBOOT_INFO_CONFIG_TABLE: u32 = 1 << 8;
const MULTIBOOT_INFO_BOOT_LOADER_NAME: u32 = 1 << 9;
const MULTIBOOT_INFO_APM_TABLE: u32 = 1 << 10;
const MULTIBOOT_INFO_VBE_INFO: u32 = 1 << 11;
const MULTIBOOT_INFO_FRAMEBUFFER_INFO: u32 = 1 << 12;

/// # Safety
/// The address must point to valid, aligned address provided by Multiboot compliant bootloader.
pub unsafe fn process(
    mb_info_ptr: klib::PhysicalPtr,
    pmm: &mut klib::mem::PhysicalMemoryManager<super::Hardware>,
) {
    let info = unsafe { mb_info_ptr.read::<super::Hardware, MultibootInfo>(false) };
    if (info.flags & MULTIBOOT_INFO_MEM_MAP) != 0 {
        print_mmap(&info);
        process_mmap(&info, pmm);
    } else {
        println!("[warning] no memory map from bootloader - no memory will be registered!");
    }
}

//==============================================================================
// Boot image handling
//==============================================================================

//==============================================================================
// Memory region handling
//==============================================================================

#[derive(Debug, PartialEq, Eq)]
enum MultibootMemoryType {
    Other,
    Available,
    ACPIReclaimable,
    Reserved,
    Nvs,
    BadRAM,
}
impl From<u32> for MultibootMemoryType {
    fn from(value: u32) -> Self {
        match value {
            MULTIBOOT_MEMORY_AVAILABLE => Self::Available,
            MULTIBOOT_MEMORY_RESERVED => Self::Reserved,
            MULTIBOOT_MEMORY_ACPI_RECLAIMABLE => Self::ACPIReclaimable,
            MULTIBOOT_MEMORY_NVS => Self::Nvs,
            MULTIBOOT_MEMORY_BADRAM => Self::BadRAM,
            _ => Self::Other,
        }
    }
}
struct MemRegion {
    base: klib::PhysicalPtr,
    len: usize,
}
enum AddrLimitResult {
    Empty,
    Ignore,
    Warn(MemRegion),
    OK(MemRegion),
}
fn limit_to_32bit_addr(addr: u64, len: u64) -> AddrLimitResult {
    if len == 0 {
        return AddrLimitResult::Empty;
    }
    let first_addr = addr;
    if (u32::MAX as u64) < first_addr {
        return AddrLimitResult::Ignore;
    }
    let last_addr = addr.saturating_add(len - 1);
    let outside_32bit = (u32::MAX as u64) < last_addr;
    let last_addr = last_addr.min(u32::MAX as u64);
    let region_len = last_addr - first_addr + 1;
    let region = MemRegion {
        base: klib::PhysicalPtr::new(first_addr as usize),
        len: usize::try_from(region_len).unwrap_or(usize::MAX),
    };
    if outside_32bit {
        return AddrLimitResult::Warn(region);
    }
    AddrLimitResult::OK(region)
}
fn human_readable_len(len: usize) -> (usize, &'static str) {
    let units = ["B", "K", "M", "G", "T"];
    let mut result_len = len;
    let mut idx = 0;
    while 1024 <= result_len {
        if (units.len() - 1) <= idx {
            break;
        }
        idx += 1;
        result_len /= 1024;
    }
    (result_len, units[idx])
}
struct MmapEntry {
    base: klib::PhysicalPtr,
    len: usize,
    typ: MultibootMemoryType,
}
struct MmapIter<'a> {
    info: &'a MultibootInfo,
    entry_addr: klib::PhysicalPtr,
    read_len: usize,
    warn_too_much_mem: bool,
}
impl<'a> MmapIter<'a> {
    fn new(info: &'a MultibootInfo) -> Self {
        let mmap_addr = klib::PhysicalPtr::new(info.mmap_addr as usize);
        Self {
            info,
            entry_addr: mmap_addr,
            read_len: 0,
            warn_too_much_mem: false,
        }
    }
}
impl<'a> Iterator for MmapIter<'a> {
    type Item = MmapEntry;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let mmap_length = self.info.mmap_length as usize;
            if self.read_len == mmap_length {
                return None;
            }
            let remaining = mmap_length
                .checked_sub(self.read_len)
                .expect("Multiboot memory-map length exceeded its buffer");
            assert!(
                remaining >= size_of::<u32>(),
                "truncated Multiboot memory-map entry header"
            );
            let entry_size =
                unsafe { self.entry_addr.read::<super::Hardware, u32>(false) } as usize;
            let minimum_entry_size = size_of::<MultibootMmapEntry>() - size_of::<u32>();
            assert!(
                entry_size >= minimum_entry_size,
                "Multiboot memory-map entry is too short"
            );
            let total_size = size_of::<u32>()
                .checked_add(entry_size)
                .expect("Multiboot memory-map entry size overflow");
            assert!(
                total_size <= remaining,
                "Multiboot memory-map entry exceeds its buffer"
            );
            let entry = unsafe {
                self.entry_addr
                    .read::<super::Hardware, MultibootMmapEntry>(false)
            };
            self.read_len = self
                .read_len
                .checked_add(total_size)
                .expect("Multiboot memory-map length overflow");
            self.entry_addr = self
                .entry_addr
                .checked_add(total_size)
                .expect("Multiboot memory-map address overflow");

            let typ = entry.typ;
            let (base, len) = match limit_to_32bit_addr(entry.addr, entry.len) {
                AddrLimitResult::Empty => continue,
                AddrLimitResult::Ignore => {
                    if typ == MULTIBOOT_MEMORY_AVAILABLE {
                        self.warn_too_much_mem = true;
                    }
                    continue;
                }
                AddrLimitResult::Warn(region) => {
                    if typ == MULTIBOOT_MEMORY_AVAILABLE {
                        self.warn_too_much_mem = true;
                    }
                    (region.base, region.len)
                }
                AddrLimitResult::OK(region) => (region.base, region.len),
            };
            let typ = MultibootMemoryType::from(typ);
            break Some(MmapEntry { base, len, typ });
        }
    }
}
fn print_mmap(info: &MultibootInfo) {
    println!("----------------- Memory map -----------------");
    println!("fromaddr  toaddr   length  type");
    let mut entries = MmapIter::new(info);
    for entry in entries.by_ref() {
        let typ = entry.typ;
        let (display_len, len_unit) = human_readable_len(entry.len);
        println!(
            "{:08x}  {:08x}  {:4}{}  {:#?}",
            entry.base.addr(),
            unsafe { entry.base.add(entry.len - 1) }.addr(),
            display_len,
            len_unit,
            typ
        );
    }
    println!("----------------------------------------------");
    if entries.warn_too_much_mem {
        println!("the system has more memory, but outside of 32-bit address space");
    }
}
fn exclude_region(
    addr: klib::PhysicalPtr,
    len: usize,
    exclude_addr: klib::PhysicalPtr,
    exclude_len: usize,
) -> (MemRegion, MemRegion) {
    let start = addr;
    let end = start
        .checked_add(len)
        .unwrap_or_else(|| klib::PhysicalPtr::new(usize::MAX));
    let exclude_start = exclude_addr;
    let exclude_end = exclude_start
        .checked_add(exclude_len)
        .unwrap_or_else(|| klib::PhysicalPtr::new(usize::MAX));
    if (end <= exclude_start) || (exclude_end <= start) {
        return (
            MemRegion { base: start, len },
            MemRegion {
                base: klib::PhysicalPtr::new(0),
                len: 0,
            },
        );
    }
    let before_start = start;
    let before_end = exclude_start;
    let after_start = exclude_end;
    let after_end = end;

    let mut before = MemRegion {
        base: klib::PhysicalPtr::new(0),
        len: 0,
    };
    let mut after = MemRegion {
        base: klib::PhysicalPtr::new(0),
        len: 0,
    };
    if before_start < before_end {
        before.base = before_start;
        before.len = before_end
            .checked_distance_from(before_start)
            .expect("physical address order changed");
    }
    if after_start < after_end {
        after.base = after_start;
        after.len = after_end
            .checked_distance_from(after_start)
            .expect("physical address order changed");
    }

    (before, after)
}
fn exclude_regions(
    regions_to_exclude: &[MemRegion],
    base: klib::PhysicalPtr,
    len: usize,
) -> Vec<MemRegion> {
    // Excluding each region can result 0 or 1 additional entries.
    // So after processing one exclusion entry, the list can grow by 2x and that list becomes input when processing next exclusion entry.
    // So the maximum size for resulting list would be 2^EXCLUDE_COUNT.

    let mut result_regions = Vec::with_capacity(1 << regions_to_exclude.len());
    result_regions.push(MemRegion { base, len });
    for i in 0..regions_to_exclude.len() {
        let old_regions_count = result_regions.len();
        for j in 0..old_regions_count {
            if result_regions[j].len == 0 {
                continue;
            }
            let (r1, r2) = exclude_region(
                result_regions[j].base,
                result_regions[j].len,
                regions_to_exclude[i].base,
                regions_to_exclude[i].len,
            );
            if (r1.len != 0) && (r2.len != 0) {
                // Both r1 and r2 exists -> Overwrite original region with r1, copy r2 to new region entry.
                result_regions[j].base = r1.base;
                result_regions[j].len = r1.len;
                result_regions.push(r2);
            } else if r1.len != 0 {
                // Only r1 exists -> Overwrite original region with r1
                result_regions[j].base = r1.base;
                result_regions[j].len = r1.len;
            } else if r2.len != 0 {
                // Only r2 exists -> Overwrite original region with r2
                result_regions[j].base = r2.base;
                result_regions[j].len = r2.len;
            } else {
                /* No region */
                result_regions[j].base = klib::PhysicalPtr::new(0);
                result_regions[j].len = 0;
            }
        }
    }
    result_regions
}
fn register_region(
    mut base: klib::PhysicalPtr,
    mut len: usize,
    pmm: &mut klib::mem::PhysicalMemoryManager<super::Hardware>,
) -> Result<(), &'static str> {
    let offset = base.align_offset(super::Hardware::PAGE_SIZE);
    if offset != 0 {
        let incr = super::Hardware::PAGE_SIZE - offset;
        if len <= incr {
            return Ok(());
        }
        base = base
            .checked_add(incr)
            .ok_or("physical address range overflow")?;
        len = len
            .checked_sub(incr)
            .ok_or("physical address range underflow")?;
    }
    let page_count = len / super::Hardware::PAGE_SIZE;
    if page_count == 0 {
        return Ok(());
    }
    println!(
        "register memory: {:08x} ~ {:08x} ({} pages)",
        base.addr(),
        base.addr() + len - 1,
        page_count,
    );
    unsafe { pmm.register_mem(base, page_count) }
}
fn process_mmap(info: &MultibootInfo, pmm: &mut klib::mem::PhysicalMemoryManager<super::Hardware>) {
    let regions_to_exclude = vec![
        MemRegion {
            base: klib::PhysicalPtr::new(0),
            len: 0x100000,
        },
        MemRegion {
            base: klib::PhysicalPtr::new(super::sections::kernel_physical_address_start()),
            len: super::sections::kernel_physical_address_end()
                - super::sections::kernel_physical_address_start(),
        },
    ];

    for entry in MmapIter::new(info) {
        if entry.typ != MultibootMemoryType::Available {
            continue;
        }
        for region in exclude_regions(&regions_to_exclude, entry.base, entry.len) {
            let base = region.base;
            let len = region.len;
            if let Err(e) = register_region(base, len, pmm) {
                println!("[warning] could not register RAM region: {}", e)
            }
        }
    }
}
