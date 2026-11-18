use crate::klib;
use core::{
    marker::{PhantomData, PhantomPinned},
    mem::{align_of, offset_of, size_of, size_of_val},
    pin::Pin,
    ptr::NonNull,
    slice::from_raw_parts_mut,
};

#[derive(Default)]
#[repr(C, align(16))]
struct HeapAlignment([u8; 0]);
#[repr(align(16))]
struct InitialHeapMemory([u8; 1024 * 1024 * 2]);

#[repr(C)]
struct Pool {
    block_pool_p: *mut u8,
    block_bitmap: klib::Bitmap<'static>,
    block_count: usize,
    used_block_count: usize,
    page_count: usize,
    heap_data: HeapAlignment,
}
type PoolHeader = klib::list::Node<Pool>;
#[repr(C)]
struct AllocHeaderData {
    block_count: usize,
    pool_p: *mut PoolHeader,
    size: usize,
    heap_data: HeapAlignment,
}
impl AllocHeaderData {
    fn pool(&self) -> &PoolHeader {
        unsafe { self.pool_p.as_ref().unwrap() }
    }
    fn pool_mut(&mut self) -> &mut PoolHeader {
        unsafe { self.pool_p.as_mut().unwrap() }
    }
}
type AllocHeader = klib::list::Node<AllocHeaderData>;

const BLOCK_SIZE: usize = 64;
const POISON_VALUES: [u8; 8] = [0xe9, 0x29, 0xf3, 0xfb, 0xd7, 0x67, 0xaa, 0x5a];
const ALLOC_HEADER_LOCATOR_SIZE: usize = size_of::<usize>();
const MIN_ALLOC_ALIGNMENT: usize = align_of::<HeapAlignment>();

const fn byte_count_for_block_count(block_count: usize) -> usize {
    (BLOCK_SIZE * block_count)
        - (size_of::<AllocHeader>()
            + ALLOC_HEADER_LOCATOR_SIZE
            + (MIN_ALLOC_ALIGNMENT - 1)
            + (size_of::<u8>() * POISON_VALUES.len()))
}
const fn actual_alloc_size(size: usize) -> usize {
    size + size_of::<AllocHeader>()
        + ALLOC_HEADER_LOCATOR_SIZE
        + (MIN_ALLOC_ALIGNMENT - 1)
        + (size_of::<u8>() * POISON_VALUES.len())
}
fn actual_alloc_size_for_alignment(size: usize, alignment: usize) -> Option<usize> {
    let alignment = alignment.max(MIN_ALLOC_ALIGNMENT);
    size.checked_add(size_of::<AllocHeader>())?
        .checked_add(ALLOC_HEADER_LOCATOR_SIZE)?
        .checked_add(alignment - 1)?
        .checked_add(POISON_VALUES.len())
}
/// # Safety
/// ptr must point to the same address returned by `malloc` or `malloc_aligned`, and was not freed.
unsafe fn alloc_header_of(ptr: *mut u8) -> Option<NonNull<AllocHeader>> {
    if ptr.is_null() || !klib::miscmath::is_aligned(ptr.addr(), MIN_ALLOC_ALIGNMENT) {
        return None;
    }
    let header_offset = unsafe {
        ptr.byte_sub(ALLOC_HEADER_LOCATOR_SIZE)
            .cast::<usize>()
            .read()
    };
    let header_addr = ptr.addr().checked_sub(header_offset)?;
    if !klib::miscmath::is_aligned(header_addr, align_of::<AllocHeader>()) {
        return None;
    }
    NonNull::new(header_addr as *mut AllocHeader)
}

pub struct Heap<HW> {
    free_block_count: usize,
    heap_pool_list: klib::UnsafeList<Pool>,
    alloc_list: klib::UnsafeList<AllocHeaderData>,
    initial_heap_memory: InitialHeapMemory,
    initial_heap_initialized: bool,
    _hw: PhantomData<HW>,
    _pin: PhantomPinned,
}
impl<HW: klib::hal::Hardware> Heap<HW> {
    pub const fn new() -> Self {
        Self {
            free_block_count: 0,
            heap_pool_list: klib::UnsafeList::new(),
            alloc_list: klib::UnsafeList::new(),
            initial_heap_memory: InitialHeapMemory([0; 1024 * 1024 * 2]),
            initial_heap_initialized: false,
            _hw: PhantomData,
            _pin: PhantomPinned,
        }
    }

