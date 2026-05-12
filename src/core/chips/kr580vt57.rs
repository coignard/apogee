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

const NUM_CHANNELS: usize = 4;
const CHANNEL_2: usize = 2;
const CHANNEL_3: usize = 3;

const COUNT_MASK: u16 = 0x3FFF;
const COUNT_MODE_PRESERVE_MASK: u16 = 0xC000;
const OPERATION_MODE_SHIFT: u16 = 14;

const OP_MODE_VERIFY: u16 = 0;
const OP_MODE_WRITE: u16 = 1;
const OP_MODE_READ: u16 = 2;

const LOWER_BYTE_MASK: u16 = 0x00FF;
const UPPER_BYTE_MASK: u16 = 0xFF00;
const BYTE_SHIFT: u16 = 8;

const PORT_A3_MASK: u16 = 0x08;
const PORT_A2_A1_MASK: u16 = 0x06;
const PORT_A2_A1_SHIFT: u16 = 1;
const PORT_A0_MASK: u16 = 0x01;

const MODE_ROTATING_PRIORITY: u8 = 0x10;
#[allow(dead_code)]
const MODE_EXTENDED_WRITE: u8 = 0x20;
const MODE_TC_STOP: u8 = 0x40;
const MODE_AUTO_LOAD: u8 = 0x80;

const STATUS_TC_MASK: u8 = 0x0F;
const STATUS_UPDATE_FLAG: u8 = 0x10;

#[derive(Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum BytePhase {
    #[default]
    Lsb,
    Msb,
}

impl BytePhase {
    #[inline]
    pub fn toggle(&mut self) {
        *self = match self {
            Self::Lsb => Self::Msb,
            Self::Msb => Self::Lsb,
        };
    }

    #[inline]
    pub fn reset(&mut self) {
        *self = Self::Lsb;
    }

