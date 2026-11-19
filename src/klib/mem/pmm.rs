use crate::klib;
use alloc::vec::Vec;
use core::marker::PhantomData;
pub struct AllocatedPageInfo {
    pub base: klib::PhysicalPtr,
    pub page_count: usize,
}
pub(in crate::klib) struct PagePool<HW> {
    bitmap: Vec<klib::bitmap::Word>,
    pub(in crate::klib) base_addr: klib::PhysicalPtr,
    pub(in crate::klib) page_count: usize,
    _hw: PhantomData<HW>,
}
impl<HW: klib::hal::Hardware> PagePool<HW> {
    const fn block_size_to_page_pool_level(&self, size: usize) -> usize {
        let mut size_per_block = self.page_count;
        let mut current_level = 0;
        while size < size_per_block {
            current_level += 1;
            size_per_block /= 2;
        }
        current_level
    }
    fn malloc(&mut self, mut page_count: usize) -> Option<AllocatedPageInfo> {
        assert_ne!(page_count, 0);
        // Adjust page size to nearest 2^n
        let mut block_size = 1;
        while block_size < page_count {
            if (usize::MAX / 2) < block_size {
                return None;
            }
            block_size *= 2;
        }
        if self.page_count < block_size {
            return None;
        }
        page_count = block_size;
        let wanted_level = self.block_size_to_page_pool_level(block_size);
        let mut found_level = wanted_level;
        let found_block_index;
        loop {
            let (bit_start, bit_end) = bit_indices_range_for_level(found_level);
            let found_at = klib::Bitmap(&mut self.bitmap[..]).find_set_bits(bit_start, 1);
            if let Some(found_at) = found_at
                && found_at <= bit_end
            {
                // Found block
                found_block_index = found_at - bit_start;
                break;
            }
            if found_level == 0 {
                return None;
            }
            found_level -= 1;
        }

        // If we found block at lower level, split blocks until we reach there.
        let mut current_level = found_level;
        let mut current_block_index = found_block_index;
        while current_level < wanted_level {
            // Mark current one as unavilable ==================================
            let bit_index = bit_index_for_page_pool_block(current_level, current_block_index);
            klib::Bitmap(&mut self.bitmap[..]).clear_bit(bit_index);
            // Mark upper block's second block as available. ===================
            // (First block will be marked as unavailable next time, so don't touch that)
            let bit_index =
                bit_index_for_page_pool_block(current_level + 1, (current_block_index * 2) + 1);
            klib::Bitmap(&mut self.bitmap[..]).set_bit(bit_index);
            current_level += 1;
            current_block_index *= 2;
        }
        // Mark resulting block as unavailable =================================
        let bit_index = bit_index_for_page_pool_block(wanted_level, current_block_index);
        klib::Bitmap(&mut self.bitmap[..]).clear_bit(bit_index);
        Some(AllocatedPageInfo {
            base: self
                .base_addr
                .checked_add(block_size * current_block_index * HW::PAGE_SIZE)
                .expect("allocation address overflow"),
            page_count,
        })
    }
    fn free(&mut self, info: AllocatedPageInfo) {
        assert!(info.page_count > 0);
        assert!(info.page_count.is_power_of_two());
        assert!(info.page_count <= self.page_count);
        let offset = info
            .base
            .checked_distance_from(self.base_addr)
            .expect("allocation precedes pool");
        let pool_size = self.page_count.checked_mul(HW::PAGE_SIZE).unwrap();
        let alloc_size = info.page_count.checked_mul(HW::PAGE_SIZE).unwrap();
        assert!(offset.is_multiple_of(alloc_size));
        assert!(offset <= pool_size);
        assert!(alloc_size <= pool_size - offset);
        let block_size = info.page_count;
        let mut current_block_index = offset / alloc_size;
        let mut current_level = self.block_size_to_page_pool_level(block_size);
        assert!(block_size <= self.page_count);

        {
            let mut level = current_level;
            let mut index = current_block_index;
            while level > 0 {
                level -= 1;
                index /= 2;
                let parent_bit = bit_index_for_page_pool_block(level, index);
                assert!(
                    !klib::Bitmap(&mut self.bitmap[..]).is_bit_set(parent_bit),
                    "block is already free in an ancestor"
                );
            }
        }
        loop {
            // Mark it as available ============================================
            let bit_index = bit_index_for_page_pool_block(current_level, current_block_index);
            assert!(!klib::Bitmap(&mut self.bitmap[..]).is_bit_set(bit_index));

            klib::Bitmap(&mut self.bitmap[..]).set_bit(bit_index);
            if current_level == 0 {
                // No lower levels
                break;
            }
            // See if neighbor block is also available =========================
            let neighbor_bit_index = if current_block_index.is_multiple_of(2) {
                bit_index + 1
            } else {
                bit_index - 1
            };
            if !klib::Bitmap(&mut self.bitmap[..]).is_bit_set(neighbor_bit_index) {
                // Neighbor is in use; No further action is needed.
                break;
            }
            // Combine with neighbor block, and move to lower level. ===========
            klib::Bitmap(&mut self.bitmap[..]).clear_bit(bit_index);
            klib::Bitmap(&mut self.bitmap[..]).clear_bit(neighbor_bit_index);
            current_level -= 1;
            current_block_index /= 2;
        }
    }
}

