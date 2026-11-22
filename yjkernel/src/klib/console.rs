// Copyright (c) 2026, Oh Inseo
// SPDX-License-Identifier: BSD-2-Clause
// See LICENSE for details.

use core::{
    fmt, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

pub(crate) type ConsoleWriter = fn(fmt::Arguments<'_>) -> fmt::Result;
pub(crate) type ConsoleReader = fn() -> char;

static WRITER: AtomicPtr<ConsoleWriter> = AtomicPtr::new(ptr::null_mut());
static READER: AtomicPtr<ConsoleReader> = AtomicPtr::new(ptr::null_mut());

pub(crate) fn set_writer(writer: &'static ConsoleWriter) {
    WRITER.store(
        writer as *const ConsoleWriter as *mut ConsoleWriter,
        Ordering::Release,
    );
}
pub(crate) fn set_reader(reader: &'static ConsoleReader) {
    READER.store(
        reader as *const ConsoleReader as *mut ConsoleReader,
        Ordering::Release,
    );
}

pub(crate) fn write(args: fmt::Arguments<'_>) -> fmt::Result {
    let writer = WRITER.load(Ordering::Acquire);
    if writer.is_null() {
        return Err(fmt::Error);
    }
    // SAFETY: `set_writer` only stores pointers to `'static` writer values.
    unsafe { (*writer)(args) }
}
pub(in crate::klib) fn read() -> Result<char, &'static str> {
    let reader = READER.load(Ordering::Acquire);
    if reader.is_null() {
        return Err("no reader available");
    }
    // SAFETY: `set_reader` only stores pointers to `'static` reader values.
    Ok(unsafe { (*reader)() })
}
