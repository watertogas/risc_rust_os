use core::panic::PanicInfo;
use super::exit;
#[panic_handler]
fn panic(info : &PanicInfo) ->!{
    if let Some(location) = info.location() {
        println! (
            "[User]Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message()
        );
    } else {
        println! ("[User]Panicked at {}", info.message());
    }
    exit(1);
    //should never come here
    loop{}
}