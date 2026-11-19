use crate::klib;
use alloc::{rc::Rc, vec::Vec};
use core::{cell::RefCell, ptr::null_mut};

pub struct VirtualMemoryManager<HW> {
    kernel_address_space: Option<AddressSpaceHandle<HW>>,
}

impl<HW: klib::hal::Hardware> VirtualMemoryManager<HW> {
    pub const fn new() -> Self {
        Self {
            kernel_address_space: None,
        }
    }
    fn real_page_fault(&self, trap_frame: &HW::TrapFrame) {
        // TODO: Port this to Rust: co_printf("attempted to %s on non-present page at %p\n", was_write ? "read" : "write", ptr);
        HW::stack_trace_for_trap_frame(trap_frame);
        panic!("fatal memory access fault");
    }
    fn page_not_present(&self, trap_frame: &HW::TrapFrame) {
        // TODO: Port this to Rust: co_printf("attempted to %s on non-present page at %p\n", was_write ? "read" : "write", ptr);
        VirtualMemoryManager::<HW>::real_page_fault(self, trap_frame);
    }
    pub fn kernel_address_space(&mut self) -> AddressSpaceHandle<HW> {
        let kernel_start = HW::kernel_vm_start_address();
        let kernel_size = HW::kernel_vm_end_address()
            .checked_sub(kernel_start)
            .and_then(|size| size.checked_add(1))
            .expect("invalid kernel virtual memory range");
        self.kernel_address_space
            .get_or_insert_with(|| AddressSpace::new(kernel_start as *mut u8, kernel_size, false))
            .clone()
    }
    pub(in crate::klib) fn existing_kernel_address_space(&self) -> Option<AddressSpaceHandle<HW>> {
        self.kernel_address_space.clone()
    }
    pub fn address_space_of(&mut self, ptr: *mut u8) -> Option<AddressSpaceHandle<HW>> {
        if (HW::kernel_vm_start_address() <= ptr.addr())
            && (ptr.addr() <= HW::kernel_vm_end_address())
        {
            // Kernel VM
            return Some(self.kernel_address_space());
        }
        if ptr.addr() < HW::KERNEL_SPACE_BASE_ADDRESS {
            // Userspace address (TODO)
            return None;
        }
        // Kernel image, scratch page, etc...
        None
    }
    pub fn ezmap(&mut self, phys_base: klib::PhysicalPtr, size: usize) -> *mut u8 {
        let (_, addr) = map_unaligned(
            &self.kernel_address_space(),
            phys_base,
            size,
            MapFlags {
                read: true,
                write: true,
                ..Default::default()
            },
        )
        .expect("failed to ezmap virtual memory");
        addr
    }
    pub fn page_fault(
        &mut self,
        mut alloc_page: impl FnMut(usize) -> Result<klib::mem::pmm::AllocatedPageInfo, &'static str>,
        ptr: *mut u8,
        was_present: bool,
        was_write: bool,
        was_user: bool,
        trap_frame: &HW::TrapFrame,
    ) {
        let page_base = klib::miscmath::align_ptr_down(ptr, HW::PAGE_SIZE);
        // Is it valid address? ================================================
        if HW::mmu_emulate(
            ptr,
            klib::mem::vmm::MapFlags {
                read: true,
                write: was_write,
                ..Default::default()
            },
            was_user,
        )
        .is_ok()
        {
            // It's a valid access, but TLB was out of sync.
            HW::mmu_flush_tlb_for(ptr);
            return;
        }
        if was_present {
            // TODO: Port this to Rust: co_printf("privilege violation: attempted to %s on page at %p\n", was_write ? "read" : "write", ptr);
            self.real_page_fault(trap_frame);
            return;
        }
        if page_base.is_null() {
            // TODO: Port this to Rust: co_printf("nullptr dereference: attempted to %s on page at %p\n", was_write ? "read" : "write", ptr);
            self.real_page_fault(trap_frame);
            return;
        }

        // See if it's an uncommitted object ===================================
        let address_space = if let Some(address_space) = self.address_space_of(page_base) {
            address_space
        } else {
            // We don't know what this address is.
            self.page_not_present(trap_frame);
            return;
        };

        let uobject: UncommittedObjectHandle<HW> = if let Some(uobject) = address_space
            .borrow()
            .find_object_in_uncommitted(ptr.addr())
        {
            uobject
        } else {
            // Not part of an uncommitted object.
            self.page_not_present(trap_frame);
            return;
        };
        let page_index =
            (page_base.addr() - uobject.borrow().object.borrow().start_addr()) / HW::PAGE_SIZE;
        // It is an uncommitted object =========================================
        {
            let mut uobject = uobject.borrow_mut();
            if !uobject.is_page_uncommitted(page_index) {
                // Already committed page...?
                panic!("non-present page on already committed page");
            }
            uobject.mark_page_committed(page_index);
        }
        let page_count = 1;
        let phys_addr = if let Some(phys_base) = uobject.borrow().object.borrow().phys_base {
            let page_offset = page_index
                .checked_mul(HW::PAGE_SIZE)
                .expect("physical page offset overflow");
            phys_base
                .checked_add(page_offset)
                .expect("physical address overflow")
        } else {
            // TODO: Run the OOM killer
            let alloc =
                alloc_page(page_count).expect("ran out of memory while trying to commit the page");
            alloc.base
        };
        HW::mmu_map(
            page_base,
            phys_addr,
            1,
            uobject.borrow().object.borrow().flags,
            uobject
                .borrow()
                .object
                .borrow()
                .address_space
                .borrow()
                .is_user,
        )
        .expect("failed to map allocated memory");
        if !uobject.borrow_mut().has_uncommitted_pages() {
            let uobject_v = uobject.borrow_mut();
            let object = uobject_v.object.borrow_mut();
            let mut address_space = object.address_space.borrow_mut();
            let pos = address_space
                .uncommitted_objects
                .iter()
                .position(|u| Rc::ptr_eq(&uobject, u))
                .unwrap();
            address_space.uncommitted_objects.remove(pos);
        }
    }
}

impl<HW: klib::hal::Hardware> Default for VirtualMemoryManager<HW> {
    fn default() -> Self {
        Self::new()
    }
}

