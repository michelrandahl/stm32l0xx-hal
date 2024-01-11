#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);

    #[cfg(feature = "stm32l0x1")]
    let mut led = gpioa.pa1.into_push_pull_output();
    #[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
    let mut led = gpioa.pa5.into_push_pull_output();

    loop {
        // Set the LED high one million times in a row.
        for _ in 0..1_000_000 {
            led.set_high().unwrap();
        }

        // Set the LED low one million times in a row.
        for _ in 0..1_000_000 {
            led.set_low().unwrap();
        }
    }
}