    /// # Safety
    /// pool_p must point to initialized & valid PoolHeader, and part of this Heap.
    unsafe fn alloc_from_pool(
        &mut self,
        pool_p: *mut PoolHeader,
        size: usize,
        alignment: usize,
    ) -> Option<NonNull<u8>> {
        let pool = unsafe { pool_p.as_mut() }.unwrap();
        if size == 0 {
            return None;
        }

        let alignment = alignment.max(MIN_ALLOC_ALIGNMENT);
        let actual_size = size
            .checked_add(size_of::<AllocHeader>())?
            .checked_add(ALLOC_HEADER_LOCATOR_SIZE)?
            .checked_add(alignment - 1)?
            .checked_add(POISON_VALUES.len())?;
        let block_count = klib::miscmath::size_to_blocks(actual_size, BLOCK_SIZE);
        let block_index = pool.block_bitmap.find_set_bits(0, block_count)?;
        for i in 0..block_count {
            assert!(pool.block_bitmap.is_bit_set(block_index + i));
        }
        pool.block_bitmap.clear_bits(block_index, block_count);
        for i in 0..block_count {
            assert!(!pool.block_bitmap.is_bit_set(block_index + i));
        }
        let alloc_off = block_index * BLOCK_SIZE;
        assert!(klib::miscmath::is_aligned(alloc_off, 16));
        let alloc_p: *mut AllocHeader =
            unsafe { pool.block_pool_p.add(alloc_off) } as *mut AllocHeader;
        pool.used_block_count += block_count;
        unsafe {
            alloc_p.write(klib::list::Node::new(AllocHeaderData {
                block_count,
                pool_p,
                size,
                heap_data: Default::default(),
            }))
        };
        let data_p = unsafe {
            alloc_p
                .byte_add(offset_of!(AllocHeader, data.heap_data))
                .cast::<u8>()
        };
        let min_payload_addr = data_p
            .addr()
            .checked_add(ALLOC_HEADER_LOCATOR_SIZE)
            .expect("heap allocation address overflow");
        let payload_addr = min_payload_addr
            .checked_add(alignment - 1)
            .expect("heap alignment address overflow")
            & !(alignment - 1);
        let payload_offset = payload_addr
            .checked_sub(alloc_p.addr())
            .expect("invalid heap allocation offset");
        let payload_p = unsafe { alloc_p.cast::<u8>().byte_add(payload_offset) };
        unsafe {
            payload_p
                .byte_sub(ALLOC_HEADER_LOCATOR_SIZE)
                .cast::<usize>()
                .write(payload_offset)
        };
        let alloc = unsafe { alloc_p.as_mut().unwrap() };
        assert!(block_count <= self.free_block_count);
        self.free_block_count -= block_count;
        unsafe { payload_p.write_bytes(0x00, size) };
        // Setup poision values ================================================
        let poison_dest = unsafe { payload_p.add(size) };
        for (i, v) in POISON_VALUES.iter().enumerate() {
            unsafe { poison_dest.add(i).write(*v) };
        }
        unsafe { self.alloc_list.insert_back(Pin::new_unchecked(alloc)) };
        NonNull::new(payload_p)
    }
    /// # Safety
    /// Memory area pointed by mem_p and mem_size must be:
    /// - Does not overlap with any registered memory.
    /// - Can be both written and read without any restrictions
    /// - Valid for 'static lifetime
    /// - Aligned to 16-byte boundary
    pub unsafe fn add_mem(
        self: Pin<&mut Self>,
        mem_p: *mut u8,
        mem_size: usize,
    ) -> Result<(), &'static str> {
        let heap = unsafe { self.get_unchecked_mut() };
        assert!(klib::miscmath::is_aligned(
            mem_p.addr(),
            align_of::<HeapAlignment>()
        ));
        let max_block_count = klib::miscmath::size_to_blocks(mem_size, BLOCK_SIZE);
        let mut word_count = klib::bitmap::needed_word_count(max_block_count);
        let metadata_size =
            size_of::<PoolHeader>() + (word_count * size_of::<klib::bitmap::Word>());
        let max_size = metadata_size + (max_block_count * BLOCK_SIZE);
        let mut page_count = klib::miscmath::size_to_blocks(max_size, HW::PAGE_SIZE);

