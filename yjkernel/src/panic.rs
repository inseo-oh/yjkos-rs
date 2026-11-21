#[cfg(not(test))]
#[panic_handler]
fn panic(_info: &core::panic::PanicInfo) -> ! {
    use crate::{hal, klib::hal::Hardware, println};

    hal::Hardware::irq_disable();
    println!();
    println!();
    println!("whoops - system panic!()ed");
    println!("> {}", _info.message());
    if let Some(location) = _info.location() {
        println!(
            "source location: {}:{}:{}",
            location.file(),
            location.line(),
            location.column()
        );
    }
    hal::Hardware::stack_trace();
    println!();
    println!("System halted. Please restart the system.");
    println!();
    hal::Hardware::hcf();
}