struct PagePoolSizes {
    page_count: usize,
    level_count: usize,
    bit_count: usize,
}
const fn calculate_page_pool_sizes(page_count: usize) -> PagePoolSizes {
    assert!(page_count > 0);
    let mut current_page_count = page_count;
    let mut result_page_count = 1;
    let mut level_count = 1;
    let mut bit_count = 1;
    let mut blocks = 1;
    while 1 < current_page_count {
        blocks *= 2;
        bit_count += blocks;
        current_page_count /= 2;
        result_page_count *= 2;
        level_count += 1;
    }
    PagePoolSizes {
        page_count: result_page_count,
        level_count,
        bit_count,
    }
}
const fn bit_indices_range_for_level(level: usize) -> (usize, usize) {
    let mut current_level = 0;
    let mut block_in_level = 1;
    let mut bit_offset = 0;
    while current_level < level {
        bit_offset += block_in_level;
        block_in_level *= 2;
        current_level += 1;
    }
    (bit_offset, bit_offset + block_in_level - 1)
}
const fn bit_index_for_page_pool_block(level: usize, block: usize) -> usize {
    let (start, end) = bit_indices_range_for_level(level);
    let blocks_in_level = end - start + 1;
    assert!(block < blocks_in_level);
    start + block
}

// Physical memory management is done using buddy allocation algorithm.
// There are several levels in metadata area, and each level corresponds to
// specific allocation size, which is also size of the block.
//
// If there are N pages:
// -    Level 0: Each block consists of N   pages, and the level has 1 page.
// -    Level 1: Each block consists of N/2 pages, and the level has 2 pages.
// -    Level 2: Each block consists of N/4 pages, and the level has 4 pages.
// - ...
// - Last level: Each block consists of 1   page,  and the level has N pages.
//
// Note that each level's single block at index M is the same as next level's
// two blocks at index (M * 2). In other words:
// Level 0 | 00000000 |
// Level 1 | 00001111 |
// Level 2 | 00112233 |
// Level 3 | 01234567 |
// ...
//
// Each block is either available(1) or not(0), and this info is stored in bitmap in the metadata area.
// Initially blocks at all levels are unavailable, except for the very first level.
// When allocating blocks, it first calculates right level for the given size, and then
// looks for the suitable block in that level.
// If it wasn't found, it keeps decreasing level until something is found. Then it splits blocks:
// 1. Mark the block as unavailable
// 2. Move to the next level, and mark corresponding blocks as available.
// 3. If we haven't reached the level for given size, go to 1.
//    (In this case we will use of the blocks we just marked as available above)
// 4. Return one of the blocks we made above(by marking it as unavilable).
//
// Deallocating works in reverse(who's surprised?).
// 1. Mark the block as available.
// 2. If neighbor block is also available, mark both blocks as unavailable, decrease level, and mark
//    the corresponding block as available.
// 3. Repeat 2 until neighbor is not marked as available, or there is no neighbor(i.e. The first level).
pub struct PhysicalMemoryManager<HW: klib::hal::Hardware> {
    pub(in crate::klib) pools: Vec<PagePool<HW>>,
}
impl<HW: klib::hal::Hardware> PhysicalMemoryManager<HW> {
    pub const fn new() -> Self {
        Self { pools: Vec::new() }
    }

