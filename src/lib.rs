#![no_std]

mod avg;
mod ble;
mod display;
mod h710;
mod history;
mod menu;
mod state;
mod switch;

extern crate alloc;
use core::cell::{Cell, RefCell};

use alloc::boxed::Box;
use ble::{ble_get_name, ble_is_connected};
use critical_section::Mutex;
use esp_backtrace as _;
use hal::i2c::I2C;
use hal::{clock::ClockControl, peripherals::Peripherals, prelude::*, Rtc};
use log::info;
// use panic_halt as _;
use ssd1306::I2CDisplayInterface;

use rotary_encoder_embedded::RotaryEncoder;

use crate::display::OLEDDisplay;
use crate::history::HistoryResult;
use crate::menu::Menu;
use crate::state::State;
use crate::switch::DebouncedSwitch;

#[global_allocator]
static ALLOCATOR: esp_alloc::EspHeap = esp_alloc::EspHeap::empty();

#[no_mangle]
pub extern "C" fn rs_init_heap(heap_start: *mut u8, heap_size: cty::size_t) {
    unsafe {
        ALLOCATOR.init(heap_start as *mut u8, heap_size.into());
    }
}

static ENCODER: Mutex<
    RefCell<
        Option<
            RotaryEncoder<
                rotary_encoder_embedded::standard::StandardMode,
                hal::gpio::GpioPin<hal::gpio::Input<hal::gpio::PullUp>, 4>,
                hal::gpio::GpioPin<hal::gpio::Input<hal::gpio::PullUp>, 15>,
            >,
        >,
    >,
> = Mutex::new(RefCell::new(None));

static ENCODER_DIRECTION: Mutex<Cell<rotary_encoder_embedded::Direction>> =
    Mutex::new(Cell::new(rotary_encoder_embedded::Direction::None));

#[repr(C)]
pub struct RustState<'a> {
    menu: Box<Menu>,
    state: Box<State>,
    display: Box<OLEDDisplay<ssd1306::prelude::I2CInterface<I2C<'a, hal::peripherals::I2C0>>>>,
    encoder_sw: Box<DebouncedSwitch<hal::gpio::GpioPin<hal::gpio::Input<hal::gpio::PullUp>, 5>>>,
    history: Box<history::Nogasm<4>>,
    h710: Box<
        h710::H710<
            hal::gpio::GpioPin<hal::gpio::Input<hal::gpio::PullUp>, 16>,
            hal::gpio::GpioPin<hal::gpio::Output<hal::gpio::PushPull>, 17>,
            hal::Delay,
        >,
    >,
    rtc: Box<Rtc<'a>>,
}

#[no_mangle]
pub extern "C" fn rs_init<'a>() -> RustState<'a> {
    // init_heap();
    let peripherals = Peripherals::take();
    let mut system = peripherals.DPORT.split();
    let clocks = ClockControl::boot_defaults(system.clock_control).freeze();

    // Disable the RTC and TIMG watchdog timers
    let rtc = Rtc::new(peripherals.RTC_CNTL);
    // let timer_group0 = TimerGroup::new(
    //     peripherals.TIMG0,
    //     &clocks,
    //     &mut system.peripheral_clock_control,
    // );
    // let mut wdt0 = timer_group0.wdt;
    // let timer_group1 = TimerGroup::new(
    //     peripherals.TIMG1,
    //     &clocks,
    //     &mut system.peripheral_clock_control,
    // );
    // let mut wdt1 = timer_group1.wdt;
    // rtc.rwdt.disable();
    // wdt0.disable();
    // wdt1.disable();
    // setup logger
    // To change the log_level change the env section in .config/cargo.toml
    // or remove it and set ESP_LOGLEVEL manually before running cargo run
    // this requires a clean rebuild because of https://github.com/rust-lang/cargo/issues/10358
    esp_println::logger::init_logger_from_env();
    // info!("Logger is setup");
    // println!("Hello world!");

    let io = hal::IO::new(peripherals.GPIO, peripherals.IO_MUX);
    let i2c = I2C::new(
        peripherals.I2C0,
        io.pins.gpio21,
        io.pins.gpio22,
        1000u32.kHz(),
        &mut system.peripheral_clock_control,
        &clocks,
    );
    let interface = I2CDisplayInterface::new(i2c);
    let display = OLEDDisplay::new(interface);

    let encoder_dt = io.pins.gpio4.into_pull_up_input();
    let encoder_clk = io.pins.gpio15.into_pull_up_input();
    let encoder = RotaryEncoder::new(encoder_dt, encoder_clk).into_standard_mode();
    critical_section::with(|cs| ENCODER.borrow_ref_mut(cs).replace(encoder));
    let encoder_sw = DebouncedSwitch::new(io.pins.gpio5.into_pull_up_input(), 500);

    let delay = hal::delay::Delay::new(&clocks);

    let sensor_data = io.pins.gpio16.into_pull_up_input();
    let sensor_clock = io.pins.gpio17.into_push_pull_output();
    let h710 = h710::H710::new(sensor_data, sensor_clock, delay, h710::Mode::HZ40);

    let history = history::Nogasm::<4>::new();

    // let mut timer00 = timer_group0.timer0;
    // hal::interrupt::enable(
    //     hal::peripherals::Interrupt::TG0_T0_LEVEL,
    //     hal::interrupt::Priority::Priority3,
    // )
    // .unwrap();
    // // timer00.set_auto_reload(true);
    // timer00.start(1100u32.micros());
    // timer00.listen();
    // critical_section::with(|cs| {
    //     TIMER00.borrow_ref_mut(cs).replace(timer00);
    // });

    let state = State::new();
    let menu = Menu::default();

    RustState {
        menu: Box::new(menu),
        state: Box::new(state),
        display: Box::new(display),
        encoder_sw: Box::new(encoder_sw),
        history: Box::new(history),
        h710: Box::new(h710),
        rtc: Box::new(rtc),
    }
}