        // We have to make sure that total size doesn't exceed size of that buffer.
        while mem_size < page_count * HW::PAGE_SIZE {
            if page_count == 0 {
                return Err("mem_size is too small");
            }
            page_count -= 1;
        }
        if page_count == 0 {
            return Err("mem_size is too small");
        }

        // Now we calculate the actual size ====================================
        let total_block_count =
            klib::miscmath::size_to_blocks(page_count * HW::PAGE_SIZE, BLOCK_SIZE);
        word_count = klib::bitmap::needed_word_count(total_block_count);
        let mut bitmap_size = word_count * size_of::<klib::bitmap::Word>();

        // Don't forget to align! ==============================================
        let align = align_of::<HeapAlignment>();
        bitmap_size += (align - bitmap_size % align) % align;
        let total_size = total_block_count * BLOCK_SIZE;
        let max_pool_size = total_size - bitmap_size - size_of::<PoolHeader>();
        let pool_block_count = max_pool_size / BLOCK_SIZE;

        // Initialize the pool =================================================
        let pool_p = mem_p as *mut PoolHeader;
        let pool_data_start =
            unsafe { pool_p.byte_add(offset_of!(PoolHeader, data.heap_data) + bitmap_size) }
                as *mut u8;
        assert!(
            pool_data_start
                .addr()
                .is_multiple_of(align_of::<HeapAlignment>())
        );
        let pool = unsafe {
            pool_p.write(klib::list::Node::new(Pool {
                block_pool_p: pool_data_start,
                block_bitmap: klib::Bitmap(from_raw_parts_mut(
                    pool_p.byte_add(offset_of!(PoolHeader, data.heap_data)) as *mut u32,
                    word_count,
                )),
                block_count: pool_block_count,
                used_block_count: 0,
                page_count,
                heap_data: Default::default(),
            }));
            pool_p.as_mut().unwrap()
        };
        pool.block_bitmap.0.fill(0);
        pool.block_bitmap.set_bits(0, pool_block_count);
        heap.free_block_count += pool.block_count;
        unsafe { heap.heap_pool_list.insert_back(Pin::new_unchecked(pool)) };

