use core::fmt::Write;
use display_interface::{DisplayError, WriteOnlyDataCommand};
use embedded_graphics::{
    mono_font::{ascii::FONT_6X10, MonoTextStyle, MonoTextStyleBuilder},
    pixelcolor::BinaryColor,
    prelude::*,
    primitives::{Circle, Line, PrimitiveStyle, Rectangle, Triangle},
    text::{Baseline, Text},
};
use heapless::String;
use ssd1306::{mode::BufferedGraphicsMode, prelude::*, Ssd1306};

use crate::{menu, state};

const THIN_STROKE: PrimitiveStyle<BinaryColor> = PrimitiveStyle::with_stroke(BinaryColor::On, 1);
const THICK_STROKE: PrimitiveStyle<BinaryColor> = PrimitiveStyle::with_stroke(BinaryColor::On, 2);
const FILLED_STYLE: PrimitiveStyle<BinaryColor> = PrimitiveStyle::with_fill(BinaryColor::On);
const NORMAL_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .build();
const UNDERLINED_TEXT_STYLE: MonoTextStyle<'_, BinaryColor> = MonoTextStyleBuilder::new()
    .font(&FONT_6X10)
    .text_color(BinaryColor::On)
    .underline()
    .build();

const FIRST_ROW: Point = Point::new(5, 0);
const SECOND_ROW: Point = Point::new(5, 14);
const INTER_FRAME_TIME_MS: u32 = 50;

pub struct OLEDDisplay<DI> {
    display: Ssd1306<DI, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>,
    next_update_ms: u32,
}

impl<DI> OLEDDisplay<DI>
where
    DI: WriteOnlyDataCommand,
    Ssd1306<DI, DisplaySize128x32, BufferedGraphicsMode<DisplaySize128x32>>:
        DrawTarget<Color = BinaryColor, Error = DisplayError>,
{
    pub fn new(interface: DI) -> OLEDDisplay<DI> {
        let mut display = Ssd1306::new(interface, DisplaySize128x32, DisplayRotation::Rotate0)
            .into_buffered_graphics_mode();
        display.init().unwrap();

        OLEDDisplay {
            display,
            next_update_ms: 0,
        }
    }

    fn print_text(&mut self, position: Point, text: &str, underlined: bool) {
        Text::with_baseline(
            text,
            position,
            match underlined {
                false => NORMAL_TEXT_STYLE,
                true => UNDERLINED_TEXT_STYLE,
            },
            Baseline::Top,
        )
        .draw(&mut self.display)
        .unwrap();
    }

    fn print_position(&mut self, pos: i32) {
        let per_pos = 32 / 6;
        Line::new(
            Point::new(0, pos * per_pos),
            Point::new(0, (pos + 1) * per_pos),
        )
        .into_styled(THICK_STROKE)
        .draw(&mut self.display)
        .unwrap()
    }

    fn print_play_button(&mut self) {
        Circle::with_center(Point::new(112, 16), 20)
            .into_styled(THIN_STROKE)
            .draw(&mut self.display)
            .unwrap();

        Triangle::new(
            Point::new(110, 12),
            Point::new(110, 20),
            Point::new(117, 16),
        )
        .into_styled(FILLED_STYLE)
        .draw(&mut self.display)
        .unwrap();
    }

    fn print_stop_button(&mut self) {
        Circle::with_center(Point::new(112, 16), 20)
            .into_styled(THIN_STROKE)
            .draw(&mut self.display)
            .unwrap();
        Rectangle::with_center(Point::new(112, 16), Size::new(8, 8))
            .into_styled(FILLED_STYLE)
            .draw(&mut self.display)
            .unwrap();
    }

    fn print_main_menu(&mut self, state: &state::State) {
        if state.running {
            self.print_stop_button();
        } else {
            self.print_play_button();
        };
        let mut text: String<30> = String::<30>::new();
        let stim_str = if !state.running {
            "Ready\n"
        } else if state.stimulating {
            write!(
                &mut text,
                "Stimulation\n{:.1}s",
                (state.cur_time_ms - state.stim_start_time) as f32 / 1000f32
            )
            .unwrap();
            text.as_str()
        } else {
            write!(
                &mut text,
                "No Stimulation\n{:.1}s",
                state
                    .hysteresis
                    .remaining(state.cur_time_ms, state.cooldown_time) as f32
                    / 1000f32
            )
            .unwrap();
            text.as_str()
        };

        let mut text = String::<100>::new();
        write!(&mut text, "{}\nB: {}", stim_str, state.ble_name).unwrap();
        self.print_text(FIRST_ROW, text.as_str(), false);
    }

    fn print_value_menu(&mut self, name: &str, value: u32, unit: &str, underlined: bool) {
        self.print_text(FIRST_ROW, name, false);

        let mut text = String::<30>::new();
        write!(&mut text, "{}{}", value, unit).unwrap();
        self.print_text(SECOND_ROW, text.as_str(), underlined);
    }

    fn print_ble_menu(&mut self, state: &state::State, underlined: bool) {
        if !state.ble_connected {
            self.print_text(FIRST_ROW, "BLE: not connected", false);
            self.print_text(SECOND_ROW, "n/a", false);
        } else {
            let mut text = String::<30>::new();
            write!(&mut text, "BLE: {}", state.ble_name).unwrap();
            self.print_text(FIRST_ROW, text.as_str(), false);
            let mut text = String::<30>::new();
            if state.running {
                write!(&mut text, "{}/20 (auto)", state.intensity).unwrap();
                self.print_text(SECOND_ROW, text.as_str(), underlined);
            } else {
                write!(&mut text, "{}/20 (manual)", state.intensity).unwrap();
                self.print_text(SECOND_ROW, text.as_str(), underlined);
            }
        }
    }

    pub fn update(&mut self, menu: &menu::Menu, state: &state::State) {
        if self.next_update_ms > state.cur_time_ms {
            return;
        }

        self.next_update_ms = state.cur_time_ms + INTER_FRAME_TIME_MS;

        use menu::MenuPosition::*;
        self.display.clear_buffer();
        match menu.position {
            Main => {
                self.print_main_menu(state);
                self.print_position(0)
            }
            Peak => {
                self.print_value_menu("Sensitivity", state.peak_value_thresh, "", false);
                self.print_position(1);
            }
            PeakSelect => {
                self.print_value_menu("Sensitivity", state.peak_value_thresh, "", true);
                self.print_position(1);
            }
            Area => {
                self.print_value_menu("Density", state.peak_area_threshold, "", false);
                self.print_position(2);
            }
            AreaSelect => {
                self.print_value_menu("Density", state.peak_area_threshold, "", true);
                self.print_position(2);
            }
            Duration => {
                self.print_value_menu("Duration", state.peak_release_time_thresh, "ms", false);
                self.print_position(3);
            }
            DurationSelect => {
                self.print_value_menu("Duration", state.peak_release_time_thresh, "ms", true);
                self.print_position(3);
            }
            Cooldown => {
                self.print_value_menu("Cooldown time", state.cooldown_time / 1_000, "s", false);
                self.print_position(4);
            }
            CooldownSelect => {
                self.print_value_menu("Cooldown time", state.cooldown_time / 1_000, "s", true);
                self.print_position(4);
            }
            Intensity => {
                self.print_ble_menu(state, false);
                self.print_position(5);
            }
            IntensitySelect => {
                self.print_ble_menu(state, true);
                self.print_position(5);
            }
        }
        self.display.flush().unwrap();
    }
}