pub fn alloc_object<HW: klib::hal::Hardware>(
    address_space: &AddressSpaceHandle<HW>,
    phys_base: Option<klib::PhysicalPtr>,
    size: usize,
    flags: MapFlags,
) -> Result<ObjectHandle<HW>, &'static str> {
    assert_ne!(size, 0);
    let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
    let page_count = klib::miscmath::size_to_blocks(size, HW::PAGE_SIZE);
    let new_size = page_count
        .checked_mul(HW::PAGE_SIZE)
        .ok_or("size too large")?;
    if let Some(phys_base) = phys_base {
        assert_eq!(phys_base.align_offset(HW::PAGE_SIZE), 0);
        phys_base
            .checked_add(new_size - 1)
            .ok_or("physical address range overflow")?;
    }

    // XXX: This was behavior copied from YJKOS C version. Not entirely sure if we still need it:
    // > We create objects first with dummy values, so that we just have to free those objects on failure.
    // > (It's a good idea to avoid undoing take_object~, because that undo operation can technically also fail)
    let new_object = create_object(address_space, null_mut(), 0, phys_base, flags);
    let uobject = create_uncommitted_object(&new_object, page_count);
    let old_object = address_space
        .borrow_mut()
        .take_object_with_min_size(page_count)
        .ok_or("out of memory")?;
    new_object.borrow_mut().start = old_object.borrow().start;
    new_object.borrow_mut().size = new_size;
    // Try to shrink the existing object.
    let shrink_result = old_object.borrow_mut().shrink_at_start(new_size);
    if shrink_result.is_ok() && old_object.borrow().size != 0 {
        // If we can(and resulting size is non-zero), add it back to list (otherwise we drop this object).
        address_space.borrow_mut().add_object(&old_object);
    }
    address_space.borrow_mut().uncommitted_objects.push(uobject);

    Ok(new_object)
}
pub fn alloc_object_at<HW: klib::hal::Hardware>(
    address_space: &AddressSpaceHandle<HW>,
    virt_base: *mut u8,
    phys_base: Option<klib::PhysicalPtr>,
    size: usize,
    flags: MapFlags,
) -> Result<ObjectHandle<HW>, &'static str> {
    assert_ne!(size, 0);
    let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
    let page_count = klib::miscmath::size_to_blocks(size, HW::PAGE_SIZE);
    let new_size = page_count
        .checked_mul(HW::PAGE_SIZE)
        .ok_or("size too large")?;
    if let Some(phys_base) = phys_base {
        assert_eq!(phys_base.align_offset(HW::PAGE_SIZE), 0);
    }
    assert!(klib::miscmath::is_aligned(virt_base.addr(), HW::PAGE_SIZE));
    let end_addr = virt_base
        .addr()
        .checked_add(new_size - 1)
        .ok_or("virtual address range overflow")?;
    if let Some(phys_base) = phys_base {
        phys_base
            .checked_add(new_size - 1)
            .ok_or("physical address range overflow")?;
    }
    // XXX: This was behavior copied from YJKOS C version. Not entirely sure if we still need it:
    // > We create objects first with dummy values, so that we just have to free those objects on failure.
    // > (It's a good idea to avoid undoing take_object~, because that undo operation can technically also fail)
    let new_object = create_object(address_space, null_mut(), 0, phys_base, flags);
    let r_object = create_object(address_space, null_mut(), 0, None, Default::default());
    let uobject = create_uncommitted_object(&new_object, page_count);
    let l_object = address_space
        .borrow_mut()
        .take_object_including(virt_base.addr(), end_addr)
        .ok_or("out of memory")?;
    let old_end = l_object.borrow().end_addr();
    let old_start = l_object.borrow().start_addr();
    new_object.borrow_mut().start = virt_base;
    new_object.borrow_mut().size = new_size;

    // Split into two objects
    if old_start < virt_base.addr() {
        let left_end = virt_base
            .addr()
            .checked_sub(1)
            .expect("left split end address underflow");
        l_object
            .borrow_mut()
            .set_end(left_end as *mut u8)
            .expect("invalid left split range");
        address_space.borrow_mut().add_object(&l_object);
    }
    if end_addr < old_end {
        let right_start = end_addr
            .checked_add(1)
            .expect("right split start address overflow");
        r_object.borrow_mut().start = right_start as *mut u8;
        r_object.borrow_mut().flags = l_object.borrow().flags;
        r_object
            .borrow_mut()
            .set_end(old_end as *mut u8)
            .expect("invalid right split range");
        address_space.borrow_mut().add_object(&r_object);
    }
    address_space.borrow_mut().uncommitted_objects.push(uobject);
    Ok(new_object)
}
pub fn free<HW: klib::hal::Hardware>(
    object: &ObjectHandle<HW>,
    pmm: &mut klib::mem::PhysicalMemoryManager<HW>,
) {
    let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
    // Remove from uncommitted memory list =====================================
    let uobject = object
        .borrow()
        .address_space
        .borrow()
        .find_object_in_uncommitted(object.borrow().start_addr());
    if let Some(uobject) = uobject {
        let pos = object
            .borrow()
            .address_space
            .borrow()
            .uncommitted_objects
            .iter()
            .position(|p| Rc::ptr_eq(p, &uobject))
            .unwrap();
        object
            .borrow()
            .address_space
            .borrow_mut()
            .uncommitted_objects
            .remove(pos);
    }
    // Free committed physical pages and unmap it ==============================
    let mut ptr = object.borrow().start;
    while ptr.addr() <= object.borrow().end_addr() {
        if let Ok(phys_addr) = HW::mmu_virtual_to_physical(ptr) {
            if object.borrow().phys_base.is_none() {
                unsafe {
                    pmm.free(klib::mem::pmm::AllocatedPageInfo {
                        base: phys_addr,
                        page_count: 1,
                    })
                };
            }
            HW::mmu_unmap(ptr, 1).expect("failed to unmap committed pages");
        }
        ptr = unsafe { ptr.add(HW::PAGE_SIZE) };
    }
    /* Return object back to the tree */
    object
        .borrow()
        .address_space
        .borrow_mut()
        .add_object(object);
}
pub fn alloc_at<HW: klib::hal::Hardware>(
    address_space: &AddressSpaceHandle<HW>,
    virt_base: *mut u8,
    size: usize,
    flags: MapFlags,
) -> Result<ObjectHandle<HW>, &'static str> {
    alloc_object_at(address_space, virt_base, None, size, flags)
}
pub fn alloc<HW: klib::hal::Hardware>(
    address_space: &AddressSpaceHandle<HW>,
    size: usize,
    flags: MapFlags,
) -> Result<ObjectHandle<HW>, &'static str> {
    alloc_object(address_space, None, size, flags)
}
pub fn map<HW: klib::hal::Hardware>(
    address_space: &AddressSpaceHandle<HW>,
    phys_base: klib::PhysicalPtr,
    size: usize,
    flags: MapFlags,
) -> Result<ObjectHandle<HW>, &'static str> {
    alloc_object(address_space, Some(phys_base), size, flags)
}
pub fn map_at<HW: klib::hal::Hardware>(
    address_space: &AddressSpaceHandle<HW>,
    virt_base: *mut u8,
    phys_base: klib::PhysicalPtr,
    size: usize,
    flags: MapFlags,
) -> Result<ObjectHandle<HW>, &'static str> {
    alloc_object_at(address_space, virt_base, Some(phys_base), size, flags)
}
pub fn map_unaligned<HW: klib::hal::Hardware>(
    address_space: &AddressSpaceHandle<HW>,
    phys_base: klib::PhysicalPtr,
    size: usize,
    flags: MapFlags,
) -> Result<(ObjectHandle<HW>, *mut u8), &'static str> {
    let offset = phys_base.addr() % HW::PAGE_SIZE;
    let page_base = phys_base
        .checked_sub(offset)
        .expect("physical address underflow");
    let actual_size: usize = size.checked_add(offset).ok_or("size too large")?;
    map(address_space, page_base, actual_size, flags)
        .map(|obj| (obj.clone(), unsafe { obj.borrow().start.add(offset) }))
}
pub fn map_unaligned_at<HW: klib::hal::Hardware>(
    address_space: &AddressSpaceHandle<HW>,
    virt_base: *mut u8,
    phys_base: klib::PhysicalPtr,
    size: usize,
    flags: MapFlags,
) -> Result<(ObjectHandle<HW>, *mut u8), &'static str> {
    let offset = phys_base.addr() % HW::PAGE_SIZE;
    let page_base = phys_base
        .checked_sub(offset)
        .expect("physical address underflow");
    let actual_size: usize = size.checked_add(offset).ok_or("size too large")?;
    map_at(address_space, virt_base, page_base, actual_size, flags)
        .map(|obj| (obj.clone(), unsafe { obj.borrow().start.add(offset) }))
}