        Ok(())
    }
    pub fn malloc(self: Pin<&mut Self>, size: usize) -> Option<NonNull<u8>> {
        self.malloc_aligned(size, MIN_ALLOC_ALIGNMENT)
    }

    /// Allocate `size` bytes with the requested power-of-two alignment.
    pub fn malloc_aligned(
        mut self: Pin<&mut Self>,
        size: usize,
        alignment: usize,
    ) -> Option<NonNull<u8>> {
        if size == 0 || alignment == 0 || !alignment.is_power_of_two() {
            return None;
        }
        let actual_size = actual_alloc_size_for_alignment(size, alignment)?;
        let actual_block_count = klib::miscmath::size_to_blocks(actual_size, BLOCK_SIZE);
        if !self.as_ref().initial_heap_initialized {
            let heap_ptr;
            let heap_size;
            {
                let heap = unsafe { self.as_mut().get_unchecked_mut() };
                heap_ptr = heap.initial_heap_memory.0.as_mut_ptr();
                heap_size = size_of_val(&heap.initial_heap_memory);
            }
            unsafe { self.as_mut().add_mem(heap_ptr, heap_size).ok()? };
            (unsafe { self.as_mut().get_unchecked_mut() }).initial_heap_initialized = true;
        }
        let mut result = None;
        if actual_block_count <= self.as_ref().free_block_count {
            let heap = unsafe { self.as_mut().get_unchecked_mut() };
            let mut next_pool = heap.heap_pool_list.front_mut();
            while let Some(pool_p) = next_pool {
                result = unsafe { heap.alloc_from_pool(pool_p.as_ptr(), size, alignment) };
                if result.is_some() {
                    break;
                }
                next_pool = unsafe { pool_p.as_ref() }.next();
            }
        }
        result
    }

    /// # Safety
    /// ptr must point to the same address returned by `malloc` or `malloc_aligned`, and not freed yet.
    pub unsafe fn free(self: Pin<&mut Self>, ptr: NonNull<u8>) {
        let heap = unsafe { self.get_unchecked_mut() };
        let mut alloc_p = unsafe { alloc_header_of(ptr.as_ptr()) }.expect("bad pointer");
        unsafe { heap.alloc_list.remove(alloc_p) };
        let alloc = unsafe { alloc_p.as_mut() };
        assert!(!alloc.pool_p.is_null());
        let pool_start_addr = alloc.pool().block_pool_p.addr();
        let pool_end_addr = pool_start_addr + (alloc.pool().block_count * BLOCK_SIZE);
        let alloc_start_addr = ptr.addr().get();
        let alloc_end_addr = alloc_p.addr().get() + (alloc.block_count * BLOCK_SIZE);
        assert!(pool_start_addr <= alloc_start_addr);
        assert!(alloc_start_addr <= pool_end_addr);
        assert!(pool_start_addr <= alloc_end_addr);
        assert!(alloc_end_addr <= pool_end_addr);
        assert!(alloc.block_count <= alloc.pool().used_block_count);
        let offset_in_pool = alloc_p.addr().get() - pool_start_addr;
        let block_index = offset_in_pool / BLOCK_SIZE;
        let block_count = alloc.block_count;
        alloc
            .pool_mut()
            .block_bitmap
            .set_bits(block_index, block_count);
        alloc.pool_mut().used_block_count -= alloc.block_count;
        heap.free_block_count += alloc.block_count;
        unsafe {
            alloc_p
                .cast::<u8>()
                .write_bytes(0x6f, alloc.block_count * BLOCK_SIZE)
        };
    }

    /// # Safety
    /// ptr, if it is not None, must point to the same address returned by `malloc` or `malloc_aligned`, and not freed yet.
    unsafe fn realloc(
        mut self: Pin<&mut Self>,
        ptr: Option<NonNull<u8>>,
        new_size: usize,
    ) -> Option<NonNull<u8>> {
        if let Some(ptr) = ptr {
            let mut alloc_p = unsafe { alloc_header_of(ptr.as_ptr()) }.expect("bad pointer");
            let alloc = unsafe { alloc_p.as_mut() };
            let copy_size = if alloc.size == new_size {
                return Some(ptr);
            } else if new_size < alloc.size {
                new_size
            } else {
                alloc.size
            };
            let new_mem = self.as_mut().malloc(new_size)?;
            unsafe { ptr.copy_to(new_mem, copy_size) };
            unsafe { self.as_mut().free(ptr) };
            Some(new_mem)
        } else {
            self.malloc(new_size)
        }
    }
    fn calloc(self: Pin<&mut Self>, size: usize, elements: usize) -> Option<NonNull<u8>> {
        if size != 0 && ((usize::MAX / size) < elements) {
            return None;
        }
        self.malloc(size * elements)
    }

    /// # Safety
    /// ptr, if it is not None, must point to the same address returned by malloc, and not free()d yet.
    unsafe fn realloc_array(
        self: Pin<&mut Self>,
        ptr: Option<NonNull<u8>>,
        new_size: usize,
        new_elements: usize,
    ) -> Option<NonNull<u8>> {
        if new_size != 0 && ((usize::MAX / new_size) < new_elements) {
            return None;
        }
        unsafe { self.realloc(ptr, new_size * new_elements) }
    }

    pub fn expand(
        heap_mutex: &klib::irq::Mutex<HW, Self>,
        pmm: &mut klib::mem::PhysicalMemoryManager<HW>,
        vmm: &mut klib::mem::VirtualMemoryManager<HW>,
    ) -> Result<(), &'static str> {
        let heap_size = pmm.get_total_mem_size();
        let object = klib::mem::vmm::alloc(
            &vmm.kernel_address_space(),
            heap_size,
            klib::mem::vmm::MapFlags {
                read: true,
                write: true,
                ..Default::default()
            },
        )?;
        let (start, size) = {
            let object = object.borrow();
            (object.start, object.size)
        };
        let mut heap_guard = heap_mutex.lock();
        let mut heap = unsafe { Pin::new_unchecked(&mut *heap_guard) };
        unsafe { heap.as_mut().add_mem(start, size) }
    }
}

