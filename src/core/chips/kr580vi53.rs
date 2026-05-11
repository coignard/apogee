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

use serde::{Deserialize, Serialize};

const MAX_COUNT_INTERNAL: u32 = 0x10000;
const COUNT_MASK: u32 = 0xFFFF;
const BYTE_MASK: u16 = 0xFF;
const MSB_SHIFT: usize = 8;

const PORT_ADDRESS_MASK: u16 = 0x03;
const PORT_CONTROL_WORD: u16 = 0x03;

const CONTROL_WORD_CHANNEL_MASK: u8 = 0b1100_0000;
const CONTROL_WORD_CHANNEL_SHIFT: u8 = 6;
const CONTROL_WORD_ACCESS_MASK: u8 = 0b0011_0000;
const CONTROL_WORD_ACCESS_SHIFT: u8 = 4;
const CONTROL_WORD_MODE_MASK: u8 = 0b0000_1110;
const CONTROL_WORD_MODE_SHIFT: u8 = 1;
const CONTROL_WORD_BCD_MASK: u8 = 0b0000_0001;

const ACCESS_LATCH_COMMAND: u8 = 0b0000_0000;
const ACCESS_LSB_ONLY: u8 = 0b0000_0001;
const ACCESS_MSB_ONLY: u8 = 0b0000_0010;
const ACCESS_LSB_THEN_MSB: u8 = 0b0000_0011;

const MODE_0: u8 = 0b000;
const MODE_1: u8 = 0b001;
const MODE_2_ALT_1: u8 = 0b010;
const MODE_2_ALT_2: u8 = 0b110;
const MODE_3_ALT_1: u8 = 0b011;
const MODE_3_ALT_2: u8 = 0b111;
const MODE_4: u8 = 0b100;
const MODE_5: u8 = 0b101;

const DECREMENT_SQUARE_WAVE: u32 = 2;
const DECREMENT_NORMAL: u32 = 1;

const CHANNELS_COUNT: usize = 3;

struct BcdCorrection {
    mask: u32,
    limit: u32,
    correction: u32,
}

const BCD_CORRECTIONS: [BcdCorrection; 4] = [
    BcdCorrection {
        mask: 0x000F,
        limit: 0x0009,
        correction: 0x0006,
    },
    BcdCorrection {
        mask: 0x00F0,
        limit: 0x0090,
        correction: 0x0060,
    },
    BcdCorrection {
        mask: 0x0F00,
        limit: 0x0900,
        correction: 0x0600,
    },
    BcdCorrection {
        mask: 0xF000,
        limit: 0x9000,
        correction: 0x6000,
    },
];

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum TimerMode {
    #[default]
    InterruptOnTerminalCount = 0,
    HardwareRetriggerableOneShot = 1,
    RateGenerator = 2,
    SquareWave = 3,
    SoftwareTriggeredStrobe = 4,
    HardwareTriggeredStrobe = 5,
}

impl TimerMode {
    fn from_control_word_bits(val: u8) -> Self {
        match val {
            MODE_0 => Self::InterruptOnTerminalCount,
            MODE_1 => Self::HardwareRetriggerableOneShot,
            MODE_2_ALT_1 | MODE_2_ALT_2 => Self::RateGenerator,
            MODE_3_ALT_1 | MODE_3_ALT_2 => Self::SquareWave,
            MODE_4 => Self::SoftwareTriggeredStrobe,
            MODE_5 => Self::HardwareTriggeredStrobe,
            _ => Self::InterruptOnTerminalCount,
        }
    }

    fn is_gate_level_sensitive(self) -> bool {
        matches!(
            self,
            Self::InterruptOnTerminalCount
                | Self::RateGenerator
                | Self::SquareWave
                | Self::SoftwareTriggeredStrobe
        )
    }

