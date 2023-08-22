const MAX_INTENSITY: u8 = 20;
const MAX_AREA: u32 = 10_000_000;
const AREA_STEP: u32 = 10_000;
const MAX_PEAK: u32 = 1_000_000;
const PEAK_STEP: u32 = 1_000;
const COOLDOWN_STEP: u32 = 1_000;
const MAX_DURATION: u32 = 5_000;
const DURATION_STEP: u32 = 25;

pub struct Hysteresis {
    entry_time: u32,
}

impl Hysteresis {
    pub fn new() -> Hysteresis {
        Hysteresis { entry_time: 0 }
    }

    pub fn enter(&mut self, time: u32) {
        self.entry_time = time
    }

    pub fn is_active(&self, time: u32, cooldown_time: u32) -> bool {
        time < (self.entry_time + cooldown_time)
    }

    pub fn remaining(&self, time: u32, cooldown_time: u32) -> u32 {
        self.entry_time + cooldown_time - time
    }
}

pub struct State {
    changed: bool,
    pub ble_connected: bool,
    pub ble_name: &'static str,
    pub running: bool,
    pub stimulating: bool,
    pub peak_area_threshold: u32,
    pub peak_value_thresh: u32,
    pub peak_release_time_thresh: u32,
    pub cooldown_time: u32,
    pub intensity: u8,
    pub hysteresis: Hysteresis,
    pub cur_time_ms: u32,
}

impl State {
    pub fn new() -> State {
        State {
            changed: true,
            ble_connected: false,
            ble_name: "n/a",
            running: false,
            stimulating: false,
            peak_area_threshold: 200_000,
            peak_value_thresh: 15_000,
            peak_release_time_thresh: 500,
            cooldown_time: 10_000,
            intensity: 10,
            hysteresis: Hysteresis::new(),
            cur_time_ms: 0,
        }
    }
    pub fn has_changed(&mut self) -> bool {
        let changed = self.changed;
        self.changed = false;
        changed || (self.running && !self.stimulating)
    }

    pub fn set_ble_connected(&mut self, connected: bool) {
        if connected == self.ble_connected {
            return;
        }
        self.ble_connected = connected;
        self.changed = true;
    }

    pub fn set_ble_name(&mut self, name: &'static str) {
        if name == self.ble_name {
            return;
        }
        self.ble_name = name;
        self.changed = true;
    }

    pub fn area_up(&mut self) {
        if !(self.peak_area_threshold >= MAX_AREA) {
            self.peak_area_threshold += AREA_STEP;
        }
    }
    pub fn area_down(&mut self) {
        self.peak_area_threshold = self.peak_area_threshold.saturating_sub(AREA_STEP);
    }
    pub fn peak_up(&mut self) {
        if !(self.peak_value_thresh >= MAX_PEAK) {
            self.peak_value_thresh += PEAK_STEP;
        }
    }
    pub fn peak_down(&mut self) {
        self.peak_value_thresh = self.peak_value_thresh.saturating_sub(PEAK_STEP);
    }
    pub fn duration_up(&mut self) {
        if !(self.peak_release_time_thresh >= MAX_DURATION) {
            self.peak_release_time_thresh += DURATION_STEP;
        }
    }
    pub fn duration_down(&mut self) {
        self.peak_release_time_thresh = self.peak_release_time_thresh.saturating_sub(DURATION_STEP);
    }
    pub fn cooldown_up(&mut self) {
        self.cooldown_time += COOLDOWN_STEP;
    }
    pub fn cooldown_down(&mut self) {
        if self.cooldown_time == 0 {
            return;
        }
        self.cooldown_time -= COOLDOWN_STEP;
    }
    pub fn toggle(&mut self) {
        self.running = !self.running;
        self.stimulating = self.running;
    }
    pub fn stop_stim(&mut self) {
        if !self.stimulating {
            return;
        }
        self.stimulating = false;
        self.changed = true;
    }
    pub fn start_stim(&mut self) {
        if self.stimulating {
            return;
        }
        self.stimulating = true;
        self.changed = true;
    }
    pub fn stop_stim_manual(&mut self) {
        if !self.running {
            self.stop_stim();
        }
    }
    pub fn start_stim_manual(&mut self) {
        if !self.running {
            self.start_stim();
        }
    }
    pub fn intensity_up(&mut self) {
        if !(self.intensity >= MAX_INTENSITY) {
            self.intensity += 1;
        }
    }
    pub fn intensity_down(&mut self) {
        if self.intensity == 0 {
            return;
        }
        self.intensity -= 1;
        if self.stimulating {}
    }

    pub fn get_cur_intensity(&self) -> u8 {
        if !self.stimulating {
            return 0;
        }
        return self.intensity;
    }
}