    /// # Safety
    /// Caller must guarantee that memory pointed by base and page_count:
    /// - Points to non-reserved physical RAM, and not used by anyone else.
    /// - Must not overlap with existing registered pages.
    /// - Must be aligned to page boundary.
    pub unsafe fn register_mem(
        &mut self,
        base: klib::PhysicalPtr,
        page_count: usize,
    ) -> Result<(), &'static str> {
        assert!(!base.is_null());
        assert!(page_count != 0);
        assert_eq!(base.align_offset(HW::PAGE_SIZE), 0);

        // Build all metadata before changing the manager, so allocation failure
        // cannot leave only part of this physical range registered.
        let mut new_pools = Vec::new();
        let mut current_base_addr = base;
        let mut remaining_page_count = page_count;
        while remaining_page_count != 0 {
            let page_pool_sizes = calculate_page_pool_sizes(remaining_page_count);
            let word_count = klib::bitmap::needed_word_count(page_pool_sizes.bit_count);
            let mut bitmap = Vec::new();
            bitmap
                .try_reserve_exact(word_count)
                .map_err(|_| "unable to allocate metadata memory")?;
            bitmap.resize(word_count, 0);
            klib::Bitmap(&mut bitmap[..]).set_bit(0);

            new_pools
                .try_reserve(1)
                .map_err(|_| "unable to allocate metadata memory")?;
            new_pools.push(PagePool {
                bitmap,
                base_addr: current_base_addr,
                page_count: page_pool_sizes.page_count,
                _hw: PhantomData,
            });

            let pool_size = page_pool_sizes
                .page_count
                .checked_mul(HW::PAGE_SIZE)
                .ok_or("physical memory range too large")?;
            current_base_addr = current_base_addr
                .checked_add(pool_size)
                .ok_or("physical memory range too large")?;
            remaining_page_count -= page_pool_sizes.page_count;
        }

        self.pools
            .try_reserve(new_pools.len())
            .map_err(|_| "unable to allocate metadata memory")?;
        self.pools.extend(new_pools);
        Ok(())
    }

    pub fn malloc(&mut self, page_count: usize) -> Result<AllocatedPageInfo, &'static str> {
        if page_count == 0 {
            return Err("page count must be nonzero");
        }
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        for pool in &mut self.pools {
            if let Some(allocation) = pool.malloc(page_count) {
                return Ok(allocation);
            }
        }
        Err("out of physical memory")
    }

    /// # Safety
    /// Caller must guarantee that memory pointed by alloc points to allocation returned by malloc, and not free()d yet.ß
    pub unsafe fn free(&mut self, alloc: AllocatedPageInfo) {
        assert!(!alloc.base.is_null());
        assert!(alloc.page_count != 0);
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        for pool in &mut self.pools {
            let pool_data_start = pool.base_addr;
            let pool_data_end = pool_data_start
                .checked_add(HW::PAGE_SIZE.checked_mul(pool.page_count).unwrap() - 1)
                .unwrap();
            if (alloc.base < pool_data_start) || (pool_data_end < alloc.base) {
                continue;
            }
            let pool_size = pool.page_count.checked_mul(HW::PAGE_SIZE).unwrap();
            let alloc_size = alloc.page_count.checked_mul(HW::PAGE_SIZE).unwrap();
            let offset = alloc
                .base
                .checked_distance_from(pool.base_addr)
                .expect("invalid free address");
            assert!(offset < pool_size);
            assert!(alloc_size <= pool_size - offset);
            pool.free(alloc);
            return;
        }
        panic!("physical page allocation does not belong to any pool");
    }
    pub fn get_total_mem_size(&self) -> usize {
        let _irq_disable = klib::irq::DisableGuard::<HW>::lock();
        let page_count = self.pools.iter().fold(0usize, |total, pool| {
            total
                .checked_add(pool.page_count)
                .expect("page count overflow")
        });
        page_count
            .checked_mul(HW::PAGE_SIZE)
            .expect("physical memory size overflow")
    }
}

impl<HW: klib::hal::Hardware> Default for PhysicalMemoryManager<HW> {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE_ADDR: usize = 0x10_0000;
    const BASE_PTR: klib::PhysicalPtr = klib::PhysicalPtr::new(BASE_ADDR);

