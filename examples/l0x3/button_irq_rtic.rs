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
    struct Shared {}

    #[local]
    struct Local {
        led: Pin<Output<PushPull>>,
    }

    #[init]
    fn init(ctx: init::Context) -> (Shared, Local, init::Monotonics) {
        let device = ctx.device;

        // Configure the clock.
        let mut rcc = device.RCC.freeze(Config::hsi16());

        // Acquire the GPI0A and GPIOC peripherals. This also enables the clock for
        // GPIOA and GPIOC in the RCC register.
        let gpioa = device.GPIOA.split(&mut rcc);
        let gpioc = device.GPIOC.split(&mut rcc);

        // Configure PA5 as output.
        let led = gpioa.pa5.into_push_pull_output().downgrade();

        // Configure PC13 as input.
        let button = gpioc.pc13.into_pull_down_input();

        let mut syscfg = SYSCFG::new(device.SYSCFG, &mut rcc);
        let mut exti = Exti::new(device.EXTI);

        // Configure the external interrupt on the falling edge for the button pin.
        let line = GpioLine::from_raw_line(button.pin_number()).unwrap();
        exti.listen_gpio(&mut syscfg, button.port(), line, TriggerEdge::Falling);

        // Return the initialised resources.
        (Shared {}, Local { led }, init::Monotonics())
    }

    #[task(binds = EXTI4_15, local = [ led, state: bool = false ])]
    fn exti4_15(ctx: exti4_15::Context) {
        // Clear the interrupt flag for line 13.
        Exti::unpend(GpioLine::from_raw_line(13).unwrap());

        // Change the LED state on each interrupt.
        if *ctx.local.state {
            ctx.local.led.set_low().unwrap();
            *ctx.local.state = false;
        } else {
            ctx.local.led.set_high().unwrap();
            *ctx.local.state = true;
        }
    }
}
