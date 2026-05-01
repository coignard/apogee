// This file is part of Apogee.
//
// Copyright (c) 2026  René Coignard <contact@renecoignard.com>
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use serde::Serialize;

#[derive(Clone, Copy, Default, PartialEq, Serialize)]
pub enum PitRwMode {
    #[default]
    Latch,
    LsbOnly,
    MsbOnly,
    LsbThenMsb,
}

impl PitRwMode {
    fn from_u8(val: u8) -> Self {
        match val {
            1 => Self::LsbOnly,
            2 => Self::MsbOnly,
            3 => Self::LsbThenMsb,
            _ => Self::Latch,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Serialize)]
pub enum PitPhase {
    #[default]
    Lsb,
    Msb,
}

const PIT_MAX_COUNT: u32 = 0x10000;
const PORT_MASK: u16 = 3;
const PORT_CWR: u16 = 3;

#[derive(Clone, Copy, Default, Serialize)]
struct TimerChannel {
    mode: u8,
    rw_mode: PitRwMode,
    reload: u16,
    reload_latch: u16,
    reload_pending: bool,
    counter: u32,
    latch: u16,
    phase: PitPhase,
    out: bool,
    latched: bool,
    counting: bool,
}

impl TimerChannel {
    fn trigger_load(&mut self) {
        if self.mode == 0 || !self.counting {
            self.reload_pending = true;
            self.counting = true;
        }
    }

    fn tick(&mut self) {
        if !self.counting {
            return;
        }

        let eff_val = if self.reload == 0 {
            PIT_MAX_COUNT
        } else {
            self.reload as u32
        };

        if self.reload_pending {
            self.reload_pending = false;
            if self.mode == 3 || self.mode == 7 {
                self.out = true;
                self.counter = match eff_val {
                    1 => 32769,
                    3 => 2,
                    _ => eff_val.div_ceil(2),
                };
            } else if self.mode == 0 {
                self.out = false;
                self.counter = eff_val;
            } else {
                self.out = true;
                self.counter = eff_val;
            }
            return;
        }

        if self.mode == 3 || self.mode == 7 {
            if self.counter == 0 {
                self.out = !self.out;
                self.counter = if self.out {
                    match eff_val {
                        1 => 32769,
                        3 => 2,
                        _ => eff_val.div_ceil(2),
                    }
                } else {
                    match eff_val {
                        1 => 32768,
                        3 => 32769,
                        _ => eff_val / 2,
                    }
                };
            }
            self.counter = self.counter.saturating_sub(1);
        } else if self.mode == 0 {
            if self.counter > 0 {
                self.counter -= 1;
                if self.counter == 0 {
                    self.out = true;
                }
            }
        } else {
            self.counter = self.counter.saturating_sub(1);
            if self.counter == 0 {
                self.counter = eff_val;
                self.out = true;
            } else {
                self.out = self.counter != 1;
            }
        }
    }
}

#[derive(Serialize)]
pub struct Kr580Vi53 {
    channels: [TimerChannel; 3],
}

impl Kr580Vi53 {
    pub fn new() -> Self {
        Self {
            channels: [TimerChannel::default(); 3],
        }
    }

    pub fn read(&mut self, port: u16) -> u8 {
        let ch_idx = (port & PORT_MASK) as usize;
        if ch_idx < 3 {
            let ch = &mut self.channels[ch_idx];

            let val = if ch.latched {
                ch.latch
            } else {
                let mut v = ch.counter;
                if ch.mode == 3 || ch.mode == 7 {
                    v *= 2;
                }
                v as u16
            };

            match ch.rw_mode {
                PitRwMode::LsbOnly => {
                    ch.latched = false;
                    (val & 0xFF) as u8
                }
                PitRwMode::MsbOnly => {
                    ch.latched = false;
                    (val >> 8) as u8
                }
                PitRwMode::LsbThenMsb => {
                    if ch.phase == PitPhase::Lsb {
                        ch.phase = PitPhase::Msb;
                        (val & 0xFF) as u8
                    } else {
                        ch.phase = PitPhase::Lsb;
                        ch.latched = false;
                        (val >> 8) as u8
                    }
                }
                PitRwMode::Latch => 0xFF,
            }
        } else {
            0xFF
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        let port_clean = port & PORT_MASK;
        if port_clean < PORT_CWR {
            let ch_idx = port_clean as usize;
            let ch = &mut self.channels[ch_idx];
            match ch.rw_mode {
                PitRwMode::LsbOnly => {
                    ch.reload_latch = (ch.reload_latch & 0xFF00) | (val as u16);
                    ch.reload = ch.reload_latch;
                    ch.trigger_load();
                }
                PitRwMode::MsbOnly => {
                    ch.reload_latch = (ch.reload_latch & 0x00FF) | ((val as u16) << 8);
                    ch.reload = ch.reload_latch;
                    ch.trigger_load();
                }
                PitRwMode::LsbThenMsb => {
                    if ch.phase == PitPhase::Lsb {
                        ch.reload_latch = (ch.reload_latch & 0xFF00) | (val as u16);
                        ch.phase = PitPhase::Msb;
                        if ch.mode == 0 {
                            ch.counting = false;
                        }
                    } else {
                        ch.reload_latch = (ch.reload_latch & 0x00FF) | ((val as u16) << 8);
                        ch.reload = ch.reload_latch;
                        ch.phase = PitPhase::Lsb;
                        ch.trigger_load();
                    }
                }
                PitRwMode::Latch => {}
            }
        } else if port_clean == PORT_CWR {
            let ch_idx = ((val >> 6) & 3) as usize;
            if ch_idx < 3 {
                let ch = &mut self.channels[ch_idx];
                let rw_mode_val = (val >> 4) & 3;
                if rw_mode_val != 0 {
                    ch.rw_mode = PitRwMode::from_u8(rw_mode_val);
                    ch.mode = (val >> 1) & 7;
                    ch.phase = PitPhase::Lsb;
                    ch.counting = false;
                    ch.out = ch.mode != 0;
                } else if !ch.latched {
                    let mut visual_counter = ch.counter;
                    if ch.mode == 3 || ch.mode == 7 {
                        visual_counter *= 2;
                    }
                    ch.latch = visual_counter as u16;
                    ch.latched = true;
                }
            }
        }
    }

    pub fn tick(&mut self) -> i32 {
        let mut mixed = 0;
        for ch in self.channels.iter_mut() {
            ch.tick();
        }
        for ch in self.channels.iter() {
            mixed += if ch.out { 1 } else { -1 };
        }
        mixed
    }
}

impl Default for Kr580Vi53 {
    fn default() -> Self {
        Self::new()
    }
}
