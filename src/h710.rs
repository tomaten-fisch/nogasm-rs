use embedded_hal::blocking::delay::DelayUs;
use embedded_hal::digital::v2::{InputPin, OutputPin};

pub struct H710<DataPin, ClkPin, Delay> {
    in_pin: DataPin,
    clk_pin: ClkPin,
    delay: Delay,
    mode: Mode,
}

#[allow(dead_code)]
pub enum Mode {
    HZ10,
    TEMP,
    HZ40,
}

impl<DataPin, ClkPin, Delay> H710<DataPin, ClkPin, Delay>
where
    DataPin: InputPin,
    ClkPin: OutputPin,
    Delay: DelayUs<u8>,
{
    pub fn new(
        in_pin: DataPin,
        clk_pin: ClkPin,
        delay: Delay,
        mode: Mode,
    ) -> H710<DataPin, ClkPin, Delay> {
        let mut h710 = H710 {
            in_pin,
            clk_pin,
            delay,
            mode,
        };
        h710.next_measurement();
        h710
    }

    pub fn read(&mut self) -> Option<u32> {
        while self.in_pin.is_high().unwrap_or_default() {}
        let mut value = 0u32;
        let mut last_zero_index = 0;
        for i in 0..24u8 {
            self.pulse();
            // info!("{}", value);
            let pin = self.int_value();
            if pin == 0 {
                last_zero_index = i;
            }
            value = (value << 1) | pin as u32;
        }
        self.next_measurement();

        // Sometimes the pin is stuck high, ignore those results
        if last_zero_index < 11 {
            None
        } else {
            Some(value ^ 0x800000)
        }
    }

    pub fn is_ready(&self) -> bool {
        self.in_pin.is_low().unwrap_or_default()
    }

    fn int_value(&self) -> u8 {
        let val = self.in_pin.is_high().unwrap_or_default() as u8;

        val
    }

    fn next_measurement(&mut self) {
        let count: u8 = match self.mode {
            Mode::HZ10 => 1,
            Mode::TEMP => 2,
            Mode::HZ40 => 3,
        };
        for _ in 0..count {
            self.pulse();
        }
    }

    fn pulse(&mut self) {
        self.clk_pin.set_high().unwrap_or_default();
        self.delay.delay_us(1u8);
        self.clk_pin.set_low().unwrap_or_default();
        self.delay.delay_us(1u8);
    }
}
