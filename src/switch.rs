use embedded_hal::digital::v2::InputPin;

pub struct DebouncedSwitch<PIN> {
    pin: PIN,
    debounce_us: u64,
    next_check: u64,
    last_val: bool,
}

impl<'a, SWITCH> DebouncedSwitch<SWITCH>
where
    SWITCH: InputPin,
{
    pub fn new(pin: SWITCH, debounce_us: u64) -> DebouncedSwitch<SWITCH> {
        DebouncedSwitch {
            pin,
            next_check: 0,
            debounce_us,
            last_val: false,
        }
    }

    pub fn clicked(&mut self, now_us: u64) -> bool {
        if self.next_check < now_us {
            let pin = self.pin.is_low().unwrap_or_default();
            if pin != self.last_val {
                self.last_val = pin;
                self.next_check = now_us + self.debounce_us;
                return pin;
            }
        }
        false
    }
}
