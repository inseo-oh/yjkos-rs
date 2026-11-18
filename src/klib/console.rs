use core::{
    fmt, ptr,
    sync::atomic::{AtomicPtr, Ordering},
};

pub type ConsoleWriter = fn(fmt::Arguments<'_>) -> fmt::Result;

static WRITER: AtomicPtr<ConsoleWriter> = AtomicPtr::new(ptr::null_mut());

pub fn set_writer(writer: &'static ConsoleWriter) {
    WRITER.store(
        writer as *const ConsoleWriter as *mut ConsoleWriter,
        Ordering::Release,
    );
}

pub fn write(args: fmt::Arguments<'_>) -> fmt::Result {
    let writer = WRITER.load(Ordering::Acquire);
    if writer.is_null() {
        return Err(fmt::Error);
    }
    // SAFETY: `set_writer` only stores pointers to `'static` writer values.
    unsafe { (*writer)(args) }
}
