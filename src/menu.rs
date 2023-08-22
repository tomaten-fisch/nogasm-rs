use crate::state::*;

#[derive(PartialEq)]
pub enum MenuPosition {
    Main,
    Peak,
    PeakSelect,
    Area,
    AreaSelect,
    Duration,
    DurationSelect,
    Intensity,
    IntensitySelect,
    Cooldown,
    CooldownSelect,
}

impl Default for MenuPosition {
    fn default() -> Self {
        MenuPosition::Main
    }
}

pub struct Menu {
    pub position: MenuPosition,
}

impl Default for Menu {
    fn default() -> Self {
        Menu {
            position: MenuPosition::default(),
        }
    }
}

impl Menu {
    pub fn backward(&mut self, state: &mut State) {
        use MenuPosition::*;
        self.position = match self.position {
            Main => Intensity,
            Peak => Main,
            Area => Peak,
            Duration => Area,
            Cooldown => Duration,
            Intensity => Cooldown,
            PeakSelect => {
                state.peak_down();
                PeakSelect
            }
            AreaSelect => {
                state.area_down();
                AreaSelect
            }
            DurationSelect => {
                state.duration_down();
                DurationSelect
            }
            IntensitySelect => {
                state.intensity_down();
                IntensitySelect
            }
            CooldownSelect => {
                state.cooldown_down();
                CooldownSelect
            }
        }
    }
    pub fn foward(&mut self, state: &mut State) {
        use MenuPosition::*;
        self.position = match self.position {
            Main => Peak,
            Peak => Area,
            Area => Duration,
            Duration => Cooldown,
            Cooldown => Intensity,
            Intensity => Main,
            PeakSelect => {
                state.peak_up();
                PeakSelect
            }
            AreaSelect => {
                state.area_up();
                AreaSelect
            }
            DurationSelect => {
                state.duration_up();
                DurationSelect
            }
            IntensitySelect => {
                state.intensity_up();
                IntensitySelect
            }
            CooldownSelect => {
                state.cooldown_up();
                CooldownSelect
            }
        }
    }
    pub fn click(&mut self, state: &mut State) {
        use MenuPosition::*;
        self.position = match self.position {
            Main => {
                state.toggle();
                Main
            }
            Peak => PeakSelect,
            Area => AreaSelect,
            Duration => DurationSelect,
            Cooldown => CooldownSelect,
            Intensity if state.ble_connected => {
                state.start_stim_manual();
                IntensitySelect
            }
            Intensity => Intensity,
            PeakSelect => Peak,
            AreaSelect => Area,
            DurationSelect => Duration,
            CooldownSelect => Cooldown,
            IntensitySelect => {
                state.stop_stim_manual();
                Intensity
            }
        }
    }
}
