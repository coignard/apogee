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

pub struct Kr580Vt57 {
    enabled: bool,
    mode: u8,
    dma_flip_flop: bool,
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
            dma_flip_flop: false,
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

        let count = self.ch2_count & 0x3FFF;
        if count == 0 {
            if (self.mode & 0x80) != 0 {
                self.ch2_addr = self.ch3_addr;
                self.ch2_count = self.ch3_count;
            } else {
                self.ch2_count = (self.ch2_count & 0xC000) | 0x3FFF;
            }
        } else {
            self.ch2_count = (self.ch2_count & 0xC000) | (count - 1);
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
        match port & 0x0F {
            4 => {
                if self.dma_flip_flop {
                    self.ch2_addr = (self.ch2_addr & 0x00FF) | ((val as u16) << 8);
                    self.ch3_addr = (self.ch3_addr & 0x00FF) | ((val as u16) << 8);
                } else {
                    self.ch2_addr = (self.ch2_addr & 0xFF00) | (val as u16);
                    self.ch3_addr = (self.ch3_addr & 0xFF00) | (val as u16);
                }
                self.dma_flip_flop = !self.dma_flip_flop;
            }
            5 => {
                if self.dma_flip_flop {
                    self.ch2_count = (self.ch2_count & 0x00FF) | ((val as u16) << 8);
                    self.ch3_count = (self.ch3_count & 0x00FF) | ((val as u16) << 8);
                } else {
                    self.ch2_count = (self.ch2_count & 0xFF00) | (val as u16);
                    self.ch3_count = (self.ch3_count & 0xFF00) | (val as u16);
                }
                self.dma_flip_flop = !self.dma_flip_flop;
            }
            6 => {
                if self.dma_flip_flop {
                    self.ch3_addr = (self.ch3_addr & 0x00FF) | ((val as u16) << 8);
                } else {
                    self.ch3_addr = (self.ch3_addr & 0xFF00) | (val as u16);
                }
                self.dma_flip_flop = !self.dma_flip_flop;
            }
            7 => {
                if self.dma_flip_flop {
                    self.ch3_count = (self.ch3_count & 0x00FF) | ((val as u16) << 8);
                } else {
                    self.ch3_count = (self.ch3_count & 0xFF00) | (val as u16);
                }
                self.dma_flip_flop = !self.dma_flip_flop;
            }
            8 => {
                self.dma_flip_flop = false;
                self.mode = val;
                self.enabled = (val & 0x04) != 0;
            }
            _ => {
                if (port & 0x0F) < 8 {
                    self.dma_flip_flop = !self.dma_flip_flop;
                }
            }
        }
    }
}
