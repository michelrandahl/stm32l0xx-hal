use crate::gpio::gpiob::{PB13, PB14};
use crate::gpio::{OpenDrain, PushPull, Output, AltMode, Pin};
use crate::pac::TSC;
use crate::rcc::{Enable, Reset, Rcc};

// From the online training notes:
//- sampling capacitor I/O configured as
//    * alternate output open-drain mode with schmidth trigger hysteresis disabled
//- channel I/O is configured as
//    * alternate output push-pull
//
// (consider re-watching: https://www.st.com/content/st_com/en/support/learning/stm32-education/stm32-online-training/stm32l4-online-training.html)

#[derive(Debug)]
pub enum Event {
    /// Max count error
    MaxCountError,
    /// End of acquisition
    EndOfAcquisition,
}

#[derive(Debug)]
pub enum Error {
    /// Max count error
    MaxCountError,
    /// Wrong GPIO for reading
    InvalidPin,
}

pub trait TscPin {
    const GROUP : u8;
    const OFFSET : u8;
    const REDUCED_SENSITIVITY : bool;
    fn group(&self) -> u8 {
        Self::GROUP
    }
    fn bit_pos(&self) -> u8 {
        Self::OFFSET - 1 + (4 * (Self::GROUP - 1))
    }
    fn group_pos(&self) -> u8 {
        Self::GROUP - 1
    }

    fn setup(&self);
}


type TouchChannelPin = Pin<Output<OpenDrain>>;

// TODO: first we create some dumb implementations to keep it simple while testing, and then we
// create the macro later when we know this works...
impl TscPin for PB13<Output<OpenDrain>> {
    const GROUP : u8 = 6;
    const OFFSET : u8 = 3;
    const REDUCED_SENSITIVITY : bool = false;
    fn setup(&self) {
        self.set_alt_mode(AltMode::AF3);
    }
}
impl TscPin for PB14<Output<PushPull>> {
    const GROUP : u8 = 6;
    const OFFSET : u8 = 4;
    const REDUCED_SENSITIVITY : bool = false;
    fn setup(&self) {
        self.set_alt_mode(AltMode::AF3);
    }
}

pub struct Tsc {
    tsc: TSC,
}

#[derive(Debug)]
pub enum ClockPrescaler {
    Hclk = 0b000,
    HclkDiv2 = 0b001,
    HclkDiv4 = 0b010,
    HclkDiv8 = 0b011,
    HclkDiv16 = 0b100,
    HclkDiv32 = 0b101,
    HclkDiv64 = 0b110,
    HclkDiv128 = 0b111,
}

#[derive(Debug)]
pub enum MaxCount {
    /// 000: 255
    U255 = 0b000,
    /// 001: 511
    U511 = 0b001,
    /// 010: 1023
    U1023 = 0b010,
    /// 011: 2047
    U2047 = 0b011,
    /// 100: 4095
    U4095 = 0b100,
    /// 101: 8191
    U8191 = 0b101,
    /// 110: 16383
    U16383 = 0b110,
}

#[derive(Debug)]
/// How many tsc cycles are spent charging / discharging
pub enum ChargeDischargeTime {
    C1 = 0b0000,
    C2 = 0b0001,
    C3 = 0b0010,
    C4 = 0b0011,
    C5 = 0b0100,
    C6 = 0b0101,
    C7 = 0b0110,
    C8 = 0b0111,
    C9 = 0b1000,
    C10 = 0b1001,
    C11 = 0b1010,
    C12 = 0b1011,
    C13 = 0b1100,
    C14 = 0b1101,
    C15 = 0b1110,
    C16 = 0b1111,
}

#[derive(Debug)]
pub struct Config {
    pub clock_prescale: Option<ClockPrescaler>,
    pub max_count: Option<MaxCount>,
    pub charge_transfer_high: Option<ChargeDischargeTime>,
    pub charge_transfer_low: Option<ChargeDischargeTime>,
}

