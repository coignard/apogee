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

#[derive(Clone, Copy, PartialEq, Default)]
pub enum BytePhase {
    #[default]
    Lsb,
    Msb,
}

impl BytePhase {
    pub fn toggle(&mut self) {
        *self = match self {
            Self::Lsb => Self::Msb,
            Self::Msb => Self::Lsb,
        };
    }
}

const PORT_MASK: u16 = 0x0F;
const PORT_CH2_ADDR: u16 = 4;
const PORT_CH2_COUNT: u16 = 5;
const PORT_CH3_ADDR: u16 = 6;
const PORT_CH3_COUNT: u16 = 7;
const PORT_MODE: u16 = 8;

const MODE_ENABLE_CH2: u8 = 0x04;
const MODE_AUTO_LOAD: u8 = 0x80;
const COUNT_MASK: u16 = 0x3FFF;
const COUNT_MODE_PRESERVE_MASK: u16 = 0xC000;

pub struct Kr580Vt57 {
    enabled: bool,
    mode: u8,
    byte_phase: BytePhase,
    ch2_addr: u16,
    ch2_count: u16,
    ch3_addr: u16,
    ch3_count: u16,
    halt_cycles: u32,
}

impl Kr580Vt57 {
    pub fn new() -> Self {
        Self {
            enabled: false,
            mode: 0,
            byte_phase: BytePhase::default(),
            ch2_addr: 0,
            ch2_count: 0,
            ch3_addr: 0,
            ch3_count: 0,
            halt_cycles: 0,
        }
    }

    #[inline]
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    #[inline]
    pub fn ch2_addr(&self) -> u16 {
        self.ch2_addr
    }

    #[inline]
    pub fn step_ch2(&mut self) {
        self.ch2_addr = self.ch2_addr.wrapping_add(1);

        let count = self.ch2_count & COUNT_MASK;
        if count == 0 {
            if (self.mode & MODE_AUTO_LOAD) != 0 {
                self.ch2_addr = self.ch3_addr;
                self.ch2_count = self.ch3_count;
            } else {
                self.ch2_count = (self.ch2_count & COUNT_MODE_PRESERVE_MASK) | COUNT_MASK;
            }
        } else {
            self.ch2_count = (self.ch2_count & COUNT_MODE_PRESERVE_MASK) | (count - 1);
        }
    }

    #[inline]
    pub fn add_halt_cycles(&mut self, amount: u32) {
        self.halt_cycles += amount;
    }

    #[inline]
    pub fn sub_halt_cycles(&mut self, amount: u32) {
        self.halt_cycles -= amount;
    }

    #[inline]
    pub fn halt_cycles(&self) -> u32 {
        self.halt_cycles
    }

    pub fn write(&mut self, port: u16, val: u8) {
        match port & PORT_MASK {
            PORT_CH2_ADDR => {
                if self.byte_phase == BytePhase::Msb {
                    self.ch2_addr = (self.ch2_addr & 0x00FF) | ((val as u16) << 8);
                    self.ch3_addr = (self.ch3_addr & 0x00FF) | ((val as u16) << 8);
                } else {
                    self.ch2_addr = (self.ch2_addr & 0xFF00) | (val as u16);
                    self.ch3_addr = (self.ch3_addr & 0xFF00) | (val as u16);
                }
                self.byte_phase.toggle();
            }
            PORT_CH2_COUNT => {
                if self.byte_phase == BytePhase::Msb {
                    self.ch2_count = (self.ch2_count & 0x00FF) | ((val as u16) << 8);
                    self.ch3_count = (self.ch3_count & 0x00FF) | ((val as u16) << 8);
                } else {
                    self.ch2_count = (self.ch2_count & 0xFF00) | (val as u16);
                    self.ch3_count = (self.ch3_count & 0xFF00) | (val as u16);
                }
                self.byte_phase.toggle();
            }
            PORT_CH3_ADDR => {
                if self.byte_phase == BytePhase::Msb {
                    self.ch3_addr = (self.ch3_addr & 0x00FF) | ((val as u16) << 8);
                } else {
                    self.ch3_addr = (self.ch3_addr & 0xFF00) | (val as u16);
                }
                self.byte_phase.toggle();
            }
            PORT_CH3_COUNT => {
                if self.byte_phase == BytePhase::Msb {
                    self.ch3_count = (self.ch3_count & 0x00FF) | ((val as u16) << 8);
                } else {
                    self.ch3_count = (self.ch3_count & 0xFF00) | (val as u16);
                }
                self.byte_phase.toggle();
            }
            PORT_MODE => {
                self.byte_phase = BytePhase::Lsb;
                self.mode = val;
                self.enabled = (val & MODE_ENABLE_CH2) != 0;
            }
            port_idx => {
                if port_idx < PORT_MODE {
                    self.byte_phase.toggle();
                }
            }
        }
    }
}

impl Default for Kr580Vt57 {
    fn default() -> Self {
        Self::new()
    }
}