    fn default_out_state(self) -> bool {
        !matches!(self, Self::InterruptOnTerminalCount)
    }
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum RwMode {
    #[default]
    LsbOnly,
    MsbOnly,
    LsbThenMsb,
}

impl RwMode {
    fn from_control_word_bits(val: u8) -> Self {
        match val {
            ACCESS_LSB_ONLY => Self::LsbOnly,
            ACCESS_MSB_ONLY => Self::MsbOnly,
            ACCESS_LSB_THEN_MSB => Self::LsbThenMsb,
            _ => Self::LsbOnly,
        }
    }
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum RwPhase {
    #[default]
    Lsb,
    Msb,
}

impl RwPhase {
    fn toggle(&mut self) {
        *self = match self {
            Self::Lsb => Self::Msb,
            Self::Msb => Self::Lsb,
        };
    }
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum ChannelState {
    #[default]
    Unprogrammed,
    WaitLoad,
    WaitTrigger,
    Counting,
}

#[derive(Clone, Copy, Default, PartialEq, Serialize, Deserialize)]
pub enum LatchState {
    #[default]
    None,
    LsbOnly(u8),
    MsbOnly(u8),
    LsbThenMsb(u8, u8),
}

#[inline(always)]
fn decrement_binary_value(val: u32, decrement_steps: u32) -> u32 {
    val.wrapping_sub(decrement_steps) & COUNT_MASK
}

#[inline(always)]
fn decrement_bcd_value(mut val: u32, decrement_steps: u32) -> u32 {
    for _ in 0..decrement_steps {
        val = val.wrapping_sub(DECREMENT_NORMAL);
        for c in &BCD_CORRECTIONS {
            if (val & c.mask) > c.limit {
                val = val.wrapping_sub(c.correction);
            }
        }
    }
    val & COUNT_MASK
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct TimerChannel {
    mode: TimerMode,
    rw_mode: RwMode,
    is_bcd: bool,
    state: ChannelState,
    rw_phase: RwPhase,
    latch: LatchState,
    reload_value: u16,
    working_counter: u32,
    out_pin: bool,
    gate_pin: bool,
    reload_pending: bool,
    strobe_fired: bool,
}

impl Default for TimerChannel {
    fn default() -> Self {
        Self {
            mode: TimerMode::default(),
            rw_mode: RwMode::default(),
            is_bcd: false,
            state: ChannelState::Unprogrammed,
            rw_phase: RwPhase::default(),
            latch: LatchState::None,
            reload_value: 0,
            working_counter: 0,
            out_pin: true,
            gate_pin: true,
            reload_pending: false,
            strobe_fired: false,
        }
    }
}

impl TimerChannel {
    fn new() -> Self {
        Self::default()
    }

    fn effective_reload_value(&self) -> u32 {
        if self.reload_value == 0 {
            MAX_COUNT_INTERNAL
        } else {
            self.reload_value as u32
        }
    }

    fn trigger_load(&mut self) {
        match self.mode {
            TimerMode::HardwareRetriggerableOneShot | TimerMode::HardwareTriggeredStrobe => {
                if self.state == ChannelState::WaitLoad {
                    self.state = ChannelState::WaitTrigger;
                }
            }
            TimerMode::RateGenerator | TimerMode::SquareWave => {
                if self.state != ChannelState::Counting {
                    self.reload_pending = true;
                }
            }
            TimerMode::InterruptOnTerminalCount | TimerMode::SoftwareTriggeredStrobe => {
                self.reload_pending = true;
            }
        }
    }

    pub fn set_gate(&mut self, state: bool) {
        let rising_edge = state && !self.gate_pin;
        let falling_edge = !state && self.gate_pin;
        self.gate_pin = state;

        if rising_edge {
            if matches!(
                self.mode,
                TimerMode::HardwareRetriggerableOneShot
                    | TimerMode::HardwareTriggeredStrobe
                    | TimerMode::RateGenerator
                    | TimerMode::SquareWave
            ) {
                self.reload_pending = true;
            }
        } else if falling_edge
            && matches!(self.mode, TimerMode::RateGenerator | TimerMode::SquareWave)
        {
            self.out_pin = true;
        }
    }

    fn tick(&mut self) {
        if self.state == ChannelState::Unprogrammed {
            return;
        }

        if !self.gate_pin && self.mode.is_gate_level_sensitive() {
            return;
        }

        if self.reload_pending {
            self.reload_pending = false;
            self.working_counter = self.effective_reload_value();
            self.state = ChannelState::Counting;
            self.strobe_fired = false;

            match self.mode {
                TimerMode::InterruptOnTerminalCount | TimerMode::HardwareRetriggerableOneShot => {
                    self.out_pin = false;
                }
                _ => {
                    self.out_pin = true;
                }
            }
            return;
        }

        if self.state != ChannelState::Counting {
            return;
        }

        if self.mode == TimerMode::SquareWave {
            if self.working_counter <= DECREMENT_SQUARE_WAVE {
                self.out_pin = !self.out_pin;
                let reload_val = self.effective_reload_value();
                let is_odd_reload_value = !reload_val.is_multiple_of(DECREMENT_SQUARE_WAVE);

                if is_odd_reload_value {
                    self.working_counter = if self.out_pin {
                        reload_val
                    } else {
                        reload_val.wrapping_sub(DECREMENT_NORMAL)
                    };
                } else {
                    self.working_counter = reload_val;
                }
            } else {
                self.working_counter = if self.is_bcd {
                    decrement_bcd_value(self.working_counter, DECREMENT_SQUARE_WAVE)
                } else {
                    decrement_binary_value(self.working_counter, DECREMENT_SQUARE_WAVE)
                };
            }
            return;
        }

        let prev_counter = self.working_counter;
        self.working_counter = if self.is_bcd {
            decrement_bcd_value(self.working_counter, DECREMENT_NORMAL)
        } else {
            decrement_binary_value(self.working_counter, DECREMENT_NORMAL)
        };
        let zero_hit = self.working_counter == 0;

        match self.mode {
            TimerMode::InterruptOnTerminalCount | TimerMode::HardwareRetriggerableOneShot => {
                if zero_hit {
                    self.out_pin = true;
                }
            }
            TimerMode::RateGenerator => {
                if self.working_counter == DECREMENT_NORMAL {
                    self.out_pin = false;
                } else if zero_hit {
                    self.out_pin = true;
                    self.working_counter = self.effective_reload_value();
                }
            }
            TimerMode::SoftwareTriggeredStrobe | TimerMode::HardwareTriggeredStrobe => {
                if zero_hit && !self.strobe_fired {
                    self.out_pin = false;
                } else if prev_counter == 0 && !self.out_pin {
                    self.out_pin = true;
                    self.strobe_fired = true;
                }
            }
            TimerMode::SquareWave => unreachable!(),
        }
    }
}

#[derive(Serialize, Deserialize)]
pub struct Kr580Vi53 {
    channels: [TimerChannel; CHANNELS_COUNT],
}

impl Kr580Vi53 {
    pub fn new() -> Self {
        Self {
            channels: [TimerChannel::new(); CHANNELS_COUNT],
        }
    }

    pub fn set_gate(&mut self, channel_index: usize, state: bool) {
        if channel_index < CHANNELS_COUNT {
            self.channels[channel_index].set_gate(state);
        }
    }

    pub fn read(&mut self, port: u16) -> u8 {
        let channel_index = (port & PORT_ADDRESS_MASK) as usize;
        if channel_index >= CHANNELS_COUNT {
            return u8::MAX;
        }

        let ch = &mut self.channels[channel_index];

        match ch.latch {
            LatchState::LsbOnly(lo) => {
                ch.latch = LatchState::None;
                lo
            }
            LatchState::MsbOnly(hi) => {
                ch.latch = LatchState::None;
                hi
            }
            LatchState::LsbThenMsb(lo, hi) => {
                ch.latch = LatchState::MsbOnly(hi);
                lo
            }
            LatchState::None => {
                let current_val = (ch.working_counter & COUNT_MASK) as u16;
                let lo = (current_val & BYTE_MASK) as u8;
                let hi = (current_val >> MSB_SHIFT) as u8;

                match ch.rw_mode {
                    RwMode::LsbOnly => lo,
                    RwMode::MsbOnly => hi,
                    RwMode::LsbThenMsb => {
                        let val = if ch.rw_phase == RwPhase::Lsb { lo } else { hi };
                        ch.rw_phase.toggle();
                        val
                    }
                }
            }
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        let port_clean = port & PORT_ADDRESS_MASK;

        if port_clean == PORT_CONTROL_WORD {
            let channel_index = (val & CONTROL_WORD_CHANNEL_MASK) >> CONTROL_WORD_CHANNEL_SHIFT;
            if channel_index >= CHANNELS_COUNT as u8 {
                return;
            }

            let ch = &mut self.channels[channel_index as usize];
            let rw_bits = (val & CONTROL_WORD_ACCESS_MASK) >> CONTROL_WORD_ACCESS_SHIFT;

            if rw_bits == ACCESS_LATCH_COMMAND {
                if ch.latch == LatchState::None {
                    let current_val = (ch.working_counter & COUNT_MASK) as u16;
                    let lo = (current_val & BYTE_MASK) as u8;
                    let hi = (current_val >> MSB_SHIFT) as u8;

                    ch.latch = match ch.rw_mode {
                        RwMode::LsbOnly => LatchState::LsbOnly(lo),
                        RwMode::MsbOnly => LatchState::MsbOnly(hi),
                        RwMode::LsbThenMsb => LatchState::LsbThenMsb(lo, hi),
                    };
                }
            } else {
                let mode_bits = (val & CONTROL_WORD_MODE_MASK) >> CONTROL_WORD_MODE_SHIFT;

                ch.rw_mode = RwMode::from_control_word_bits(rw_bits);
                ch.mode = TimerMode::from_control_word_bits(mode_bits);
                ch.is_bcd = (val & CONTROL_WORD_BCD_MASK) != 0;

                ch.state = ChannelState::WaitLoad;
                ch.rw_phase = RwPhase::Lsb;
                ch.latch = LatchState::None;
                ch.out_pin = ch.mode.default_out_state();
                ch.reload_pending = false;
                ch.strobe_fired = false;
            }
        } else {
            let ch = &mut self.channels[port_clean as usize];
            let val_u16 = val as u16;

            match ch.rw_mode {
                RwMode::LsbOnly => {
                    ch.reload_value = (ch.reload_value & !BYTE_MASK) | val_u16;
                    ch.trigger_load();
                }
                RwMode::MsbOnly => {
                    ch.reload_value = (ch.reload_value & BYTE_MASK) | (val_u16 << MSB_SHIFT);
                    ch.trigger_load();
                }
                RwMode::LsbThenMsb => {
                    if ch.rw_phase == RwPhase::Lsb {
                        ch.reload_value = (ch.reload_value & !BYTE_MASK) | val_u16;
                        ch.rw_phase = RwPhase::Msb;

                        if ch.mode == TimerMode::InterruptOnTerminalCount {
                            ch.state = ChannelState::WaitLoad;
                        }
                    } else {
                        ch.reload_value = (ch.reload_value & BYTE_MASK) | (val_u16 << MSB_SHIFT);
                        ch.rw_phase = RwPhase::Lsb;
                        ch.trigger_load();
                    }
                }
            }
        }
    }

    pub fn tick(&mut self) -> i32 {
        let mut mixed_output = 0;
        for ch in self.channels.iter_mut() {
            ch.tick();
            mixed_output += if ch.out_pin { 1 } else { -1 };
        }
        mixed_output
    }
}

impl Default for Kr580Vi53 {
    fn default() -> Self {
        Self::new()
    }
}