#[repr(C)]
pub struct AddressSpace<HW> {
    object_list: Vec<ObjectHandle<HW>>,
    uncommitted_objects: Vec<UncommittedObjectHandle<HW>>,
    is_user: bool,
}
pub(in crate::klib) type AddressSpaceHandle<HW> = Rc<RefCell<AddressSpace<HW>>>;

impl<HW: klib::hal::Hardware> AddressSpace<HW> {
    pub fn new(start: *mut u8, size: usize, is_user: bool) -> AddressSpaceHandle<HW> {
        start
            .addr()
            .checked_add(size.checked_sub(1).expect("address space size is zero"))
            .expect("address space range overflow");
        let address_space = Rc::new(RefCell::new(AddressSpace {
            object_list: Default::default(),
            uncommitted_objects: Default::default(),
            is_user,
        }));
        let object = create_object(&address_space, start, size, None, Default::default());
        address_space.borrow_mut().add_object(&object);
        address_space
    }
    pub(in crate::klib) fn objects(&self) -> &[ObjectHandle<HW>] {
        &self.object_list
    }

    pub(in crate::klib) fn uncommitted_objects(&self) -> &[UncommittedObjectHandle<HW>] {
        &self.uncommitted_objects
    }
    fn take_object(&mut self, object: &ObjectHandle<HW>) {
        let value_index = self
            .object_list
            .iter()
            .position(|o| Rc::ptr_eq(o, object))
            .unwrap();
        self.object_list.remove(value_index);
    }
    fn take_object_with_min_size(&mut self, page_count: usize) -> Option<ObjectHandle<HW>> {
        assert_ne!(page_count, 0);
        let chosen = self
            .object_list
            .iter()
            .find(|object_h| {
                let object = object_h.borrow();
                let object_page_count = klib::miscmath::size_to_blocks(object.size, HW::PAGE_SIZE);
                page_count <= object_page_count
            })
            .cloned()?;
        self.take_object(&chosen);
        Some(chosen)
    }
    fn take_object_including(
        &mut self,
        start_addr: usize,
        end_addr: usize,
    ) -> Option<ObjectHandle<HW>> {
        let range_size = end_addr.checked_sub(start_addr)?.checked_add(1)?;
        let page_count = klib::miscmath::size_to_blocks(range_size, HW::PAGE_SIZE);
        assert_ne!(page_count, 0);
        let rounded_size = page_count.checked_mul(HW::PAGE_SIZE)?;
        let end_addr = start_addr.checked_add(rounded_size.checked_sub(1)?)?;
        let chosen = self
            .object_list
            .iter()
            .find(|object_h| {
                let object = object_h.borrow();
                (object.start_addr() <= start_addr)
                    && (start_addr <= object.end_addr())
                    && (object.start_addr() <= end_addr)
                    && (end_addr <= object.end_addr())
            })
            .cloned()?;
        self.take_object(&chosen);
        Some(chosen)
    }
    fn add_object(&mut self, object: &ObjectHandle<HW>) {
        let (new_start, new_size) = {
            let object = object.borrow();
            (object.start_addr(), object.size)
        };
        assert_ne!(
            new_size, 0,
            "cannot add an empty object to an address space"
        );
        assert!(klib::miscmath::is_aligned(new_size, HW::PAGE_SIZE));
        let new_end = new_start
            .checked_add(new_size - 1)
            .expect("object address range overflow");

        // Verify that it doesn't overlap with existing objects, and find sorted position to insert to.
        let mut insert_pos = self.object_list.len();
        for (i, object_h) in self.object_list.iter().enumerate() {
            let old_object = object_h.borrow();
            let old_start = old_object.start_addr();
            let old_end = old_object.end_addr();
            assert!(
                new_end < old_start || old_end < new_start,
                "address space objects overlap"
            );
            if insert_pos == self.object_list.len() && new_start < old_start {
                insert_pos = i;
            }
        }
        self.object_list.insert(insert_pos, object.clone());
    }
    fn find_object_in_uncommitted(&self, addr: usize) -> Option<UncommittedObjectHandle<HW>> {
        let page_base = klib::miscmath::align_usize_down(addr, HW::PAGE_SIZE);
        self.uncommitted_objects
            .iter()
            .find(|uobject_h| {
                let uobject = uobject_h.borrow();
                (uobject.object.borrow().start_addr() <= page_base)
                    && (page_base <= uobject.object.borrow().end_addr())
            })
            .cloned()
    }
}

