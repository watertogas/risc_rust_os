use core::panic::PanicInfo;
use super::exit;
#[panic_handler]
fn panic(info : &PanicInfo) ->!{
    if let Some(location) = info.location() {
        println! (
            "Panicked at {}:{} {}",
            location.file(),
            location.line(),
            info.message().unwrap()
        );
    } else {
        println! ("Panicked at {}", info.message().unwrap());
    }
    exit(1);
    //should never come here
    loop{}
}