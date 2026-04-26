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

#[derive(Clone, Copy, Default)]
struct TimerChannel {
    mode: u8,
    rw_mode: u8,
    reload: u16,
    counter: u32,
    latch: u16,
    phase: u8,
    out: bool,
    latched: bool,
    counting: bool,
    reload_pending: bool,
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
            0x10000
        } else {
            self.reload as u32
        };

        if self.reload_pending {
            self.reload_pending = false;
            if self.mode == 3 || self.mode == 7 {
                self.out = true;
                self.counter = eff_val.div_ceil(2);
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
                    eff_val.div_ceil(2)
                } else {
                    eff_val / 2
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
        let ch_idx = (port & 3) as usize;
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
                1 => {
                    ch.latched = false;
                    (val & 0xFF) as u8
                }
                2 => {
                    ch.latched = false;
                    (val >> 8) as u8
                }
                3 => {
                    if ch.phase == 0 {
                        ch.phase = 1;
                        (val & 0xFF) as u8
                    } else {
                        ch.phase = 0;
                        ch.latched = false;
                        (val >> 8) as u8
                    }
                }
                _ => 0xFF,
            }
        } else {
            0xFF
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        let port_clean = port & 3;
        match port_clean {
            0..=2 => {
                let ch_idx = port_clean as usize;
                let ch = &mut self.channels[ch_idx];
                match ch.rw_mode {
                    1 => {
                        ch.reload = (ch.reload & 0xFF00) | (val as u16);
                        ch.trigger_load();
                    }
                    2 => {
                        ch.reload = (ch.reload & 0x00FF) | ((val as u16) << 8);
                        ch.trigger_load();
                    }
                    3 => {
                        if ch.phase == 0 {
                            ch.reload = (ch.reload & 0xFF00) | (val as u16);
                            ch.phase = 1;
                            if ch.mode == 0 {
                                ch.counting = false;
                            }
                        } else {
                            ch.reload = (ch.reload & 0x00FF) | ((val as u16) << 8);
                            ch.phase = 0;
                            ch.trigger_load();
                        }
                    }
                    _ => {}
                }
            }
            3 => {
                let ch_idx = ((val >> 6) & 3) as usize;
                if ch_idx < 3 {
                    let ch = &mut self.channels[ch_idx];
                    let rw_mode = (val >> 4) & 3;
                    if rw_mode != 0 {
                        ch.rw_mode = rw_mode;
                        ch.mode = (val >> 1) & 7;
                        ch.phase = 0;
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
            _ => {}
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