impl<HW: klib::hal::Hardware> Default for Heap<HW> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::{ptr::addr_of_mut, slice::from_raw_parts};

    #[repr(align(16))]
    struct TestPoolMemory<const N: usize>([u8; N]);

    fn new_heap() -> Pin<alloc::boxed::Box<Heap<klib::hal::TestHW>>> {
        let mut heap = alloc::boxed::Box::<Heap<klib::hal::TestHW>>::new_uninit();
        let p = heap.as_mut_ptr();
        unsafe {
            // We initialize the struct manually, so that we don't attempt to initialize InitialHeapMemory on stack.
            addr_of_mut!((*p).free_block_count).write(0);
            addr_of_mut!((*p).heap_pool_list).write(klib::UnsafeList::default());
            addr_of_mut!((*p).alloc_list).write(klib::UnsafeList::default());
            addr_of_mut!((*p).initial_heap_memory)
                .cast::<u8>()
                .write_bytes(0, size_of::<InitialHeapMemory>());
            addr_of_mut!((*p).initial_heap_initialized).write(false);
            addr_of_mut!((*p)._hw).write(PhantomData);
            addr_of_mut!((*p)._pin).write(PhantomPinned);
            alloc::boxed::Box::into_pin(heap.assume_init())
        }
    }

    #[test]
    fn malloc_returns_aligned_payload_and_free_reclaims_blocks() {
        let mut heap = new_heap();
        let ptr = heap.as_mut().malloc(24).expect("allocation should succeed");

        assert_eq!(ptr.as_ptr() as usize % align_of::<HeapAlignment>(), 0);
        assert!(heap.initial_heap_initialized);
        let allocated_block_count = unsafe { alloc_header_of(ptr.as_ptr()) }
            .map(|header| unsafe { header.as_ref().block_count })
            .expect("returned pointer should identify its allocation header");
        let free_blocks_after_alloc = heap.free_block_count;

        unsafe { heap.as_mut().free(ptr) };

        assert_eq!(
            heap.free_block_count,
            free_blocks_after_alloc + allocated_block_count
        );
        assert_eq!(heap.alloc_list.front(), None);
    }

    #[test]
    fn malloc_aligned_honors_supported_alignment_values() {
        let mut heap = new_heap();

        for alignment in [1, 8, 16, 64, 256, 4096, 65536] {
            let allocation = heap
                .as_mut()
                .malloc_aligned(13, alignment)
                .expect("heap should satisfy requested alignment");
            assert_eq!(allocation.addr().get() % alignment, 0);
            unsafe { heap.as_mut().free(allocation) };
        }
    }

    #[test]
    fn malloc_aligned_rejects_invalid_requests_without_consuming_space() {
        let mut heap = new_heap();
        let free_before = heap.as_ref().free_block_count;

        assert!(heap.as_mut().malloc_aligned(0, 16).is_none());
        assert!(heap.as_mut().malloc_aligned(1, 0).is_none());
        assert!(heap.as_mut().malloc_aligned(1, 3).is_none());
        assert!(heap.as_mut().malloc_aligned(usize::MAX, 16).is_none());
        assert_eq!(heap.as_ref().free_block_count, free_before);
    }

    #[test]
    fn add_mem_rejects_a_pool_too_small_for_metadata_and_data() {
        #[repr(align(16))]
        struct SmallMemory([u8; 16]);

        let mut memory = SmallMemory([0; 16]);
        let mut heap = new_heap();

        let result = unsafe { heap.as_mut().add_mem(memory.0.as_mut_ptr(), memory.0.len()) };

        assert_eq!(result, Err("mem_size is too small"));
        assert_eq!(heap.as_ref().free_block_count, 0);
    }

    #[test]
    fn malloc_reuses_freed_blocks() {
        let mut heap = new_heap();
        let first = heap
            .as_mut()
            .malloc(48)
            .expect("first allocation should succeed");
        let first_addr = first.as_ptr() as usize;
        unsafe { first.as_ptr().write_bytes(0xa5, 48) };
        unsafe { heap.as_mut().free(first) };

        let second = heap
            .as_mut()
            .malloc(48)
            .expect("second allocation should succeed");

        assert_eq!(second.as_ptr() as usize, first_addr);
        assert!(
            unsafe { from_raw_parts(second.as_ptr(), 48) }
                .iter()
                .all(|byte| *byte == 0)
        );
        unsafe { heap.as_mut().free(second) };
    }

    #[test]
    fn simultaneous_allocations_do_not_overlap() {
        let mut heap = new_heap();
        let sizes = [1, 48, 256, 65, 1024];
        let allocations: alloc::vec::Vec<_> = sizes
            .iter()
            .map(|size| {
                heap.as_mut()
                    .malloc(*size)
                    .expect("allocation should succeed")
            })
            .collect();
        let ranges: alloc::vec::Vec<_> = allocations
            .iter()
            .map(|ptr| {
                let header = unsafe { alloc_header_of(ptr.as_ptr()) }.unwrap();
                let start = header.as_ptr() as usize;
                let end = start + unsafe { header.as_ref().block_count } * BLOCK_SIZE;
                start..end
            })
            .collect();

        for i in 0..ranges.len() {
            for j in (i + 1)..ranges.len() {
                assert!(ranges[i].end <= ranges[j].start || ranges[j].end <= ranges[i].start);
            }
        }

        for ptr in allocations {
            unsafe { heap.as_mut().free(ptr) };
        }
    }

    #[test]
    fn malloc_places_poison_bytes_immediately_after_payload() {
        let mut heap = new_heap();
        let size = 37;
        let ptr = heap
            .as_mut()
            .malloc(size)
            .expect("allocation should succeed");
        let poison = unsafe { from_raw_parts(ptr.as_ptr().add(size), POISON_VALUES.len()) };

        assert_eq!(poison, POISON_VALUES);

        unsafe { heap.as_mut().free(ptr) };
    }

    #[test]
    fn malloc_skips_a_free_run_that_is_too_small() {
        let mut heap = new_heap();
        let two_block_size = byte_count_for_block_count(2);
        let a = heap
            .as_mut()
            .malloc(two_block_size)
            .expect("first allocation should succeed");
        let b = heap
            .as_mut()
            .malloc(two_block_size)
            .expect("second allocation should succeed");
        let c = heap
            .as_mut()
            .malloc(two_block_size)
            .expect("third allocation should succeed");
        let c_header = unsafe { alloc_header_of(c.as_ptr()) }.unwrap();
        let c_block_index = (c_header.as_ptr() as usize
            - unsafe { c_header.as_ref().pool().block_pool_p as usize })
            / BLOCK_SIZE;

        unsafe { heap.as_mut().free(b) };
        let three_block_size = byte_count_for_block_count(3);
        let d = heap
            .as_mut()
            .malloc(three_block_size)
            .expect("allocation should find a large enough later run");
        let d_header = unsafe { alloc_header_of(d.as_ptr()) }.unwrap();
        let d_block_index = (d_header.as_ptr() as usize
            - unsafe { d_header.as_ref().pool().block_pool_p as usize })
            / BLOCK_SIZE;

        assert!(d_block_index >= c_block_index + 2);

        unsafe {
            heap.as_mut().free(a);
            heap.as_mut().free(c);
            heap.as_mut().free(d);
        }
    }

    #[test]
    fn malloc_searches_later_pools_when_the_first_one_is_too_small() {
        let mut heap = new_heap();
        let mut small = alloc::boxed::Box::new(TestPoolMemory([0; 32 * 1024]));
        let mut large = alloc::boxed::Box::new(TestPoolMemory([0; 64 * 1024]));
        unsafe {
            heap.as_mut()
                .add_mem(small.0.as_mut_ptr(), size_of_val(&small.0))
                .expect("small test pool should register");
            heap.as_mut()
                .add_mem(large.0.as_mut_ptr(), size_of_val(&large.0))
                .expect("large test pool should register");
        }

        let ptr = heap
            .as_mut()
            .malloc(40 * 1024)
            .expect("allocation should succeed in the larger later pool");

        let large_start = large.0.as_ptr() as usize;
        let large_end = large_start + size_of_val(&large.0);
        assert!((large_start..large_end).contains(&(ptr.as_ptr() as usize)));
        unsafe { heap.as_mut().free(ptr) };
    }

    #[test]
    fn malloc_can_consume_the_entire_free_pool() {
        let mut heap = new_heap();
        let warmup = heap
            .as_mut()
            .malloc(1)
            .expect("warmup allocation should succeed");
        unsafe { heap.as_mut().free(warmup) };

        let pool = heap
            .heap_pool_list
            .front()
            .expect("pool should be initialized");
        let pool_block_count = unsafe { pool.as_ref().block_count };
        let exact_fit_size = byte_count_for_block_count(pool_block_count);

        assert_eq!(
            actual_alloc_size(exact_fit_size),
            pool_block_count * BLOCK_SIZE
        );
        let full_pool = heap
            .as_mut()
            .malloc(exact_fit_size)
            .expect("request exactly matching all free blocks should succeed");
        assert_eq!(heap.free_block_count, 0);
        assert!(heap.as_mut().malloc(1).is_none());

        unsafe { heap.as_mut().free(full_pool) };
        assert_eq!(heap.free_block_count, pool_block_count);
    }

    #[test]
    fn calloc_returns_zeroed_memory() {
        let mut heap = new_heap();
        let ptr = heap
            .as_mut()
            .calloc(7, 19)
            .expect("allocation should succeed");
        let bytes = unsafe { from_raw_parts(ptr.as_ptr(), 7 * 19) };

        assert!(bytes.iter().all(|byte| *byte == 0));

        unsafe { heap.as_mut().free(ptr) };
    }

    #[test]
    fn realloc_preserves_existing_bytes() {
        let mut heap = new_heap();
        let ptr = heap
            .as_mut()
            .malloc(8)
            .expect("initial allocation should succeed");
        unsafe {
            ptr.as_ptr()
                .copy_from_nonoverlapping(b"heaptest".as_ptr(), 8)
        };

        let resized = unsafe { heap.as_mut().realloc(Some(ptr), 32) }
            .expect("growing allocation should succeed");
        let bytes = unsafe { from_raw_parts(resized.as_ptr(), 8) };

        assert_eq!(bytes, b"heaptest");

        unsafe { heap.as_mut().free(resized) };
    }

    #[test]
    fn realloc_same_size_returns_the_original_allocation() {
        let mut heap = new_heap();
        let ptr = heap
            .as_mut()
            .malloc(16)
            .expect("initial allocation should succeed");
        let free_blocks_before = heap.free_block_count;
        let alloc_count_before = unsafe { alloc_header_of(ptr.as_ptr()) }
            .map(|header| unsafe { header.as_ref().pool().used_block_count })
            .expect("allocation header should be valid");

        let same = unsafe { heap.as_mut().realloc(Some(ptr), 16) }
            .expect("same-size realloc should succeed");

        assert_eq!(same, ptr);
        assert_eq!(heap.free_block_count, free_blocks_before);
        let alloc_count_after = unsafe { alloc_header_of(same.as_ptr()) }
            .map(|header| unsafe { header.as_ref().pool().used_block_count })
            .expect("allocation header should remain valid");
        assert_eq!(alloc_count_after, alloc_count_before);
        unsafe { heap.as_mut().free(same) };
    }

    #[test]
    fn realloc_shrinks_and_preserves_the_retained_prefix() {
        let mut heap = new_heap();
        let ptr = heap
            .as_mut()
            .malloc(32)
            .expect("initial allocation should succeed");
        unsafe {
            ptr.as_ptr()
                .copy_from_nonoverlapping(b"0123456789abcdefghijklmnopqrstuv".as_ptr(), 32)
        };

        let resized = unsafe { heap.as_mut().realloc(Some(ptr), 8) }
            .expect("shrinking allocation should succeed");

        assert_eq!(unsafe { from_raw_parts(resized.as_ptr(), 8) }, b"01234567");
        unsafe { heap.as_mut().free(resized) };
    }

    #[test]
    fn realloc_null_pointer_allocates_new_memory() {
        let mut heap = new_heap();

        let ptr = unsafe { heap.as_mut().realloc(None, 24) }
            .expect("realloc with a null pointer should allocate memory");

        assert!(unsafe { alloc_header_of(ptr.as_ptr()) }.is_some());
        unsafe { heap.as_mut().free(ptr) };
    }

    #[test]
    fn realloc_array_resizes_and_preserves_existing_bytes() {
        let mut heap = new_heap();
        let ptr = unsafe { heap.as_mut().realloc_array(None, 4, 2) }
            .expect("initial array allocation should succeed");
        unsafe {
            ptr.as_ptr()
                .copy_from_nonoverlapping(b"abcdefgh".as_ptr(), 8)
        };

        let resized = unsafe { heap.as_mut().realloc_array(Some(ptr), 4, 4) }
            .expect("array growth should succeed");

        assert_eq!(unsafe { from_raw_parts(resized.as_ptr(), 8) }, b"abcdefgh");
        unsafe { heap.as_mut().free(resized) };
    }

    #[test]
    fn failed_realloc_keeps_original_allocation_valid() {
        let mut heap = new_heap();
        let ptr = heap
            .as_mut()
            .malloc(8)
            .expect("initial allocation should succeed");
        unsafe {
            ptr.as_ptr()
                .copy_from_nonoverlapping(b"keepdata".as_ptr(), 8)
        };
        let free_blocks_before = heap.free_block_count;

        assert!(unsafe { heap.as_mut().realloc(Some(ptr), 3 * 1024 * 1024) }.is_none());
        assert_eq!(heap.free_block_count, free_blocks_before);
        assert_eq!(unsafe { from_raw_parts(ptr.as_ptr(), 8) }, b"keepdata");
        assert_eq!(heap.alloc_list.front().unwrap().as_ptr(), unsafe {
            alloc_header_of(ptr.as_ptr()).unwrap().as_ptr()
        });

        unsafe { heap.as_mut().free(ptr) };
    }

    #[test]
    fn realloc_array_rejects_zero_and_overflowing_sizes_without_losing_original() {
        let mut heap = new_heap();
        let ptr = heap
            .as_mut()
            .malloc(8)
            .expect("initial allocation should succeed");
        unsafe {
            ptr.as_ptr()
                .copy_from_nonoverlapping(b"keepdata".as_ptr(), 8)
        };
        let free_blocks_before = heap.free_block_count;

        assert!(unsafe { heap.as_mut().realloc_array(Some(ptr), 4, 0) }.is_none());
        assert!(unsafe { heap.as_mut().realloc_array(Some(ptr), usize::MAX, 2) }.is_none());
        assert_eq!(heap.free_block_count, free_blocks_before);
        assert_eq!(unsafe { from_raw_parts(ptr.as_ptr(), 8) }, b"keepdata");

        unsafe { heap.as_mut().free(ptr) };
    }

    #[test]
    fn zero_sized_and_overflowing_requests_fail() {
        let mut heap = new_heap();

        assert!(heap.as_mut().malloc(0).is_none());
        assert!(heap.as_mut().malloc(usize::MAX).is_none());
        assert!(heap.as_mut().calloc(0, 4).is_none());
        assert!(heap.as_mut().calloc(usize::MAX, 2).is_none());
        assert!(unsafe { heap.as_mut().realloc_array(None, usize::MAX, 2) }.is_none());
        assert!(!heap.initial_heap_initialized);
        assert_eq!(heap.alloc_list.front(), None);
    }
}