impl Tsc {
    /// Initialise the touch controller peripheral
    pub fn tsc(tsc: TSC, rcc: &mut Rcc, cfg: Option<Config>) -> Self {
        // Enable the peripheral clock
        rcc.ahbenr.modify(|_, w| w.touchen().set_bit());
        rcc.ahbrstr.modify(|_, w| w.touchrst().set_bit());
        rcc.ahbrstr.modify(|_, w| w.touchrst().clear_bit());

        let config = cfg.unwrap_or(Config {
            clock_prescale: None,
            max_count: None,
            charge_transfer_high: None,
            charge_transfer_low: None,
        });

        tsc.cr.write(|w| unsafe {
            w.ctph()
                .bits(
                    config
                        .charge_transfer_high
                        .unwrap_or(ChargeDischargeTime::C2) as u8,
                )
                .ctpl()
                .bits(
                    config
                        .charge_transfer_low
                        .unwrap_or(ChargeDischargeTime::C2) as u8,
                )
                .sse()
                .set_bit()
                .ssd()
                .bits(16)
                .pgpsc()
                .bits(config.clock_prescale.unwrap_or(ClockPrescaler::HclkDiv16) as u8)
                .mcv()
                .bits(config.max_count.unwrap_or(MaxCount::U8191) as u8)
                .tsce()
                .set_bit()
        });

        // clear interrupt & flags
        tsc.icr.write(|w| w.eoaic().set_bit().mceic().set_bit());

        Tsc { tsc }
    }

    ///// Set up sample group
    pub fn setup_sample_group<P>(&mut self, p : &mut P)
    where
        P : TscPin,
    {
        p.setup();
        let bit_pos = p.bit_pos();
        let group_pos = p.group_pos();

        // Schmitt trigger hysteresis on sample IOs
        self.tsc
            .iohcr
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << bit_pos) });

        // Set the sampling pin
        self.tsc
            .ioscr
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << bit_pos) });

        // Set the acquisition group based on the channel pins
        self.tsc
            .iogcsr
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << group_pos) });
    }

    /// Add a GPIO for use as a channel
    pub fn enable_channel<P>(&self, channel: &mut P)
    where
        P : TscPin,
    {
        channel.setup();
        let bit_pos = channel.bit_pos();

        // Set a channel pin
        self.tsc
            .ioccr
            .modify(|r, w| unsafe { w.bits(r.bits() | 1 << bit_pos) });
    }

    /// Remove a GPIO from use as a channel
    pub fn disable_channel<P>(&self, channel: &mut P)
    where
        P : TscPin,
    {
        let bit_pos = channel.bit_pos();

        // Remove a channel pin
        self.tsc
            .ioccr
            .modify(|r, w| unsafe { w.bits(r.bits() & !(1 << bit_pos)) });
    }

    /// Clear interrupt & flags
    pub fn clear(&self, event: Event) {
        match event {
            Event::EndOfAcquisition => {
                self.tsc.icr.write(|w| w.eoaic().set_bit());
            }
            Event::MaxCountError => {
                self.tsc.icr.write(|w| w.mceic().set_bit());
            }
        }
    }

    /// Starts a charge acquisition
    pub fn start(&self) {
        self.clear(Event::EndOfAcquisition);
        self.clear(Event::MaxCountError);

        // Discharge the caps ready for a new reading
        self.tsc.cr.modify(|_, w| w.iodef().clear_bit());
        self.tsc.cr.modify(|_, w| w.start().set_bit());
    }

    /// Check for events on the TSC
    pub fn check_event(&self) -> Option<Event> {
        let isr = self.tsc.isr.read();
        if isr.eoaf().bit_is_set() {
            Some(Event::EndOfAcquisition)
        } else if isr.mcef().bit_is_set() {
            Some(Event::MaxCountError)
        } else {
            None
        }
    }

    /// Blocks waiting for a acquisition to complete or for a Max Count Error
    pub fn acquire(&self) -> Result<(), Error> {
        // Start the acquisition
        self.start();

        loop {
            match self.check_event() {
                Some(Event::MaxCountError) => {
                    self.clear(Event::MaxCountError);
                    break Err(Error::MaxCountError);
                }
                Some(Event::EndOfAcquisition) => {
                    self.clear(Event::EndOfAcquisition);
                    break Ok(());
                }
                None => {}
            }
        }
    }

    /// Reads the tsc group count register
    pub fn read_unchecked(&self, group: u8) -> u16 {
        match group {
            1 => self.tsc.iog1cr().read().cnt().bits(),
            2 => self.tsc.iog2cr().read().cnt().bits(),
            3 => self.tsc.iog3cr().read().cnt().bits(),
            4 => self.tsc.iog4cr().read().cnt().bits(),
            5 => self.tsc.iog5cr().read().cnt().bits(),
            6 => self.tsc.iog6cr().read().cnt().bits(),
            7 => self.tsc.iog7cr().read().cnt().bits(),
            8 => self.tsc.iog8cr().read().cnt().bits(),
            _ => 0,
        }
    }

    /// Reads the group count register
    pub fn read<PIN>(&self, input : &mut PIN) -> Result<u16, Error>
    where
        PIN: TscPin,
    {
        let bit_pos = input.bit_pos();

        // Read the current channel config
        let channel = self.tsc.ioccr.read().bits();

        // Check whether one of the enabled pins was supplied
        if channel & (1 << bit_pos) != 0 {
            Ok(self.read_unchecked(input.group()))
        } else {
            Err(Error::InvalidPin)
        }
    }

    /// Enables an interrupt event
    pub fn listen(&mut self, event: Event) {
        match event {
            Event::EndOfAcquisition => {
                self.tsc.ier.modify(|_, w| w.eoaie().set_bit());
            }
            Event::MaxCountError => {
                self.tsc.ier.modify(|_, w| w.mceie().set_bit());
            }
        }
    }

    /// Disables an interrupt event
    pub fn unlisten(&self, event: Event) {
        match event {
            Event::EndOfAcquisition => {
                self.tsc.ier.modify(|_, w| w.eoaie().clear_bit());
            }
            Event::MaxCountError => {
                self.tsc.ier.modify(|_, w| w.mceie().clear_bit());
            }
        }
    }

    /// Releases the TSC peripheral
    pub fn free(self) -> TSC {
        self.tsc
    }
}

