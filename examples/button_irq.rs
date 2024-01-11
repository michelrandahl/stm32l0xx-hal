#![no_main]
#![no_std]

extern crate panic_halt;

use core::cell::RefCell;
use core::ops::DerefMut;

use cortex_m::asm;
use cortex_m::interrupt::Mutex;
use cortex_m::peripheral::NVIC;
use cortex_m_rt::entry;
use stm32l0xx_hal::{
    exti::{Exti, ExtiLine, GpioLine, TriggerEdge},
    gpio::*,
    pac::{self, interrupt, Interrupt},
    prelude::*,
    rcc::Config,
    syscfg::SYSCFG,
};


#[cfg(not(feature = "stm32l0x3"))]
static LED: Mutex<RefCell<Option<gpiob::PB6<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));
#[cfg(any(feature = "stm32l0x3"))]
static LED: Mutex<RefCell<Option<gpioa::PA5<Output<PushPull>>>>> = Mutex::new(RefCell::new(None));

#[entry]
fn main() -> ! {
    let dp = pac::Peripherals::take().unwrap();

    // Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    // Acquire the GPIOB peripheral. This also enables the clock for GPIOB in
    // the RCC register.
    let gpioa = dp.GPIOA.split(&mut rcc);
    #[cfg(not(feature = "stm32l0x3"))]
    let gpiob = dp.GPIOB.split(&mut rcc);
    #[cfg(any(feature = "stm32l0x3"))]
    let gpioc = dp.GPIOC.split(&mut rcc);


    #[cfg(feature = "stm32l0x1")]
    let mut led = gpioa.pa1.into_push_pull_output();
    #[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
    let mut led = gpioa.pa5.into_push_pull_output();


    #[cfg(not(feature = "stm32l0x3"))]
    let button = gpiob.pb2.into_pull_up_input();
    #[cfg(any(feature = "stm32l0x3"))]
    let button = gpioc.pc13.into_pull_down_input();


    let mut syscfg = SYSCFG::new(dp.SYSCFG, &mut rcc);
    let mut exti = Exti::new(dp.EXTI);

    // Configure the external interrupt on the falling edge for the pin 0.
    let line = GpioLine::from_raw_line(button.pin_number()).unwrap();
    exti.listen_gpio(&mut syscfg, button.port(), line, TriggerEdge::Falling);

    // Store the external interrupt and LED in mutex reffcells to make them
    // available from the interrupt.
    cortex_m::interrupt::free(|cs| {
        *LED.borrow(cs).borrow_mut() = Some(led);
    });

    // Enable the external interrupt in the NVIC.
    #[cfg(not(feature = "stm32l0x3"))]
    unsafe { NVIC::unmask(Interrupt::EXTI2_3); }
    #[cfg(feature = "stm32l0x3")]
    unsafe { NVIC::unmask(Interrupt::EXTI4_15); }

    loop {
        asm::wfi();
    }
}

#[cfg(not(feature = "stm32l0x3"))]
#[interrupt]
fn EXTI2_3() {
    // Keep the LED state.
    static mut STATE: bool = false;

    cortex_m::interrupt::free(|cs| {
        // Clear the interrupt flag.
        Exti::unpend(GpioLine::from_raw_line(2).unwrap());

        // Change the LED state on each interrupt.
        if let Some(ref mut led) = LED.borrow(cs).borrow_mut().deref_mut() {
            if *STATE {
                led.set_low().unwrap();
                *STATE = false;
            } else {
                led.set_high().unwrap();
                *STATE = true;
            }
        }
    });
}

#[cfg(feature = "stm32l0x3")]
#[interrupt]
fn EXTI4_15() {
    // Keep the LED state.
    static mut STATE: bool = false;

    cortex_m::interrupt::free(|cs| {
        // Clear the interrupt flag.
        Exti::unpend(GpioLine::from_raw_line(13).unwrap());

        // Change the LED state on each interrupt.
        if let Some(ref mut led) = LED.borrow(cs).borrow_mut().deref_mut() {
            if *STATE {
                led.set_low().unwrap();
                *STATE = false;
            } else {
                led.set_high().unwrap();
                *STATE = true;
            }
        }
    });
}
