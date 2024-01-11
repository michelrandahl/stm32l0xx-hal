#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;

use cortex_m_rt::entry;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPI0A, GPIOB and GPIOC peripherals. This also enables the clock for
    // GPIOA, GPIOB and GPIOC in the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);
    #[cfg(feature = "stm32l0x3")]
    let gpioc = dp.GPIOC.split(&mut rcc);

    #[cfg(not(feature = "stm32l0x3"))]
    let button = gpioa.pa0.into_pull_up_input();
    #[cfg(feature = "stm32l0x3")]
    let button = gpioc.pc13.into_pull_down_input();

    #[cfg(feature = "stm32l0x1")]
    let mut led = gpioa.pa1.into_push_pull_output();
    #[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
    let mut led = gpioa.pa5.into_push_pull_output();

    // Get the delay provider.
    let mut delay = cp.SYST.delay(rcc.clocks);

    loop {
        let wait = match button.is_low() {
            Ok(true) =>
                // hold the current state for one second when button is pressed
                1000.milliseconds(),
            Ok(false) => 100.milliseconds(),
            _ => unreachable!(),
        };
        delay.delay(wait);
        led.toggle().unwrap();
    }
}
