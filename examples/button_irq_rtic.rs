#![deny(warnings)]
#![deny(unsafe_code)]
#![no_main]
#![no_std]

use panic_halt as _;
use rtic::app;

#[app(device = stm32l0xx_hal::pac, peripherals = true)]
mod app {
    use stm32l0xx_hal::{
        exti::{Exti, ExtiLine, GpioLine, TriggerEdge},
        gpio::*,
        prelude::*,
        rcc::Config,
        syscfg::SYSCFG,
    };

    #[shared]
    struct Shared {
        led: Pin<Output<PushPull>>
    }

    #[local]
    struct Local { }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let device = ctx.device;

        // Configure the clock.
        let mut rcc = device.RCC.freeze(Config::hsi16());

        // Acquire the GPIOA peripheral. This also enables the clock for GPIOA in
        // the RCC register.
        let gpioa = device.GPIOA.split(&mut rcc);
        #[cfg(feature = "stm32l0x3")]
        let gpioc = device.GPIOC.split(&mut rcc);

        #[cfg(feature = "stm32l0x1")]
        let led = gpioa.pa1.into_push_pull_output().downgrade();
        #[cfg(any(feature = "stm32l0x2", feature = "stm32l0x3"))]
        let led = gpioa.pa5.into_push_pull_output().downgrade();

        #[cfg(not(feature = "stm32l0x3"))]
        let button = gpioa.pa0.into_pull_up_input();
        #[cfg(feature = "stm32l0x3")]
        let button = gpioc.pc13.into_pull_down_input();

        let mut syscfg = SYSCFG::new(device.SYSCFG, &mut rcc);
        let mut exti = Exti::new(device.EXTI);

        // Configure the external interrupt on the falling edge for the pin 0.
        let line = GpioLine::from_raw_line(button.pin_number()).unwrap();
        exti.listen_gpio(&mut syscfg, button.port(), line, TriggerEdge::Falling);

        // Return the initialised resources.
        (Shared { led }, Local { }, init::Monotonics())
    }

    #[task(binds = EXTI0_1, shared = [ led ], local = [ state: bool = false ])]
    fn exti0_1(ctx: exti0_1::Context) {
        // Clear the interrupt flag.
        Exti::unpend(GpioLine::from_raw_line(0).unwrap());

        let mut led = ctx.shared.led;

        // Change the LED state on each interrupt.
        if *ctx.local.state {
            led.lock(|led| led.set_low().unwrap());
            *ctx.local.state = false;
        } else {
            led.lock(|led| led.set_high().unwrap());
            *ctx.local.state = true;
        }
    }

    #[task(binds = EXTI4_15, shared = [ led ], local = [ state: bool = false ])]
    fn exti4_15(ctx: exti4_15::Context) {
        // Clear the interrupt flag for line 13.
        Exti::unpend(GpioLine::from_raw_line(13).unwrap());

        let mut led = ctx.shared.led;

        // Change the LED state on each interrupt.
        if *ctx.local.state {
            led.lock(|led| led.set_low().unwrap());
            *ctx.local.state = false;
        } else {
            led.lock(|led| led.set_high().unwrap());
            *ctx.local.state = true;
        }
    }
}
