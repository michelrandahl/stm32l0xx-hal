#![deny(unsafe_code)]
#![no_main]
#![no_std]

extern crate panic_halt;
//use panic_semihosting as _;
use rtt_target::{rprintln, rtt_init_print};

use cortex_m_rt::entry;
//use cortex_m_semihosting::hprintln;
use stm32l0xx_hal::{pac, prelude::*, rcc::Config};
use stm32l0xx_hal::tsc::{TscPin, Tsc};

#[entry]
fn main() -> ! {
    rtt_init_print!();
    rprintln!("Hello, world!");
    let dp = pac::Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();

    //// Configure the clock.
    let mut rcc = dp.RCC.freeze(Config::hsi16());

    let gpioa = dp.GPIOA.split(&mut rcc);
    //// Acquire GPIOB
    let gpiob = dp.GPIOB.split(&mut rcc);

    /// TODO describe components: 470nF cap, 1k resistor?
    //// Configure PB13 as a sampling pin and PB14 as a touch sensor pin
    let mut sample_pin = gpiob.pb13.into_open_drain_output();  // Adjust as per your setup
    let mut touch_channel_pin = gpiob.pb14.into_push_pull_output();

    // Initialize the TSC
    let mut tsc = Tsc::tsc(dp.TSC, &mut rcc, None);
    tsc.setup_sample_group(&mut sample_pin);
    tsc.enable_channel(&mut touch_channel_pin);

    // Configure LED
    let mut led = gpioa.pa5.into_push_pull_output();

    // Get the delay provider.
    let mut delay = cp.SYST.delay(rcc.clocks);

    loop {
        match tsc.acquire() {
            Ok(_) => {
                match tsc.read(&mut touch_channel_pin) {
                    Ok(v) => {
                        rprintln!("read value {}", v);
                    },
                    Err(err) => {
                        rprintln!("ERROR {:?}", err);
                    },
                }
            },
            Err(err) => {
                rprintln!("ERROR {:?}", err);
            },
        }
    }

    //let baseline = tsc.acquire(&mut touch_channel_pin).unwrap();
    //rprintln!("baseline {}", baseline);
    ////hprintln!("baseline: {}", baseline);
    ////let threshold = baseline + (baseline / 100) * 50; //(baseline / 100) * 90;
    //let threshold = 0; //(baseline / 10) * 11; //(baseline / 100) * 90;
    //rprintln!("threshold {}", threshold);
    ////hprintln!("threshold: {}", threshold);
    ////let threshold = baseline;

    //loop {
    //    if let Ok(result) = tsc.acquire(&mut touch_channel_pin) {
    //        rprintln!("result {}", result);
    //        //hprintln!("result: {}", result);
    //        if result > threshold {  // Define a suitable threshold
    //            led.toggle().unwrap();
    //            delay.delay_ms(500_u32);  // Add a delay to debounce the touch detection
    //        }
    //    }
    //}
}
