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

    // Acquire the GPI0A and GPIOC peripherals. This also enables the clock for
    // GPIOA and GPIOC in the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);
    let gpioc = dp.GPIOC.split(&mut rcc);

    // Configure PC13 as input.
    let button = gpioc.pc13.into_pull_down_input();

    // Configure PA5 as output.
    let mut led = gpioa.pa5.into_push_pull_output();

    // Get the delay provider.
    let mut delay = cp.SYST.delay(rcc.clocks);

    let debounce_duration = 50_u32;

    loop {
        if button.is_low().unwrap() {
            // Wait for debounce duration.
            delay.delay_ms(debounce_duration);

            // Check if button is still pressed.
            if button.is_low().unwrap() {
                led.toggle().unwrap();

                // Wait until button is released to avoid repeated toggling.
                while button.is_low().unwrap() {}
            }
        }
    }
}