    fn new_pool(page_count: usize) -> PagePool<klib::hal::TestHW> {
        new_pool_at(page_count, BASE_PTR)
    }

    fn new_pool_at(page_count: usize, base_addr: klib::PhysicalPtr) -> PagePool<klib::hal::TestHW> {
        let sizes = calculate_page_pool_sizes(page_count);
        let word_count = klib::bitmap::needed_word_count(sizes.bit_count);
        let mut bitmap = alloc::vec![0; word_count];
        klib::Bitmap(&mut bitmap[..]).set_bit(0);
        PagePool {
            bitmap,
            base_addr,
            page_count: sizes.page_count,
            _hw: PhantomData,
        }
    }

    #[test]
    fn malloc_rounds_up_to_a_power_of_two_pages() {
        let mut pool = new_pool(8);

        let allocation = pool.malloc(3).expect("pool should have enough pages");

        assert_eq!(allocation.base, BASE_PTR);
        assert_eq!(allocation.page_count, 4);
        pool.free(allocation);
        assert_eq!(pool.malloc(8).unwrap().page_count, 8);
    }

    #[test]
    fn freeing_all_single_pages_coalesces_the_whole_pool() {
        let mut pool = new_pool(8);
        let mut allocations = alloc::vec::Vec::new();

        for page in 0..8 {
            let allocation = pool.malloc(1).expect("all eight pages should be available");
            assert_eq!(allocation.page_count, 1);
            assert_eq!(
                allocation.base,
                klib::PhysicalPtr::new(
                    BASE_ADDR + page * <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE,
                )
            );
            allocations.push(allocation);
        }
        assert!(pool.malloc(1).is_none());

        for allocation in allocations {
            pool.free(allocation);
        }

        let allocation = pool.malloc(8).expect("freed pages should coalesce");
        assert_eq!(allocation.base, BASE_PTR);
        assert_eq!(allocation.page_count, 8);
    }

    #[test]
    fn allocation_does_not_cross_pool_bounds() {
        let mut pool = new_pool(4);

        assert!(pool.malloc(5).is_none());
        assert_eq!(pool.malloc(4).unwrap().page_count, 4);
    }

    #[test]
    fn allocation_handles_largest_request_without_overflowing() {
        let mut pool = new_pool(8);

        assert!(pool.malloc(usize::MAX).is_none());
        assert_eq!(pool.malloc(8).unwrap().page_count, 8);
    }

    #[test]
    fn fragmentation_prevents_a_large_allocation_until_neighbors_are_freed() {
        let mut pool = new_pool(4);
        let allocations: alloc::vec::Vec<_> = (0..4).map(|_| pool.malloc(1).unwrap()).collect();

        pool.free(AllocatedPageInfo {
            base: allocations[0].base,
            page_count: allocations[0].page_count,
        });
        pool.free(AllocatedPageInfo {
            base: allocations[2].base,
            page_count: allocations[2].page_count,
        });
        assert!(pool.malloc(2).is_none());

        pool.free(AllocatedPageInfo {
            base: allocations[1].base,
            page_count: allocations[1].page_count,
        });
        let allocation = pool.malloc(2).expect("adjacent free pages should coalesce");
        assert_eq!(allocation.base, BASE_PTR);
        assert_eq!(allocation.page_count, 2);
    }

    #[test]
    #[should_panic]
    fn freeing_the_same_allocation_twice_panics() {
        let mut pool = new_pool(2);
        let allocation = pool.malloc(1).unwrap();
        let duplicate = AllocatedPageInfo {
            base: allocation.base,
            page_count: allocation.page_count,
        };
        pool.free(allocation);
        pool.free(duplicate);
    }

    #[test]
    #[should_panic]
    fn zero_page_allocation_panics() {
        let mut pool = new_pool(2);
        pool.malloc(0);
    }

    #[test]
    #[should_panic]
    fn freeing_a_non_page_aligned_address_panics() {
        let mut pool = new_pool(2);
        pool.free(AllocatedPageInfo {
            base: klib::PhysicalPtr::new(BASE_ADDR + 1),
            page_count: 1,
        });
    }

