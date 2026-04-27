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

const PORT_MASK: u16 = 3;
const PORT_A: u16 = 0;
const PORT_B: u16 = 1;
const PORT_C: u16 = 2;
const PORT_CWR: u16 = 3;

const CWR_DEFAULT: u8 = 0x9B;
const CWR_MODE_SET_FLAG: u8 = 0x80;
const CWR_PORT_A_IN: u8 = 0x10;
const CWR_PORT_B_IN: u8 = 0x02;
const CWR_PORT_C_HI_IN: u8 = 0x08;
const CWR_PORT_C_LO_IN: u8 = 0x01;

const BSR_BIT_MASK: u8 = 0x07;
const BSR_VAL_MASK: u8 = 0x01;

const BEEPER_BIT_MASK: u8 = 0x01;

#[derive(Clone, Copy)]
pub struct Kr580Vv55a {
    control: u8,
    pub port_a_out: u8,
    pub port_b_out: u8,
    pub port_c_out: u8,
}

impl Default for Kr580Vv55a {
    fn default() -> Self {
        Self::new()
    }
}

impl Kr580Vv55a {
    pub fn new() -> Self {
        Self {
            control: CWR_DEFAULT,
            port_a_out: 0,
            port_b_out: 0,
            port_c_out: 0,
        }
    }

    #[inline]
    pub fn is_beeper_active(&self) -> bool {
        (self.port_c_out & BEEPER_BIT_MASK) != 0
    }

    pub fn read(&self, port: u16, in_a: u8, in_b: u8, in_c: u8) -> u8 {
        match port & PORT_MASK {
            PORT_A => {
                if self.control & CWR_PORT_A_IN != 0 {
                    in_a
                } else {
                    self.port_a_out
                }
            }
            PORT_B => {
                if self.control & CWR_PORT_B_IN != 0 {
                    in_b
                } else {
                    self.port_b_out
                }
            }
            PORT_C => {
                let mut res = 0;
                res |= if self.control & CWR_PORT_C_LO_IN != 0 {
                    in_c & 0x0F
                } else {
                    self.port_c_out & 0x0F
                };
                res |= if self.control & CWR_PORT_C_HI_IN != 0 {
                    in_c & 0xF0
                } else {
                    self.port_c_out & 0xF0
                };
                res
            }
            PORT_CWR => self.control,
            _ => 0xFF,
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        match port & PORT_MASK {
            PORT_A => self.port_a_out = val,
            PORT_B => self.port_b_out = val,
            PORT_C => self.port_c_out = val,
            PORT_CWR => {
                if val & CWR_MODE_SET_FLAG != 0 {
                    self.control = val;
                    self.port_a_out = 0;
                    self.port_b_out = 0;
                    self.port_c_out = 0;
                } else {
                    let bit = (val >> 1) & BSR_BIT_MASK;
                    if val & BSR_VAL_MASK != 0 {
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
