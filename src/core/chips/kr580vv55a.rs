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
pub struct Kr580Vv55a {
    control: u8,
    pub port_a_out: u8,
    pub port_b_out: u8,
    pub port_c_out: u8,
}

impl Kr580Vv55a {
    pub fn new() -> Self {
        Self {
            control: 0x9B,
            port_a_out: 0,
            port_b_out: 0,
            port_c_out: 0,
        }
    }

    pub fn read(&self, port: u16, in_a: u8, in_b: u8, in_c: u8) -> u8 {
        match port & 3 {
            0 => {
                if self.control & 0x10 != 0 {
                    in_a
                } else {
                    self.port_a_out
                }
            }
            1 => {
                if self.control & 0x02 != 0 {
                    in_b
                } else {
                    self.port_b_out
                }
            }
            2 => {
                let mut res = 0;
                res |= if self.control & 0x01 != 0 {
                    in_c & 0x0F
                } else {
                    self.port_c_out & 0x0F
                };
                res |= if self.control & 0x08 != 0 {
                    in_c & 0xF0
                } else {
                    self.port_c_out & 0xF0
                };
                res
            }
            3 => self.control,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        match port & 3 {
            0 => self.port_a_out = val,
            1 => self.port_b_out = val,
            2 => self.port_c_out = val,
            3 => {
                if val & 0x80 != 0 {
                    self.control = val;
                    self.port_a_out = 0;
                    self.port_b_out = 0;
                    self.port_c_out = 0;
                } else {
                    let bit = (val >> 1) & 7;
                    if val & 1 != 0 {
                        self.port_c_out |= 1 << bit;
                    } else {
                        self.port_c_out &= !(1 << bit);
                    }
                }
            }
            _ => {}
        }
    }
}