#[no_mangle]
pub extern "C" fn loop_once(rust_state: *mut RustState) -> u8 {
    let rust_state = unsafe { rust_state.as_mut().unwrap() };

    /* Read user input */
    critical_section::with(|cs| {
        match ENCODER_DIRECTION.borrow(cs).get() {
            rotary_encoder_embedded::Direction::Clockwise => {
                rust_state.menu.foward(&mut rust_state.state);
            }
            rotary_encoder_embedded::Direction::Anticlockwise => {
                rust_state.menu.backward(&mut rust_state.state);
            }
            _ => {}
        };
        ENCODER_DIRECTION
            .borrow(cs)
            .set(rotary_encoder_embedded::Direction::None);
    });

    if rust_state.encoder_sw.clicked(rust_state.rtc.get_time_us()) {
        rust_state.menu.click(&mut rust_state.state);
    }

    /* Check bluetooth */
    rust_state.state.set_ble_connected(ble_is_connected());
    rust_state.state.set_ble_name(ble_get_name());

    /* get current time */
    rust_state.state.cur_time_ms = rust_state.rtc.get_time_ms() as u32;

    /* Update display (only updates if necessary) */
    rust_state
        .display
        .update(&rust_state.menu, &rust_state.state);

    /* If not running, let manual override work */
    if !rust_state.state.running {
        return rust_state.state.get_cur_intensity();
    }

    /* If running, read sensor and update if necessary */
    if rust_state.h710.is_ready() {
        let val = rust_state.h710.read();
        if let Some(val) = val {
            let res = rust_state.history.add(
                val,
                rust_state.rtc.get_time_ms() as u32,
                &mut rust_state.state,
            );
            info!("V:{}, A:{}", val, rust_state.history.get_area());
            match res {
                HistoryResult::Stop => {
                    rust_state.state.stop_stim();
                }
                HistoryResult::Resume => {
                    rust_state.state.start_stim();
                }
            }
        }
    }
    rust_state.state.get_cur_intensity()
}

#[no_mangle]
pub extern "C" fn rs_handle_timer(_: *mut cty::c_void) {
    // info!("timer!");
    critical_section::with(|cs| {
        // let mut timer = TIMER00.borrow_ref_mut(cs);
        // let timer = timer.as_mut().unwrap();

        // if timer.is_interrupt_set() {
        //     timer.clear_interrupt();
        //     timer.start(1100u32.micros());

        //     // esp_println::println!("Interrupt Level 2 - Timer0");
        // }
        let mut rotary_encoder = ENCODER.borrow_ref_mut(cs);
        let rotary_encoder = rotary_encoder.as_mut().unwrap();
        rotary_encoder.update();
        let dir = rotary_encoder.direction();
        match dir {
            rotary_encoder_embedded::Direction::Clockwise => ENCODER_DIRECTION.borrow(cs).set(dir),
            rotary_encoder_embedded::Direction::Anticlockwise => {
                ENCODER_DIRECTION.borrow(cs).set(dir)
            }
            _ => {}
        }

        // ENCODER_DIRECTION.borrow(cs).set(rotary_encoder.direction());
    });
}