    #[test]
    #[should_panic]
    fn freeing_a_block_outside_the_pool_panics() {
        let mut pool = new_pool(2);
        pool.free(AllocatedPageInfo {
            base: klib::PhysicalPtr::new(
                BASE_ADDR + 2 * <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE,
            ),
            page_count: 1,
        });
    }

    #[test]
    fn pool_size_calculation_rounds_down_to_largest_power_of_two() {
        let sizes = calculate_page_pool_sizes(13);

        assert_eq!(sizes.page_count, 8);
        assert_eq!(sizes.level_count, 4);
        assert_eq!(sizes.bit_count, 15);
    }

    #[test]
    fn pool_size_calculation_covers_power_of_two_boundaries() {
        for (requested, expected) in [
            (1, 1),
            (2, 2),
            (3, 2),
            (7, 4),
            (8, 8),
            (9, 8),
            (15, 8),
            (16, 16),
            (17, 16),
        ] {
            let sizes = calculate_page_pool_sizes(requested);
            assert_eq!(sizes.page_count, expected, "requested {requested} pages");
            assert_eq!(sizes.bit_count, (expected * 2) - 1);
            assert_eq!(sizes.level_count, expected.ilog2() as usize + 1);
        }
    }

    #[test]
    fn buddy_tree_levels_and_block_indices_are_contiguous() {
        for (level, expected_range) in [
            (0, (0, 0)),
            (1, (1, 2)),
            (2, (3, 6)),
            (3, (7, 14)),
            (4, (15, 30)),
        ] {
            let range = bit_indices_range_for_level(level);
            assert_eq!(range, expected_range);
            assert_eq!(bit_index_for_page_pool_block(level, 0), range.0);
            assert_eq!(
                bit_index_for_page_pool_block(level, range.1 - range.0),
                range.1
            );
        }
    }

    #[test]
    fn pool_level_matches_power_of_two_block_sizes() {
        let pool = new_pool(16);

        for (block_size, expected_level) in [(16, 0), (8, 1), (4, 2), (2, 3), (1, 4)] {
            assert_eq!(
                pool.block_size_to_page_pool_level(block_size),
                expected_level
            );
        }
    }

    #[test]
    #[should_panic]
    fn pool_size_calculation_rejects_zero_pages() {
        let _ = calculate_page_pool_sizes(0);
    }

    #[test]
    #[should_panic]
    fn buddy_block_index_rejects_a_block_outside_its_level() {
        let _ = bit_index_for_page_pool_block(2, 4);
    }

    #[test]
    #[should_panic]
    fn freeing_a_non_power_of_two_block_panics() {
        let mut pool = new_pool(4);
        pool.free(AllocatedPageInfo {
            base: BASE_PTR,
            page_count: 3,
        });
    }

    #[test]
    fn differently_sized_allocations_do_not_overlap() {
        let mut pool = new_pool(16);
        let allocations = [
            pool.malloc(3).unwrap(),
            pool.malloc(2).unwrap(),
            pool.malloc(5).unwrap(),
            pool.malloc(1).unwrap(),
            pool.malloc(1).unwrap(),
        ];

        assert_eq!(
            allocations
                .iter()
                .map(|allocation| allocation.page_count)
                .collect::<alloc::vec::Vec<_>>(),
            alloc::vec![4, 2, 8, 1, 1]
        );
        for (index, allocation) in allocations.iter().enumerate() {
            let start = allocation.base.addr();
            let end = start
                + allocation.page_count * <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE;
            for other in &allocations[index + 1..] {
                let other_start = other.base.addr();
                let other_end = other_start
                    + other.page_count * <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE;
                assert!(end <= other_start || other_end <= start);
            }
        }
        assert!(pool.malloc(1).is_none());
    }

    #[test]
    fn freeing_in_arbitrary_order_coalesces_the_pool() {
        let mut pool = new_pool(8);
        let mut allocations: alloc::vec::Vec<_> = (0..8).map(|_| pool.malloc(1).unwrap()).collect();

        for index in [3, 6, 1, 7, 0, 5, 2, 4] {
            let target_base =
                BASE_ADDR + index * <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE;
            let position = allocations
                .iter()
                .position(|allocation| allocation.base.addr() == target_base)
                .unwrap();
            pool.free(allocations.swap_remove(position));
        }

        let allocation = pool
            .malloc(8)
            .expect("all pages should merge regardless of free order");
        assert_eq!(allocation.base, BASE_PTR);
        assert_eq!(allocation.page_count, 8);
    }

