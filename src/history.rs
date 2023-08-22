use crate::{avg::RunningAverage, state::State};
use log::debug;

#[derive(Copy, Clone)]
#[cfg_attr(test, derive(Debug, PartialEq))]
enum PeakState {
    None,
    In {
        area: u32,
    },
    Exiting {
        peak_area: u32,
        exiting_area: u32,
        exit_time: u32,
    },
}

pub enum HistoryResult {
    Stop,
    Resume,
}

pub struct Nogasm<const AVG_SAMPLES: usize> {
    avg: RunningAverage<AVG_SAMPLES>,
    pub min: u32,
    pub min_decay: u32,
    state: PeakState,
}

impl<const AVG_SAMPLES: usize> Nogasm<{ AVG_SAMPLES }> {
    pub fn new() -> Nogasm<AVG_SAMPLES> {
        Nogasm {
            avg: RunningAverage::new(),
            min: u32::MAX,
            min_decay: u32::MAX,
            state: PeakState::None,
        }
    }

    pub fn add(&mut self, val: u32, time: u32, state: &mut State) -> HistoryResult {
        debug!("New value: {}", val);
        use PeakState::*;
        self.avg.add(val);
        if self.min > val {
            self.min = val;
        }
        if self.min_decay == u32::MAX {
            self.min_decay = val;
        }
        self.min_decay = (self.min_decay * 199 + val) / 200;
        // let val = val - self.min;

        let cur = self.get_current_value() as u32;

        debug!("Current value: {}", val);
        // let cur = val as u32;

        self.state = match self.state {
            None if cur >= state.peak_value_thresh => In { area: cur },
            In { area } if area > state.peak_area_threshold => {
                debug!("Max area reached");
                self.min = u32::MAX;
                state.hysteresis.enter(time);
                None
            }
            In { area } if cur >= state.peak_value_thresh => {
                debug!("Entering peak");
                In { area: area + cur }
            }
            In { area } => {
                debug!("Exiting peak");
                Exiting {
                    peak_area: area,
                    exiting_area: cur,
                    exit_time: time,
                }
            }
            Exiting {
                peak_area,
                exiting_area,
                exit_time,
            } => {
                if cur >= state.peak_value_thresh {
                    debug!("Back in peak");
                    In {
                        area: peak_area + exiting_area + cur,
                    }
                } else if time.wrapping_sub(exit_time) >= state.peak_release_time_thresh {
                    debug!("Out of peak");
                    None
                } else {
                    Exiting {
                        peak_area,
                        exiting_area: exiting_area + cur,
                        exit_time,
                    }
                }
            }
            default => default,
        };

        let stop = state.hysteresis.is_active(time, state.cooldown_time);
        debug!("Hysteresis result (should stop?): {}", stop);

        if stop {
            HistoryResult::Stop
        } else {
            HistoryResult::Resume
        }
    }

    pub fn get_current_value(&self) -> u32 {
        self.avg.get().saturating_sub(self.min_decay)
    }

    pub fn get_area(&self) -> u32 {
        use PeakState::*;
        match self.state {
            In { area } => area,
            Exiting {
                peak_area,
                exiting_area: _,
                exit_time: _,
            } => peak_area,
            _ => 0,
        }
    }
}
