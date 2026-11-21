use alloc::vec::Vec;

use crate::{klib, println};
use core::{ops::Range, slice};

pub const USER_MODE_BASE: usize = 0x8000_0000;

pub fn load<HW: klib::hal::Hardware>(
    program: &[u8],
    address_space: &klib::mem::vmm::AddressSpaceHandle<HW>,
    vmm: &klib::irq::Mutex<HW, klib::mem::vmm::VirtualMemoryManager<HW>>,
) -> Result<usize, &'static str> {
    let file = elf::ElfBytes::<elf::endian::LittleEndian>::minimal_parse(program).map_err(|e| {
        println!("failed to parse ELF file: {:?}", e);
        "Invalid ELF executable: parse error"
    })?;
    let entry = file.ehdr.e_entry;
    let segments = file
        .segments()
        .ok_or("Invalid ELF executable: Segments could not be found")?;

    struct AllocRange {
        range: Range<usize>, // Page number
        flags: klib::mem::vmm::MapFlags,
    }

    let mut alloc_page_ranges = Vec::<AllocRange>::new();

    // Collect PT_LOAD segments ===================================================
    for phdr in segments {
        if phdr.p_type != elf::abi::PT_LOAD || phdr.p_memsz == 0 {
            continue;
        }
        if phdr.p_memsz < phdr.p_filesz {
            return Err("Invalid ELF executable: p_filesz is larger then p_memsz");
        }
        let start = usize::try_from(phdr.p_vaddr).map_err(|_| "ELF: invalid virtual address")?;
        let memsz = usize::try_from(phdr.p_memsz).map_err(|_| "ELF: invalid memory size")?;
        let end = start
            .checked_add(memsz)
            .ok_or("Invalid ELF executable: p_memsz is too large")?;
        let page_start = start / HW::PAGE_SIZE;
        let page_end = end / HW::PAGE_SIZE + usize::from(end % HW::PAGE_SIZE != 0);
        let mut flags = klib::mem::vmm::MapFlags::default();
        flags.read = (phdr.p_flags & elf::abi::PF_R) != 0;
        flags.write = (phdr.p_flags & elf::abi::PF_W) != 0;
        flags.exec = (phdr.p_flags & elf::abi::PF_X) != 0;
        alloc_page_ranges.push(AllocRange {
            range: page_start..page_end,
            flags,
        });
    }

    // Collect memory boundaries ==================================================
    let mut boundaries = Vec::<usize>::new();
    for entry in &alloc_page_ranges {
        boundaries.push(entry.range.start);
        boundaries.push(entry.range.end);
    }
    boundaries.sort_unstable();
    boundaries.dedup();

    // Merge regions ===========================================================
    let mut merged = Vec::<AllocRange>::new();
    for pair in boundaries.windows(2) {
        let start = pair[0];
        let end = pair[1];

        let mut flags = klib::mem::vmm::MapFlags::default();
        let mut covered = false;

        for entry in &alloc_page_ranges {
            if entry.range.start <= start && end <= entry.range.end {
                covered = true;
                flags.read |= entry.flags.read;
                flags.write |= entry.flags.write;
                flags.exec |= entry.flags.exec;
            }
        }
        if !covered {
            continue;
        }
        // If the range is contiguous AND have same permission, merge two ranges.
        if let Some(last) = merged.last_mut()
            && last.range.end == start
            && last.flags.read == flags.read
            && last.flags.write == flags.write
            && last.flags.exec == flags.exec
        {
            last.range.end = end;
            continue;
        }
        merged.push(AllocRange {
            range: start..end,
            flags,
        });
    }

    // Allocate VM objects =====================================================
    let mut objects = Vec::new();
    for entry in merged {
        let address = entry
            .range
            .start
            .checked_mul(HW::PAGE_SIZE)
            .ok_or("Invalid ELF executable: start address is too large")?;
        let alloc_size = (entry.range.end - entry.range.start)
            .checked_mul(HW::PAGE_SIZE)
            .ok_or("Invalid ELF executable: allocation size is too large")?;
        let object = klib::mem::vmm::alloc_at(
            address_space,
            address as *mut u8,
            alloc_size,
            klib::mem::vmm::MapFlags {
                write: true, // Temporaily force-enable writes
                ..entry.flags
            },
        )?;
        objects.push((object, entry));
    }

    let is_user = address_space.borrow().is_user();
    klib::mem::vmm::AddressSpace::with_active(address_space, vmm, || {
        // Finally, we load segments ===========================================
        for phdr in segments {
            if phdr.p_type != elf::abi::PT_LOAD || phdr.p_memsz == 0 {
                continue;
            }
            let dest = unsafe {
                slice::from_raw_parts_mut(phdr.p_vaddr as *mut u8, phdr.p_memsz as usize)
            };
            let offset = usize::try_from(phdr.p_offset)
                .map_err(|_| "Invalid ELF executable: invalid offset")?;
            let filesz = usize::try_from(phdr.p_filesz)
                .map_err(|_| "Invalid ELF executable: invalid file size")?;
            let end = offset
                .checked_add(filesz)
                .ok_or("Invalid ELF executable: file range overflow")?;
            let src = program
                .get(offset..end)
                .ok_or("Invalid ELF executable: segment exceeds file size")?;
            dest[..filesz].copy_from_slice(src);
            dest[filesz..].fill(0);
        }

        // And protect pages as needed =========================================
        for (object, entry) in objects {
            HW::mmu_remap(
                object.borrow().start,
                object.borrow().size / HW::PAGE_SIZE,
                entry.flags,
                is_user,
            )?;
        }
        Ok(())
    })?;

    Ok(entry as usize)
}