    #[test]
    fn manager_searches_multiple_pools_and_frees_to_the_matching_pool() {
        let second_base = klib::PhysicalPtr::new(BASE_ADDR + 0x10_000);
        let mut first_pool = new_pool_at(2, BASE_PTR);
        let mut second_pool = new_pool_at(4, second_base);

        // Exhaust the first pool so the manager must continue to the second.
        let first_pool_allocation = first_pool.malloc(2).unwrap();
        let mut manager = PhysicalMemoryManager::new();
        manager.pools.push(first_pool);
        manager.pools.push(second_pool);

        assert_eq!(
            manager.get_total_mem_size(),
            6 * <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE
        );
        let allocation = manager
            .malloc(1)
            .expect("second pool should have free pages");
        assert_eq!(allocation.base, second_base);

        unsafe { manager.free(allocation) };
        assert_eq!(manager.pools[1].malloc(4).unwrap().base, second_base);
        manager.pools[0].free(first_pool_allocation);
    }

    #[test]
    #[should_panic]
    fn manager_rejects_freeing_an_address_outside_registered_pools() {
        let mut manager = PhysicalMemoryManager::new();
        manager.pools.push(new_pool(2));

        unsafe {
            manager.free(AllocatedPageInfo {
                base: klib::PhysicalPtr::new(BASE_ADDR + 0x10_000),
                page_count: 1,
            })
        };
    }

    #[test]
    fn empty_manager_reports_no_memory_and_cannot_allocate() {
        let mut manager = PhysicalMemoryManager::<klib::hal::TestHW>::new();

        assert_eq!(manager.get_total_mem_size(), 0);
        assert!(manager.malloc(1).is_err());
    }

    #[test]
    fn register_mem_splits_ranges_into_power_of_two_pools() {
        let mut manager = PhysicalMemoryManager::<klib::hal::TestHW>::new();
        let page_size = <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE;

        unsafe { manager.register_mem(BASE_PTR, 13) }.expect("memory should register");

        assert_eq!(
            manager
                .pools
                .iter()
                .map(|pool| pool.page_count)
                .collect::<alloc::vec::Vec<_>>(),
            alloc::vec![8, 4, 1]
        );
        assert_eq!(
            manager
                .pools
                .iter()
                .map(|pool| pool.base_addr)
                .collect::<alloc::vec::Vec<_>>(),
            alloc::vec![
                BASE_PTR,
                klib::PhysicalPtr::new(BASE_ADDR + 8 * page_size),
                klib::PhysicalPtr::new(BASE_ADDR + 12 * page_size)
            ]
        );
        assert_eq!(manager.get_total_mem_size(), 13 * page_size);

        let allocations = [
            manager.malloc(8).expect("first pool should be allocatable"),
            manager
                .malloc(4)
                .expect("second pool should be allocatable"),
            manager.malloc(1).expect("last pool should be allocatable"),
        ];
        assert_eq!(
            allocations
                .iter()
                .map(|allocation| allocation.page_count)
                .collect::<alloc::vec::Vec<_>>(),
            alloc::vec![8, 4, 1]
        );
        assert_eq!(allocations[0].base, BASE_PTR);
        assert_eq!(
            allocations[1].base,
            klib::PhysicalPtr::new(BASE_ADDR + 8 * page_size)
        );
        assert_eq!(
            allocations[2].base,
            klib::PhysicalPtr::new(BASE_ADDR + 12 * page_size)
        );
        assert!(manager.malloc(1).is_err());
    }

    #[test]
    fn register_mem_overflow_leaves_manager_unchanged() {
        let mut manager = PhysicalMemoryManager::<klib::hal::TestHW>::new();
        let page_size = <klib::hal::TestHW as klib::hal::Hardware>::PAGE_SIZE;
        let base = usize::MAX - (page_size - 1);

        assert_eq!(
            unsafe { manager.register_mem(klib::PhysicalPtr::new(base), 2) },
            Err("physical memory range too large")
        );
        assert!(manager.pools.is_empty());
        assert_eq!(manager.get_total_mem_size(), 0);
        assert!(manager.malloc(1).is_err());
    }
}