#[derive(Default, Clone, Copy)]
pub struct MapFlags {
    pub read: bool,
    pub write: bool,
    pub exec: bool,
    pub no_cache: bool,
}

#[repr(C)]
pub struct Object<HW> {
    pub phys_base: Option<klib::PhysicalPtr>, // None means it allocates pages instead of mapping existing pages.
    pub address_space: AddressSpaceHandle<HW>,
    pub start: *mut u8,
    pub size: usize,
    pub flags: MapFlags,
}
impl<HW: klib::hal::Hardware> Object<HW> {
    fn start_addr(&self) -> usize {
        self.start.addr()
    }
    fn end_addr(&self) -> usize {
        self.start
            .addr()
            .checked_add(self.size.checked_sub(1).expect("object size is zero"))
            .expect("object address range overflow")
    }
    fn shrink_at_start(&mut self, new_size: usize) -> Result<(), &'static str> {
        if let Some(size) = self.size.checked_sub(new_size) {
            self.start = unsafe { self.start.add(new_size) };
            self.size = size;
            Ok(())
        } else {
            Err("size is too large")
        }
    }
    fn set_end(&mut self, ptr: *mut u8) -> Result<(), &'static str> {
        let end_exclusive = ptr.addr().checked_add(1).ok_or("invalid ptr address")?;
        let size = end_exclusive
            .checked_sub(self.start.addr())
            .ok_or("invalid end address")?;
        if size == 0 {
            return Err("invalid end address");
        }
        self.size = size;
        Ok(())
    }
}
pub type ObjectHandle<HW> = Rc<RefCell<Object<HW>>>;
fn create_object<HW>(
    address_space: &AddressSpaceHandle<HW>,
    start: *mut u8,
    size: usize,
    phys_base: Option<klib::PhysicalPtr>,
    flags: MapFlags,
) -> ObjectHandle<HW> {
    Rc::new(RefCell::new(Object {
        address_space: address_space.clone(),
        start,
        size,
        phys_base,
        flags,
    }))
}

#[repr(C)]
pub(in crate::klib) struct UncommittedObject<HW> {
    object: ObjectHandle<HW>,
    bitmap: Vec<klib::bitmap::Word>,
}
pub(in crate::klib) type UncommittedObjectHandle<HW> = Rc<RefCell<UncommittedObject<HW>>>;
impl<HW: klib::hal::Hardware> UncommittedObject<HW> {
    pub(in crate::klib) fn object(&self) -> &ObjectHandle<HW> {
        &self.object
    }

