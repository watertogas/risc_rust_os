use crate::sbi::shutdown;
use core::panic::PanicInfo;
#[panic_handler]
fn panic(info : &PanicInfo) ->!{
    if let Some(location) = info.location() {
        println! (
            "[Kernel]Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message(),
        );
    } else {
        println! ("[Kernel]Panicked at {}", info.message());
    }
    shutdown(true);
}