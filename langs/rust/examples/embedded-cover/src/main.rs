#![no_main]
#![no_std]

use cortex_m_semihosting::debug;

use defmt_rtt as _; // global logger

use mantra_rust_macros::req;
use xmc4_hal as _;

use panic_probe as _;

// same panicking *behavior* as `panic-probe` but doesn't print a panic message
// this prevents the panic message being printed *twice* when `defmt::panic` is invoked
#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

/// Terminates the application and makes a semihosting-capable debug tool exit
/// with status code 0.
pub fn exit() -> ! {
    loop {
        debug::exit(debug::EXIT_SUCCESS);
    }
}

/// Hardfault handler.
///
/// Terminates the application and makes a semihosting-capable debug tool exit
/// with an error. This seems better than the default, which is to spin in a
/// loop.
#[cortex_m_rt::exception]
unsafe fn HardFault(_frame: &cortex_m_rt::ExceptionFrame) -> ! {
    loop {
        debug::exit(debug::EXIT_FAILURE);
    }
}

#[req(123)]
fn hit(o: i32) {
    defmt::info!("hitted! {}", o);
}

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("Hello reqcov!");

    hit(0);

    let mut o = 1;

    for _ in 0..400 {
        for _ in 0..2000 {
            let y = 1;
            o += y;
            o *= y;
        }

        hit(o);
    }

    self::exit()
}