    pub(in crate::klib) fn uncommited_count(&self) -> usize {
        let page_count = self.object.borrow().size / HW::PAGE_SIZE;
        (0..page_count)
            .filter(|&page_index| self.is_page_uncommitted(page_index))
            .count()
    }
    fn is_page_uncommitted(&self, page_index: usize) -> bool {
        let bits_per_word = klib::bitmap::Word::BITS as usize;
        self.bitmap
            .get(page_index / bits_per_word)
            .is_some_and(|word| (word & (1 << (page_index % bits_per_word))) != 0)
    }
    fn mark_page_committed(&mut self, page_index: usize) {
        klib::Bitmap(&mut self.bitmap[..]).clear_bit(page_index);
    }
    fn has_uncommitted_pages(&mut self) -> bool {
        klib::Bitmap(&mut self.bitmap[..])
            .find_first_set_bit(0)
            .is_some()
    }
}
fn create_uncommitted_object<HW>(
    object: &ObjectHandle<HW>,
    page_count: usize,
) -> UncommittedObjectHandle<HW> {
    let word_count = klib::bitmap::needed_word_count(page_count);
    let mut bitmap_data = alloc::vec![0; word_count];
    klib::Bitmap(&mut bitmap_data[..]).set_bits(0, page_count);
    Rc::new(RefCell::new(UncommittedObject {
        object: object.clone(),
        bitmap: bitmap_data,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const PAGE_SIZE: usize = <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE;

    #[repr(align(4096))]
    struct TestMemory([u8; 8 * PAGE_SIZE]);

    #[test]
    fn alloc_object_at_splits_space_and_tracks_the_returned_object() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 8 * PAGE_SIZE, false);
        let requested_start = unsafe { base.add(2 * PAGE_SIZE) };
        let flags = MapFlags {
            read: true,
            write: true,
            ..Default::default()
        };

        let allocated = alloc_object_at::<klib::hal::TestHW>(
            &address_space,
            requested_start,
            Some(klib::PhysicalPtr::new(0x8000)),
            2 * PAGE_SIZE,
            flags,
        )
        .expect("allocation should fit in the address space");

        assert_eq!(allocated.borrow().start_addr(), requested_start.addr());
        assert_eq!(allocated.borrow().size, 2 * PAGE_SIZE);
        assert_eq!(
            allocated.borrow().phys_base,
            Some(klib::PhysicalPtr::new(0x8000))
        );
        assert!(allocated.borrow().flags.read);
        assert!(allocated.borrow().flags.write);

        let address_space = address_space.borrow();
        assert_eq!(address_space.object_list.len(), 2);
        assert_eq!(
            address_space.object_list[0].borrow().start_addr(),
            base.addr()
        );
        assert_eq!(address_space.object_list[0].borrow().size, 2 * PAGE_SIZE);
        assert_eq!(
            address_space.object_list[1].borrow().start_addr(),
            unsafe { base.add(4 * PAGE_SIZE) }.addr()
        );
        assert_eq!(address_space.object_list[1].borrow().size, 4 * PAGE_SIZE);
        assert_eq!(address_space.object_list[1].borrow().phys_base, None);

        assert_eq!(address_space.uncommitted_objects.len(), 1);
        let mut uobject = address_space.uncommitted_objects[0].borrow_mut();
        assert_eq!(uobject.object.borrow().start_addr(), requested_start.addr());
        assert!(uobject.is_page_uncommitted(0));
        assert!(uobject.is_page_uncommitted(1));
        assert!(!uobject.is_page_uncommitted(2));
        uobject.mark_page_committed(0);
        assert!(uobject.has_uncommitted_pages());
        uobject.mark_page_committed(1);
        assert!(!uobject.has_uncommitted_pages());
    }

    #[test]
    fn alloc_object_exact_fit_does_not_reinsert_an_empty_region() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 2 * PAGE_SIZE, false);

        let allocated = alloc_object::<klib::hal::TestHW>(
            &address_space,
            None,
            2 * PAGE_SIZE,
            MapFlags::default(),
        )
        .expect("whole region should be allocatable");

        assert_eq!(allocated.borrow().start_addr(), base.addr());
        assert_eq!(allocated.borrow().size, 2 * PAGE_SIZE);
        assert!(address_space.borrow().object_list.is_empty());
        assert_eq!(address_space.borrow().uncommitted_objects.len(), 1);
    }

    fn assert_alloc_object_at_boundary(
        offset_pages: usize,
        allocation_pages: usize,
        expected_free_ranges: &[(usize, usize)],
    ) {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 8 * PAGE_SIZE, false);
        let requested_start = unsafe { base.add(offset_pages * PAGE_SIZE) };

        let allocated = alloc_object_at::<klib::hal::TestHW>(
            &address_space,
            requested_start,
            None,
            allocation_pages * PAGE_SIZE,
            MapFlags::default(),
        )
        .expect("boundary allocation should fit");

        assert_eq!(allocated.borrow().start_addr(), requested_start.addr());
        assert_eq!(allocated.borrow().size, allocation_pages * PAGE_SIZE);
        let address_space = address_space.borrow();
        assert_eq!(address_space.object_list.len(), expected_free_ranges.len());
        for (object, (start_page, page_count)) in
            address_space.object_list.iter().zip(expected_free_ranges)
        {
            assert_eq!(
                object.borrow().start_addr(),
                unsafe { base.add(start_page * PAGE_SIZE) }.addr()
            );
            assert_eq!(object.borrow().size, page_count * PAGE_SIZE);
        }
    }

    #[test]
    fn alloc_object_at_handles_start_end_and_whole_range_boundaries() {
        // Allocation at the startning leaves only a right-hand free range.
        assert_alloc_object_at_boundary(0, 2, &[(2, 6)]);
        // Allocation at the end leaves only a left-hand free range.
        assert_alloc_object_at_boundary(6, 2, &[(0, 6)]);
        // Exact fit leaves no free range and must not create empty objects.
        assert_alloc_object_at_boundary(0, 8, &[]);
    }

    #[test]
    fn failed_allocations_leave_address_space_unchanged() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 8 * PAGE_SIZE, false);
        let original = address_space.borrow().object_list[0].clone();

        assert!(
            alloc_object::<klib::hal::TestHW>(
                &address_space,
                None,
                usize::MAX,
                MapFlags::default(),
            )
            .is_err()
        );
        assert!(
            alloc_object_at::<klib::hal::TestHW>(
                &address_space,
                unsafe { base.add(8 * PAGE_SIZE) },
                None,
                PAGE_SIZE,
                MapFlags::default(),
            )
            .is_err()
        );

        let address_space = address_space.borrow();
        assert_eq!(address_space.object_list.len(), 1);
        assert!(Rc::ptr_eq(&address_space.object_list[0], &original));
        assert!(address_space.uncommitted_objects.is_empty());
    }

    #[test]
    fn unaligned_mapping_apis_return_the_requested_offset() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 8 * PAGE_SIZE, false);
        let phys_base = klib::PhysicalPtr::new(0x800d);
        let size = PAGE_SIZE + 20;
        let offset = phys_base.addr() % PAGE_SIZE;

        let (object, mapped_ptr) = map_unaligned::<klib::hal::TestHW>(
            &address_space,
            phys_base,
            size,
            MapFlags::default(),
        )
        .expect("unaligned physical mapping should fit");

        assert_eq!(object.borrow().start_addr(), base.addr());
        assert_eq!(
            object.borrow().phys_base,
            Some(phys_base.checked_sub(offset).unwrap())
        );
        assert_eq!(object.borrow().size, 2 * PAGE_SIZE);
        assert_eq!(mapped_ptr.addr(), base.addr() + offset);

        let mut second_memory = TestMemory([0; 8 * PAGE_SIZE]);
        let second_base = second_memory.0.as_mut_ptr();
        let second_address_space =
            AddressSpace::<klib::hal::TestHW>::new(second_base, 8 * PAGE_SIZE, false);
        let virt_base = unsafe { second_base.add(2 * PAGE_SIZE) };
        let phys_base = klib::PhysicalPtr::new(0xa013);
        let offset = phys_base.addr() % PAGE_SIZE;

        let (object, mapped_ptr) = map_unaligned_at::<klib::hal::TestHW>(
            &second_address_space,
            virt_base,
            phys_base,
            200,
            MapFlags::default(),
        )
        .expect("unaligned fixed mapping should fit");

        assert_eq!(object.borrow().start_addr(), virt_base.addr());
        assert_eq!(
            object.borrow().phys_base,
            Some(phys_base.checked_sub(offset).unwrap())
        );
        assert_eq!(object.borrow().size, PAGE_SIZE);
        assert_eq!(mapped_ptr.addr(), virt_base.addr() + offset);
    }

    #[test]
    fn add_object_keeps_ranges_sorted_when_inserted_out_of_order() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 8 * PAGE_SIZE, false);
        let original = address_space.borrow().object_list[0].clone();
        address_space.borrow_mut().take_object(&original);

        let high = create_object(
            &address_space,
            unsafe { base.add(4 * PAGE_SIZE) },
            PAGE_SIZE,
            None,
            MapFlags::default(),
        );
        let low = create_object(&address_space, base, PAGE_SIZE, None, MapFlags::default());
        address_space.borrow_mut().add_object(&high);
        address_space.borrow_mut().add_object(&low);

        let address_space = address_space.borrow();
        assert_eq!(address_space.object_list.len(), 2);
        assert_eq!(
            address_space.object_list[0].borrow().start_addr(),
            base.addr()
        );
        assert_eq!(
            address_space.object_list[1].borrow().start_addr(),
            unsafe { base.add(4 * PAGE_SIZE) }.addr()
        );
    }

    #[test]
    #[should_panic(expected = "address space objects overlap")]
    fn add_object_rejects_overlapping_ranges() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 2 * PAGE_SIZE, false);
        let overlapping = create_object(
            &address_space,
            unsafe { base.add(PAGE_SIZE) },
            PAGE_SIZE,
            None,
            MapFlags::default(),
        );

        address_space.borrow_mut().add_object(&overlapping);
    }

    #[test]
    #[should_panic]
    fn alloc_object_rejects_zero_size() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let address_space =
            AddressSpace::<klib::hal::TestHW>::new(memory.0.as_mut_ptr(), 8 * PAGE_SIZE, false);

        let _ = alloc_object::<klib::hal::TestHW>(&address_space, None, 0, MapFlags::default());
    }

    #[test]
    #[should_panic]
    fn alloc_object_rejects_unaligned_physical_base() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let address_space =
            AddressSpace::<klib::hal::TestHW>::new(memory.0.as_mut_ptr(), 8 * PAGE_SIZE, false);

        let _ = alloc_object::<klib::hal::TestHW>(
            &address_space,
            Some(klib::PhysicalPtr::new(PAGE_SIZE + 1)),
            PAGE_SIZE,
            MapFlags::default(),
        );
    }

    #[test]
    #[should_panic]
    fn alloc_object_at_rejects_unaligned_virtual_base() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let address_space =
            AddressSpace::<klib::hal::TestHW>::new(memory.0.as_mut_ptr(), 8 * PAGE_SIZE, false);

        let _ = alloc_object_at::<klib::hal::TestHW>(
            &address_space,
            unsafe { memory.0.as_mut_ptr().add(1) },
            None,
            PAGE_SIZE,
            MapFlags::default(),
        );
    }

    #[test]
    fn fixed_allocation_reports_virtual_range_overflow() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let address_space =
            AddressSpace::<klib::hal::TestHW>::new(memory.0.as_mut_ptr(), 8 * PAGE_SIZE, false);
        let virt_base = (usize::MAX & !(PAGE_SIZE - 1)) as *mut u8;

        let error = alloc_object_at::<klib::hal::TestHW>(
            &address_space,
            virt_base,
            None,
            2 * PAGE_SIZE,
            MapFlags::default(),
        )
        .err()
        .unwrap();

        assert_eq!(error, "virtual address range overflow");
    }

    #[test]
    fn fixed_mapping_reports_physical_range_overflow() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let address_space =
            AddressSpace::<klib::hal::TestHW>::new(memory.0.as_mut_ptr(), 8 * PAGE_SIZE, false);

        let error = alloc_object::<klib::hal::TestHW>(
            &address_space,
            Some(klib::PhysicalPtr::new(usize::MAX - PAGE_SIZE + 1)),
            2 * PAGE_SIZE,
            MapFlags::default(),
        )
        .err()
        .unwrap();

        assert_eq!(error, "physical address range overflow");
    }

    #[test]
    fn unaligned_mapping_apis_reject_size_overflow() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let address_space =
            AddressSpace::<klib::hal::TestHW>::new(memory.0.as_mut_ptr(), 8 * PAGE_SIZE, false);

        assert_eq!(
            map_unaligned::<klib::hal::TestHW>(
                &address_space,
                klib::PhysicalPtr::new(PAGE_SIZE + 1),
                usize::MAX,
                MapFlags::default(),
            )
            .err()
            .unwrap(),
            "size too large"
        );

        assert_eq!(
            map_unaligned_at::<klib::hal::TestHW>(
                &address_space,
                memory.0.as_mut_ptr(),
                klib::PhysicalPtr::new(PAGE_SIZE + 1),
                usize::MAX,
                MapFlags::default(),
            )
            .err()
            .unwrap(),
            "size too large"
        );
    }

    #[test]
    #[should_panic(expected = "address space size is zero")]
    fn address_space_rejects_zero_size() {
        let _ = AddressSpace::<klib::hal::TestHW>::new(core::ptr::null_mut(), 0, false);
    }

    #[test]
    #[should_panic(expected = "address space range overflow")]
    fn address_space_rejects_range_overflow() {
        let _ = AddressSpace::<klib::hal::TestHW>::new(usize::MAX as *mut u8, 2, false);
    }

    #[test]
    fn default_map_flags_disable_every_permission_and_cache_option() {
        let flags = MapFlags::default();

        assert!(!flags.read);
        assert!(!flags.write);
        assert!(!flags.exec);
        assert!(!flags.no_cache);
    }

    #[test]
    fn take_object_with_min_size_leaves_space_unchanged_when_no_region_fits() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 2 * PAGE_SIZE, false);
        let original = address_space.borrow().object_list[0].clone();

        assert!(
            address_space
                .borrow_mut()
                .take_object_with_min_size(3)
                .is_none()
        );
        assert_eq!(address_space.borrow().object_list.len(), 1);
        assert!(Rc::ptr_eq(
            &address_space.borrow().object_list[0],
            &original
        ));
    }

    #[test]
    fn take_object_with_min_size_removes_a_fitting_region() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 4 * PAGE_SIZE, false);
        let original = address_space.borrow().object_list[0].clone();

        let taken = address_space
            .borrow_mut()
            .take_object_with_min_size(4)
            .unwrap();

        assert!(Rc::ptr_eq(&taken, &original));
        assert!(address_space.borrow().object_list.is_empty());
    }

    #[test]
    fn take_object_including_requires_the_rounded_range_to_fit() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 2 * PAGE_SIZE, false);
        let original = address_space.borrow().object_list[0].clone();

        assert!(
            address_space
                .borrow_mut()
                .take_object_including(base.addr() + PAGE_SIZE, base.addr() + 2 * PAGE_SIZE)
                .is_none()
        );
        assert_eq!(address_space.borrow().object_list.len(), 1);

        let taken = address_space
            .borrow_mut()
            .take_object_including(base.addr() + PAGE_SIZE, base.addr() + 2 * PAGE_SIZE - 1)
            .unwrap();
        assert!(Rc::ptr_eq(&taken, &original));
        assert!(address_space.borrow().object_list.is_empty());
    }

    #[test]
    fn shrink_at_start_preserves_end_and_rejects_oversized_shrink() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 4 * PAGE_SIZE, false);
        let object = create_object(
            &address_space,
            base,
            4 * PAGE_SIZE,
            None,
            MapFlags::default(),
        );

        object.borrow_mut().shrink_at_start(PAGE_SIZE).unwrap();
        assert_eq!(object.borrow().start_addr(), base.addr() + PAGE_SIZE);
        assert_eq!(object.borrow().size, 3 * PAGE_SIZE);
        assert_eq!(object.borrow().end_addr(), base.addr() + 4 * PAGE_SIZE - 1);

        assert_eq!(
            object.borrow_mut().shrink_at_start(4 * PAGE_SIZE),
            Err("size is too large")
        );
        assert_eq!(object.borrow().size, 3 * PAGE_SIZE);
    }

    #[test]
    fn set_end_updates_size_and_rejects_an_end_before_start() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 4 * PAGE_SIZE, false);
        let object = create_object(
            &address_space,
            base,
            3 * PAGE_SIZE,
            None,
            MapFlags::default(),
        );

        object
            .borrow_mut()
            .set_end(unsafe { base.add(2 * PAGE_SIZE - 1) })
            .unwrap();
        assert_eq!(object.borrow().size, 2 * PAGE_SIZE);
        assert_eq!(
            object.borrow_mut().set_end((base.addr() - 1) as *mut u8),
            Err("invalid end address")
        );
        assert_eq!(object.borrow().size, 2 * PAGE_SIZE);
    }

    #[test]
    fn uncommitted_lookup_matches_any_page_in_the_object() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 4 * PAGE_SIZE, false);
        let object = create_object(
            &address_space,
            base,
            2 * PAGE_SIZE,
            None,
            MapFlags::default(),
        );
        let uncommitted = create_uncommitted_object(&object, 2);
        address_space
            .borrow_mut()
            .uncommitted_objects
            .push(uncommitted.clone());

        let found = address_space
            .borrow()
            .find_object_in_uncommitted(base.addr() + PAGE_SIZE + 17)
            .unwrap();
        assert!(Rc::ptr_eq(&found, &uncommitted));
        assert!(
            address_space
                .borrow()
                .find_object_in_uncommitted(base.addr() + 2 * PAGE_SIZE)
                .is_none()
        );
    }

    #[test]
    fn allocation_rounds_requested_size_up_to_whole_pages() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 8 * PAGE_SIZE, false);

        let object = alloc_object::<klib::hal::TestHW>(
            &address_space,
            None,
            PAGE_SIZE + 1,
            MapFlags::default(),
        )
        .unwrap();

        assert_eq!(object.borrow().size, 2 * PAGE_SIZE);
        assert_eq!(address_space.borrow().object_list.len(), 1);
        assert_eq!(
            address_space.borrow().object_list[0].borrow().size,
            6 * PAGE_SIZE
        );
        assert_eq!(address_space.borrow().uncommitted_objects.len(), 1);
        assert!(
            address_space.borrow().uncommitted_objects[0]
                .borrow_mut()
                .is_page_uncommitted(1)
        );
    }

    #[test]
    fn fixed_allocation_at_reports_physical_range_overflow() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 8 * PAGE_SIZE, false);

        let error = alloc_object_at::<klib::hal::TestHW>(
            &address_space,
            base,
            Some(klib::PhysicalPtr::new(usize::MAX - PAGE_SIZE + 1)),
            2 * PAGE_SIZE,
            MapFlags::default(),
        )
        .err()
        .unwrap();

        assert_eq!(error, "physical address range overflow");
    }

    #[test]
    fn freeing_an_uncommitted_object_restores_its_virtual_range() {
        let mut memory = TestMemory([0; 8 * PAGE_SIZE]);
        let base = memory.0.as_mut_ptr();
        let address_space = AddressSpace::<klib::hal::TestHW>::new(base, 4 * PAGE_SIZE, false);
        let object = alloc_object::<klib::hal::TestHW>(
            &address_space,
            None,
            2 * PAGE_SIZE,
            MapFlags::default(),
        )
        .unwrap();
        let mut pmm = klib::mem::PhysicalMemoryManager::<klib::hal::TestHW>::new();

        free::<klib::hal::TestHW>(&object, &mut pmm);

        let address_space = address_space.borrow();
        assert!(address_space.uncommitted_objects.is_empty());
        assert_eq!(address_space.object_list.len(), 2);
        assert_eq!(
            address_space.object_list[0].borrow().start_addr(),
            base.addr()
        );
        assert_eq!(address_space.object_list[0].borrow().size, 2 * PAGE_SIZE);
        assert_eq!(
            address_space.object_list[1].borrow().start_addr(),
            base.addr() + 2 * PAGE_SIZE
        );
        assert_eq!(address_space.object_list[1].borrow().size, 2 * PAGE_SIZE);
    }

    #[test]
    fn page_fault_commits_a_page_from_a_fixed_physical_mapping() {
        let virt_base = 0x20_000usize as *mut u8;
        let phys_base = klib::PhysicalPtr::new(0x80_000);
        let address_space = AddressSpace::<klib::hal::TestHW>::new(virt_base, 2 * PAGE_SIZE, false);
        let object = alloc_object_at::<klib::hal::TestHW>(
            &address_space,
            virt_base,
            Some(phys_base),
            PAGE_SIZE,
            MapFlags {
                read: true,
                write: true,
                ..Default::default()
            },
        )
        .unwrap();
        let mut manager = VirtualMemoryManager::<klib::hal::TestHW> {
            kernel_address_space: Some(address_space.clone()),
        };
        let mut pmm = klib::mem::PhysicalMemoryManager::<klib::hal::TestHW>::new();

        manager.page_fault(
            |count| pmm.malloc(count),
            virt_base,
            false,
            true,
            false,
            &(),
        );

        assert!(address_space.borrow().uncommitted_objects.is_empty());
        assert!(
            address_space
                .borrow()
                .find_object_in_uncommitted(virt_base.addr())
                .is_none()
        );
        assert_eq!(object.borrow().phys_base, Some(phys_base));
    }

    #[test]
    fn kernel_address_space_is_lazily_created_and_reused_for_kernel_addresses() {
        let mut manager = VirtualMemoryManager::<klib::hal::TestHW> {
            kernel_address_space: None,
        };
        let kernel_start = <klib::hal::TestHW as klib::hal::Hardware>::kernel_vm_start_address();
        let kernel_end = <klib::hal::TestHW as klib::hal::Hardware>::kernel_vm_end_address();

        let from_start = manager.address_space_of(kernel_start as *mut u8).unwrap();
        let from_end = manager.address_space_of(kernel_end as *mut u8).unwrap();
        let from_accessor = manager.kernel_address_space();

        assert!(Rc::ptr_eq(&from_start, &from_end));
        assert!(Rc::ptr_eq(&from_start, &from_accessor));
        let address_space = from_accessor.borrow();
        assert_eq!(address_space.object_list.len(), 1);
        let range = address_space.object_list[0].borrow();
        assert_eq!(range.start_addr(), kernel_start);
        assert_eq!(
            range.size,
            kernel_end - kernel_start + 1,
            "kernel virtual range includes both endpoints"
        );
        assert!(
            manager
                .address_space_of((kernel_end + 1) as *mut u8)
                .is_none()
        );
    }

    #[test]
    fn ezmap_returns_the_address_within_its_page_for_unaligned_physical_memory() {
        let mut manager = VirtualMemoryManager::<klib::hal::TestHW> {
            kernel_address_space: None,
        };
        let phys_base = klib::PhysicalPtr::new(0x12_345);
        let size = 17;

        let mapped = manager.ezmap(phys_base, size);

        let address_space = manager.kernel_address_space();
        let address_space = address_space.borrow();
        assert_eq!(address_space.object_list.len(), 1);
        let object = address_space.uncommitted_objects[0].borrow().object.clone();
        let object = object.borrow();
        assert_eq!(
            object.phys_base,
            Some(phys_base.checked_sub(phys_base.addr() % PAGE_SIZE).unwrap())
        );
        assert_eq!(
            object.start_addr(),
            <klib::hal::TestHW as klib::hal::Hardware>::kernel_vm_start_address()
        );
        assert_eq!(object.size, PAGE_SIZE);
        assert!(object.flags.read);
        assert!(object.flags.write);
        assert_eq!(
            mapped.addr(),
            object.start_addr() + phys_base.addr() % PAGE_SIZE
        );
        assert_eq!(
            klib::miscmath::size_to_blocks(size + phys_base.addr() % PAGE_SIZE, PAGE_SIZE),
            1,
            "the requested mapping fits in one physical page"
        );
    }

    #[test]
    #[should_panic(expected = "ran out of memory while trying to commit the page")]
    fn page_fault_panics_when_the_physical_memory_manager_is_empty() {
        let virt_base = 0x20_000usize as *mut u8;
        let address_space = AddressSpace::<klib::hal::TestHW>::new(virt_base, 2 * PAGE_SIZE, false);
        alloc_object_at::<klib::hal::TestHW>(
            &address_space,
            virt_base,
            None,
            2 * PAGE_SIZE,
            MapFlags {
                read: true,
                write: true,
                ..Default::default()
            },
        )
        .unwrap();
        let mut manager = VirtualMemoryManager::<klib::hal::TestHW> {
            kernel_address_space: Some(address_space.clone()),
        };
        let mut pmm = klib::mem::pmm::PhysicalMemoryManager::<klib::hal::TestHW>::new();

        manager.page_fault(
            |count| pmm.malloc(count),
            virt_base,
            false,
            false,
            false,
            &(),
        );
    }

    #[test]
    #[should_panic(expected = "non-present page on already committed page")]
    fn repeated_page_fault_on_a_committed_page_panics() {
        let virt_base = 0x20_000usize as *mut u8;
        let address_space = AddressSpace::<klib::hal::TestHW>::new(virt_base, 2 * PAGE_SIZE, false);
        alloc_object_at::<klib::hal::TestHW>(
            &address_space,
            virt_base,
            Some(klib::PhysicalPtr::new(0x80_000)),
            2 * PAGE_SIZE,
            MapFlags::default(),
        )
        .unwrap();
        let mut manager = VirtualMemoryManager::<klib::hal::TestHW> {
            kernel_address_space: Some(address_space),
        };
        let mut pmm = klib::mem::pmm::PhysicalMemoryManager::<klib::hal::TestHW>::new();

        manager.page_fault(
            |count| pmm.malloc(count),
            virt_base,
            false,
            false,
            false,
            &(),
        );
        manager.page_fault(
            |count| pmm.malloc(count),
            virt_base,
            false,
            false,
            false,
            &(),
        );
    }

    #[test]
    #[should_panic(expected = "fatal memory access fault")]
    fn page_fault_on_a_present_page_is_fatal() {
        let virt_base = 0x20_000usize as *mut u8;
        let address_space = AddressSpace::<klib::hal::TestHW>::new(virt_base, PAGE_SIZE, false);
        let mut manager = VirtualMemoryManager::<klib::hal::TestHW> {
            kernel_address_space: Some(address_space),
        };
        let mut pmm = klib::mem::pmm::PhysicalMemoryManager::<klib::hal::TestHW>::new();

        manager.page_fault(
            |count| pmm.malloc(count),
            virt_base,
            true,
            false,
            false,
            &(),
        );
    }

    #[test]
    #[should_panic(expected = "fatal memory access fault")]
    fn null_page_fault_is_fatal() {
        let mut manager = VirtualMemoryManager::<klib::hal::TestHW> {
            kernel_address_space: None,
        };
        let mut pmm = klib::mem::pmm::PhysicalMemoryManager::<klib::hal::TestHW>::new();

        manager.page_fault(
            |count| pmm.malloc(count),
            core::ptr::null_mut(),
            false,
            false,
            false,
            &(),
        );
    }

    #[test]
    #[should_panic(expected = "fatal memory access fault")]
    fn page_fault_on_a_page_outside_uncommitted_objects_is_fatal() {
        let virt_base = 0x20_000usize as *mut u8;
        let address_space = AddressSpace::<klib::hal::TestHW>::new(virt_base, 2 * PAGE_SIZE, false);
        let mut manager = VirtualMemoryManager::<klib::hal::TestHW> {
            kernel_address_space: Some(address_space),
        };
        let mut pmm = klib::mem::PhysicalMemoryManager::<klib::hal::TestHW>::new();

        manager.page_fault(
            |count| pmm.malloc(count),
            virt_base,
            false,
            false,
            false,
            &(),
        );
    }
}