//use crate::gpio::gpioa::{PA0, PA1, PA2, PA3, PA4, PA5, PA6, PA7, PA9, PA10, PA11, PA12};
//use crate::gpio::gpiob::{PB0, PB1, PB2, PB3, PB4, PB6, PB7, PB11, PB12, PB13, PB14};
//use crate::gpio::gpioc::{PC0, PC1, PC2, PC3, PC5, PC6, PC7, PC8, PC9};
//use crate::gpio::{OpenDrain, PushPull, Output};
//use crate::rcc::{Enable, Reset, Rcc};
//use crate::pac::TSC;
//
//#[derive(Clone, Copy, Debug, PartialEq)]
//pub enum Event {
//    /// Max count error
//    MaxCountError,
//    /// End of acquisition
//    EndOfAcquisition,
//}
//
//#[derive(Clone, Copy, Debug, PartialEq)]
//pub enum Error {
//    /// Max count error
//    MaxCountError,
//    /// Wrong GPIO for reading - returns the ioccr register
//    InvalidPin(u32),
//}
//
//pub trait SamplePin {
//    const GROUP: u32;
//    const OFFSET: u32;
//    const REDUCED_SENSITIVITY: bool;  // Indicates if the pin has reduced sensitivity
//
//    // Method to check if the pin has reduced sensitivity
//    fn has_reduced_sensitivity() -> bool {
//        Self::REDUCED_SENSITIVITY
//    }
//}
//
//pub trait TouchChannelPin {
//    const GROUP: u32;
//    const OFFSET: u32;
//    const REDUCED_SENSITIVITY: bool;  // Indicates if the pin has reduced sensitivity
//
//    // Method to check if the pin has reduced sensitivity
//    fn has_reduced_sensitivity() -> bool {
//        Self::REDUCED_SENSITIVITY
//    }
//}
//
//macro_rules! impl_sample_pin {
//    ($(($pin:ident, $group:expr, $offset:expr, $reduced_sensitivity:expr)),+) => {
//        $(
//            impl SamplePin for $pin<Output<OpenDrain>> {
//                const GROUP: u32 = $group;
//                const OFFSET: u32 = $offset;
//                const REDUCED_SENSITIVITY: bool = $reduced_sensitivity;
//            }
//            impl TouchChannelPin for $pin<Output<PushPull>> {
//                const GROUP: u32 = $group;
//                const OFFSET: u32 = $offset;
//                const REDUCED_SENSITIVITY: bool = $reduced_sensitivity;
//            }
//        )+
//    }
//}
//
//impl_sample_pin!(
//    // Group 1
//    (PA0, 1, 0, false),
//    (PA1, 1, 1, false),
//    (PA2, 1, 2, false),
//    (PA3, 1, 3, false),
//    // Group 2
//    (PA4, 2, 0, true), // PA4 has reduced sensitivity
//    (PA5, 2, 1, false),
//    (PA6, 2, 2, false),
//    (PA7, 2, 3, false),
//    // Group 3
//    (PC5, 3, 0, false),
//    (PB0, 3, 1, false),
//    (PB1, 3, 2, false),
//    (PB2, 3, 3, false),
//    // Group 4
//    (PA9, 4, 0, false),
//    (PA10, 4, 1, false),
//    (PA11, 4, 2, false),
//    (PA12, 4, 3, false),
//    // Group 5
//    (PB3, 5, 0, false),
//    (PB4, 5, 1, false),
//    (PB6, 5, 2, false),
//    (PB7, 5, 3, false),
//    // Group 6
//    (PB11, 6, 0, false),
//    (PB12, 6, 1, false),
//    (PB13, 6, 2, false),
//    (PB14, 6, 3, false),
//    // Group 7
//    (PC0, 7, 0, false),
//    (PC1, 7, 1, false),
//    (PC2, 7, 2, false),
//    (PC3, 7, 3, false),
//    // Group 8
//    (PC6, 8, 0, false),
//    (PC7, 8, 1, false),
//    (PC8, 8, 2, false),
//    (PC9, 8, 3, false)
//);
//
//pub struct Tsc<SPIN> {
//    sample_pin: SPIN,
//    tsc: TSC,
//}
//
//#[derive(Clone, Copy, Debug, PartialEq)]
//pub enum ClockPrescaler {
//    Hclk = 0b000,
//    HclkDiv2 = 0b001,
//    HclkDiv4 = 0b010,
//    HclkDiv8 = 0b011,
//    HclkDiv16 = 0b100,
//    HclkDiv32 = 0b101,
//    HclkDiv64 = 0b110,
//    HclkDiv128 = 0b111,
//}
//
//#[derive(Clone, Copy, Debug, PartialEq)]
//pub enum MaxCountError {
//    /// 000: 255
//    U255 = 0b000,
//    /// 001: 511
//    U511 = 0b001,
//    /// 010: 1023
//    U1023 = 0b010,
//    /// 011: 2047
//    U2047 = 0b011,
//    /// 100: 4095
//    U4095 = 0b100,
//    /// 101: 8191
//    U8191 = 0b101,
//    /// 110: 16383
//    U16383 = 0b110,
//}
//
//#[derive(Clone, Copy, Debug, PartialEq)]
///// How many tsc cycles are spent charging / discharging
//pub enum ChargeDischargeTime {
//    C1 = 0b0000,
//    C2 = 0b0001,
//    C3 = 0b0010,
//    C4 = 0b0011,
//    C5 = 0b0100,
//    C6 = 0b0101,
//    C7 = 0b0110,
//    C8 = 0b0111,
//    C9 = 0b1000,
//    C10 = 0b1001,
//    C11 = 0b1010,
//    C12 = 0b1011,
//    C13 = 0b1100,
//    C14 = 0b1101,
//    C15 = 0b1110,
//    C16 = 0b1111,
//}
//
//#[derive(Clone, Copy, Debug, PartialEq)]
//pub struct Config {
//    pub clock_prescale: Option<ClockPrescaler>,
//    pub max_count_error: Option<MaxCountError>,
//    pub charge_transfer_high: Option<ChargeDischargeTime>,
//    pub charge_transfer_low: Option<ChargeDischargeTime>,
//    /// Spread spectrum deviation - a value between 0 and 128
//    pub spread_spectrum_deviation: Option<u8>,
//}
//
//impl<SPIN> Tsc<SPIN> {
//    pub fn sc(tsc: TSC, sample_pin: SPIN, rcc: &mut Rcc, cfg: Option<Config>) -> Self
//    where
//        SPIN: SamplePin,
//    {
//        /* Enable the peripheral clock */
//        TSC::enable(rcc);
//        TSC::reset(rcc);
//
//        let config = cfg.unwrap_or(Config {
//            clock_prescale: None,
//            max_count_error: None,
//            charge_transfer_high: None,
//            charge_transfer_low: None,
//            spread_spectrum_deviation: None,
//        });
//
//        tsc.cr.write(|w| unsafe {
//            w.ctph()
//                .bits(
//                    config
//                        .charge_transfer_high
//                        .unwrap_or(ChargeDischargeTime::C2) as u8,
//                )
//                .ctpl()
//                .bits(
//                    config
//                        .charge_transfer_low
//                        .unwrap_or(ChargeDischargeTime::C2) as u8,
//                )
//                .pgpsc()
//                .bits(config.clock_prescale.unwrap_or(ClockPrescaler::Hclk) as u8)
//                .mcv()
//                .bits(config.max_count_error.unwrap_or(MaxCountError::U8191) as u8)
//                .sse()
//                .bit(config.spread_spectrum_deviation.is_some())
//                .ssd()
//                .bits(config.spread_spectrum_deviation.unwrap_or(0u8))
//                .tsce()
//                .set_bit()
//        });
//
//        let bit_pos = SPIN::OFFSET + (4 * (SPIN::GROUP - 1));
//
//        // Schmitt trigger hysteresis on sample IOs
//        tsc.iohcr.write(|w| unsafe { w.bits(1 << bit_pos) });
//
//        // Set the sampling pin
//        tsc.ioscr.write(|w| unsafe { w.bits(1 << bit_pos) });
//
//        // set the acquisitiuon groups based of the channel pins, stm32l432xx only has group 2
//        //tsc.iogcsr.write(|w| w.g2e().set_bit());
//        tsc.iogcsr.write(|w| w.g6e().set_bit());
//
//        // clear interrupt & flags
//        tsc.icr.write(|w| w.eoaic().set_bit().mceic().set_bit());
//
//        Tsc { tsc, sample_pin }
//    }
//
//    /// Starts a charge acquisition
//    pub fn start<PIN>(&self, _input: &mut PIN)
//    where
//        PIN: TouchChannelPin,
//    {
//        self.clear(Event::EndOfAcquisition);
//        self.clear(Event::MaxCountError);
//
//        // discharge the caps ready for a new reading
//        self.tsc.cr.modify(|_, w| w.iodef().clear_bit());
//
//        let bit_pos = PIN::OFFSET + (4 * (PIN::GROUP - 1));
//
//        // Set the channel pin
//        self.tsc.ioccr.write(|w| unsafe { w.bits(1 << bit_pos) });
//
//        self.tsc.cr.modify(|_, w| w.start().set_bit());
//    }
//
//    /// Clear interrupt & flags
//    pub fn clear(&self, event: Event) {
//        match event {
//            Event::EndOfAcquisition => {
//                self.tsc.icr.write(|w| w.eoaic().set_bit());
//            }
//            Event::MaxCountError => {
//                self.tsc.icr.write(|w| w.mceic().set_bit());
//            }
//        }
//    }
//
//    /// Blocks waiting for a acquisition to complete or for a Max Count Error
//    pub fn acquire<PIN>(&self, input: &mut PIN) -> Result<u16, Error>
//    where
//        PIN: TouchChannelPin,
//    {
//        // start the acq
//        self.start(input);
//
//        let result = loop {
//            let isr = self.tsc.isr.read();
//            if isr.eoaf().bit_is_set() {
//                self.tsc.icr.write(|w| w.eoaic().set_bit());
//                break Ok(self.read_unchecked());
//            } else if isr.mcef().bit_is_set() {
//                self.tsc.icr.write(|w| w.mceic().set_bit());
//                break Err(Error::MaxCountError);
//            }
//        };
//        self.tsc.ioccr.write(|w| unsafe { w.bits(0b0) }); // clear channel register
//        result
//    }
//
//    /// Reads the tsc group 2 count register
//    pub fn read<PIN>(&self, _input: &mut PIN) -> Result<u16, Error>
//    where
//        PIN: TouchChannelPin,
//    {
//        let bit_pos = PIN::OFFSET + (4 * (PIN::GROUP - 1));
//        // Read the current channel config
//        let channel = self.tsc.ioccr.read().bits();
//        // if they are equal we have the right pin
//        if channel == (1 << bit_pos) {
//            Ok(self.read_unchecked())
//        } else {
//            Err(Error::InvalidPin(channel))
//        }
//    }
//
//    /// Reads the tsc group 2 count register
//    /// WARNING, just returns the contents of the register! No validation of the correct pin
//    pub fn read_unchecked(&self) -> u16 {
//        //self.tsc.iog2cr().read().cnt().bits()
//        self.tsc.iog6cr().read().cnt().bits()
//        //self.tsc.iog2cr.read().cnt().bits()
//    }
//
//    /// Is the tsc performing an aquisition
//    pub fn in_progress(&mut self) -> bool {
//        self.tsc.cr.read().start().bit_is_set()
//    }
//
//    /// Enables an interrupt event
//    pub fn listen(&mut self, event: Event) {
//        match event {
//            Event::EndOfAcquisition => {
//                self.tsc.ier.modify(|_, w| w.eoaie().set_bit());
//            }
//            Event::MaxCountError => {
//                self.tsc.ier.modify(|_, w| w.mceie().set_bit());
//            }
//        }
//    }
//
//    /// Disables an interrupt event
//    pub fn unlisten(&self, event: Event) {
//        match event {
//            Event::EndOfAcquisition => {
//                self.tsc.ier.modify(|_, w| w.eoaie().clear_bit());
//            }
//            Event::MaxCountError => {
//                self.tsc.ier.modify(|_, w| w.mceie().clear_bit());
//            }
//        }
//    }
//
//    /// Releases the TSC peripheral and associated pins
//    pub fn free(self) -> (TSC, SPIN) {
//        (self.tsc, self.sample_pin)
//    }
//}
