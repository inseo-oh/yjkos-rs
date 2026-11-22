// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use crate::syscall;

const PAGE_SIZE: usize = 4096;

pub fn alloc_objects<T>(
    phys_base: Option<usize>,
    count: usize,
    flags: syscall::VMMapFlags,
) -> Result<*mut T, &'static str> {
    syscall::vm_alloc_object(
        phys_base,
        size_of::<T>()
            .checked_mul(count)
            .ok_or("count is too large")?,
        flags,
    )
    .map(|v| v.cast::<T>())
}
pub fn alloc_object<T>(
    phys_base: Option<usize>,
    flags: syscall::VMMapFlags,
) -> Result<*mut T, &'static str> {
    alloc_objects(phys_base, 1, flags)
}
pub fn alloc_objects_at<T>(
    virt_base: *mut u8,
    phys_base: Option<usize>,
    count: usize,
    flags: syscall::VMMapFlags,
) -> Result<*mut T, &'static str> {
    syscall::vm_alloc_object_at(
        virt_base,
        phys_base,
        size_of::<T>()
            .checked_mul(count)
            .ok_or("count is too large")?,
        flags,
    )
    .map(|v| v.cast::<T>())
}
pub fn alloc_object_at<T>(
    virt_base: *mut u8,
    phys_base: Option<usize>,
    flags: syscall::VMMapFlags,
) -> Result<*mut T, &'static str> {
    alloc_objects_at(virt_base, phys_base, 1, flags)
}
pub fn free<T>(object: *mut T) -> Result<(), &'static str> {
    syscall::vm_free_object(object as *mut u8)
}
pub fn alloc_array_at<T>(
    virt_base: *mut u8,
    count: usize,
    flags: syscall::VMMapFlags,
) -> Result<*mut T, &'static str> {
    alloc_objects_at(virt_base, None, count, flags)
}
pub fn alloc_at<T>(virt_base: *mut u8, flags: syscall::VMMapFlags) -> Result<*mut T, &'static str> {
    alloc_object_at(virt_base, None, flags)
}
pub fn alloc_array<T>(count: usize, flags: syscall::VMMapFlags) -> Result<*mut T, &'static str> {
    alloc_objects(None, count, flags)
}
pub fn alloc<T>(flags: syscall::VMMapFlags) -> Result<*mut T, &'static str> {
    alloc_object(None, flags)
}
pub fn map_array<T>(
    phys_base: usize,
    count: usize,
    flags: syscall::VMMapFlags,
) -> Result<*mut T, &'static str> {
    alloc_objects(Some(phys_base), count, flags)
}
pub fn map<T>(phys_base: usize, flags: syscall::VMMapFlags) -> Result<*mut T, &'static str> {
    alloc_object(Some(phys_base), flags)
}
pub fn map_array_at<T>(
    virt_base: *mut u8,
    phys_base: usize,
    count: usize,
    flags: syscall::VMMapFlags,
) -> Result<*mut T, &'static str> {
    alloc_objects_at(virt_base, Some(phys_base), count, flags)
}
pub fn map_at<T>(
    virt_base: *mut u8,
    phys_base: usize,
    flags: syscall::VMMapFlags,
) -> Result<*mut T, &'static str> {
    alloc_object_at(virt_base, Some(phys_base), flags)
}

/// Returned values are (object pointer - used by free(), pointer to actual data)
pub fn map_unaligned_array<T>(
    phys_base: usize,
    count: usize,
    flags: syscall::VMMapFlags,
) -> Result<(*mut u8, *mut T), &'static str> {
    assert!(phys_base % align_of::<T>() == 0);
    assert!(align_of::<T>() <= PAGE_SIZE);
    let size = count
        .checked_mul(size_of::<T>())
        .ok_or("count is too large")?;
    let offset = phys_base % PAGE_SIZE;
    let page_base = phys_base
        .checked_sub(offset)
        .expect("physical address underflow");
    let actual_size: usize = size.checked_add(offset).ok_or("size too large")?;
    map_array::<u8>(page_base, actual_size, flags)
        .map(|obj| (obj.clone(), obj.map_addr(|a| a + offset).cast::<T>()))
}
/// Returned values are (object pointer - used by free(), pointer to actual data)
pub fn map_unaligned<T>(
    phys_base: usize,
    flags: syscall::VMMapFlags,
) -> Result<(*mut u8, *mut T), &'static str> {
    map_unaligned_array(phys_base, 1, flags)
}
pub fn map_unaligned_array_at<T>(
    virt_base: *mut u8,
    phys_base: usize,
    count: usize,
    flags: syscall::VMMapFlags,
) -> Result<(*mut u8, *mut T), &'static str> {
    assert!(phys_base % align_of::<T>() == 0);
    assert!(align_of::<T>() <= PAGE_SIZE);
    let size = count
        .checked_mul(size_of::<T>())
        .ok_or("count is too large")?;
    let offset = phys_base % PAGE_SIZE;
    let page_base = phys_base
        .checked_sub(offset)
        .expect("physical address underflow");
    let actual_size: usize = size.checked_add(offset).ok_or("size too large")?;
    map_array_at::<u8>(virt_base, page_base, actual_size, flags)
        .map(|obj| (obj.clone(), obj.map_addr(|a| a + offset).cast::<T>()))
}
pub fn map_unaligned_at<T>(
    virt_base: *mut u8,
    phys_base: usize,
    flags: syscall::VMMapFlags,
) -> Result<(*mut u8, *mut T), &'static str> {
    map_unaligned_array_at(virt_base, phys_base, 1, flags)
}
