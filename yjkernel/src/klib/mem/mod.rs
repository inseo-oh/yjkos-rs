// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

pub(in crate::klib) mod pmm;
pub(crate) mod vmm;

pub(crate) use pmm::PhysicalMemoryManager;
pub(crate) use vmm::{MapFlags, VirtualMemoryManager};
pub(in crate::klib) use vmm::{AddressSpace, AddressSpaceHandle, ObjectHandle};