    #[inline]
    pub fn apply_to_u16(self, current: u16, byte: u8) -> u16 {
        let byte16 = byte as u16;
        match self {
            Self::Lsb => (current & UPPER_BYTE_MASK) | byte16,
            Self::Msb => (current & LOWER_BYTE_MASK) | (byte16 << BYTE_SHIFT),
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DmaOperation {
    Verify,
    Write,
    Read,
    Illegal,
}

impl DmaOperation {
    #[inline]
    pub fn from_count_register(count: u16) -> Self {
        match count >> OPERATION_MODE_SHIFT {
            OP_MODE_VERIFY => Self::Verify,
            OP_MODE_WRITE => Self::Write,
            OP_MODE_READ => Self::Read,
            _ => Self::Illegal,
        }
    }
}

#[derive(Clone, Copy, Default, Serialize, Deserialize)]
pub struct DmaChannel {
    pub address: u16,
    pub count: u16,
}

#[derive(Serialize, Deserialize)]
pub struct Kr580Vt57 {
    channels: [DmaChannel; NUM_CHANNELS],
    mode: u8,
    status: u8,
    byte_phase: BytePhase,
    drq: [bool; NUM_CHANNELS],
    hlda: bool,
    last_serviced_channel: usize,
    halt_cycles: u32,
}

impl Kr580Vt57 {
    pub fn new() -> Self {
        Self {
            channels: [DmaChannel::default(); NUM_CHANNELS],
            mode: 0,
            status: 0,
            byte_phase: BytePhase::default(),
            drq: [false; NUM_CHANNELS],
            hlda: false,
            last_serviced_channel: NUM_CHANNELS - 1,
            halt_cycles: 0,
        }
    }

    pub fn hardware_reset(&mut self) {
        self.mode = 0;
        self.status = 0;
        self.byte_phase.reset();
        self.drq = [false; NUM_CHANNELS];
        self.hlda = false;
        self.last_serviced_channel = NUM_CHANNELS - 1;
        self.halt_cycles = 0;
    }

    #[inline]
    pub fn is_enabled(&self, channel: usize) -> bool {
        channel < NUM_CHANNELS && (self.mode & (1 << channel)) != 0
    }

    pub fn operation_type(&self, channel: usize) -> DmaOperation {
        if channel >= NUM_CHANNELS {
            return DmaOperation::Illegal;
        }
        DmaOperation::from_count_register(self.channels[channel].count)
    }

    #[inline]
    pub fn set_drq(&mut self, channel: usize, state: bool) {
        if channel < NUM_CHANNELS {
            self.drq[channel] = state;
        }
    }

    #[inline]
    pub fn hrq(&self) -> bool {
        for ch in 0..NUM_CHANNELS {
            if self.is_enabled(ch) && self.drq[ch] {
                return true;
            }
        }
        false
    }

    #[inline]
    pub fn set_hlda(&mut self, state: bool) {
        self.hlda = state;
    }

    #[inline]
    pub fn halt_cycles(&self) -> u32 {
        self.halt_cycles
    }

    #[inline]
    pub fn add_halt_cycles(&mut self, cycles: u32) {
        self.halt_cycles += cycles;
    }

    #[inline]
    pub fn sub_halt_cycles(&mut self, cycles: u32) {
        self.halt_cycles = self.halt_cycles.saturating_sub(cycles);
    }

    #[inline]
    pub fn ch2_addr(&self) -> u16 {
        if (self.status & STATUS_UPDATE_FLAG) != 0 {
            self.channels[CHANNEL_3].address
        } else {
            self.channels[CHANNEL_2].address
        }
    }

    #[inline]
    pub fn step_ch2(&mut self) {
        let _ = self.step_channel(CHANNEL_2);
    }

    fn step_channel(&mut self, ch: usize) -> (u16, bool) {
        let is_update_cycle = ch == CHANNEL_2 && (self.status & STATUS_UPDATE_FLAG) != 0;

        if is_update_cycle {
            self.channels[CHANNEL_2] = self.channels[CHANNEL_3];
            self.status &= !STATUS_UPDATE_FLAG;
        }

        let addr = self.channels[ch].address;
        self.channels[ch].address = self.channels[ch].address.wrapping_add(1);

        let count_val = self.channels[ch].count & COUNT_MASK;
        let is_tc = count_val == 0;

        let new_count = count_val.wrapping_sub(1) & COUNT_MASK;
        self.channels[ch].count = (self.channels[ch].count & COUNT_MODE_PRESERVE_MASK) | new_count;

        if is_tc {
            self.status |= 1 << (ch as u8);

            let mut disable_channel = (self.mode & MODE_TC_STOP) != 0;

            if ch == CHANNEL_2 && (self.mode & MODE_AUTO_LOAD) != 0 {
                disable_channel = false;
                self.status |= STATUS_UPDATE_FLAG;
            }

            if disable_channel {
                self.mode &= !(1 << (ch as u8));
            }
        }

        self.last_serviced_channel = ch;
        (addr, is_tc)
    }

    pub fn dma_transfer_cycle(&mut self) -> Option<(usize, u16, bool)> {
        if !self.hlda {
            return None;
        }

        let priority_base = if (self.mode & MODE_ROTATING_PRIORITY) != 0 {
            (self.last_serviced_channel + 1) % NUM_CHANNELS
        } else {
            0
        };

        for offset in 0..NUM_CHANNELS {
            let ch = (priority_base + offset) % NUM_CHANNELS;

            if self.is_enabled(ch) && self.drq[ch] {
                let (addr, is_tc) = self.step_channel(ch);
                return Some((ch, addr, is_tc));
            }
        }

        None
    }

    pub fn read(&mut self, port: u16) -> u8 {
        if (port & PORT_A3_MASK) != 0 {
            let current_status = self.status;
            self.status &= !STATUS_TC_MASK;
            current_status
        } else {
            let channel = ((port & PORT_A2_A1_MASK) >> PORT_A2_A1_SHIFT) as usize;
            let is_count = (port & PORT_A0_MASK) != 0;

            let val16 = if is_count {
                self.channels[channel].count
            } else {
                self.channels[channel].address
            };

            let result = match self.byte_phase {
                BytePhase::Lsb => (val16 & LOWER_BYTE_MASK) as u8,
                BytePhase::Msb => (val16 >> BYTE_SHIFT) as u8,
            };

            self.byte_phase.toggle();
            result
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        if (port & PORT_A3_MASK) != 0 {
            self.mode = val;
            if (self.mode & MODE_AUTO_LOAD) == 0 {
                self.status &= !STATUS_UPDATE_FLAG;
            }
            self.byte_phase.reset();
        } else {
            let channel = ((port & PORT_A2_A1_MASK) >> PORT_A2_A1_SHIFT) as usize;
            let is_count = (port & PORT_A0_MASK) != 0;

            if is_count {
                self.channels[channel].count = self
                    .byte_phase
                    .apply_to_u16(self.channels[channel].count, val);

                if channel == CHANNEL_2 && (self.mode & MODE_AUTO_LOAD) != 0 {
                    self.channels[CHANNEL_3].count = self.channels[CHANNEL_2].count;
                }
            } else {
                self.channels[channel].address = self
                    .byte_phase
                    .apply_to_u16(self.channels[channel].address, val);

                if channel == CHANNEL_2 && (self.mode & MODE_AUTO_LOAD) != 0 {
                    self.channels[CHANNEL_3].address = self.channels[CHANNEL_2].address;
                }
            }

            self.byte_phase.toggle();
        }
    }
}

impl Default for Kr580Vt57 {
    fn default() -> Self {
        Self::new()
    }
}
