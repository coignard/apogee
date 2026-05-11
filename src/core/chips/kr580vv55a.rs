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

const PORT_ADDRESS_MASK: u16 = 0x03;
const PORT_A: u16 = 0x00;
const PORT_B: u16 = 0x01;
const PORT_C: u16 = 0x02;
const PORT_CONTROL: u16 = 0x03;

const ALL_PINS_MASK: u8 = 0xFF;
const PORT_C_LOWER_NIBBLE_MASK: u8 = 0x0F;
const PORT_C_UPPER_NIBBLE_MASK: u8 = 0xF0;
const PORT_C_OUTPUT_MASK: u8 = 0x00;

const CTRL_MODE_SET_FLAG: u8 = 0x80;
const CTRL_GROUP_A_MODE_MASK: u8 = 0x60;
const CTRL_GROUP_A_MODE_SHIFT: u8 = 5;

const CTRL_GROUP_MODE_0: u8 = 0b00;
const CTRL_GROUP_MODE_1: u8 = 0b01;

const CTRL_GROUP_A_PORT_A_DIR: u8 = 0x10;
const CTRL_GROUP_A_PORT_C_UPPER_DIR: u8 = 0x08;
const CTRL_GROUP_B_MODE: u8 = 0x04;
const CTRL_GROUP_B_PORT_B_DIR: u8 = 0x02;
const CTRL_GROUP_B_PORT_C_LOWER_DIR: u8 = 0x01;

const CTRL_RESET_ALL_INPUT_MODE0: u8 = CTRL_MODE_SET_FLAG
    | CTRL_GROUP_A_PORT_A_DIR
    | CTRL_GROUP_A_PORT_C_UPPER_DIR
    | CTRL_GROUP_B_PORT_B_DIR
    | CTRL_GROUP_B_PORT_C_LOWER_DIR;

const BSR_BIT_INDEX_MASK: u8 = 0x0E;
const BSR_BIT_INDEX_SHIFT: u8 = 1;
const BSR_BIT_VALUE_MASK: u8 = 0x01;

const BIT_MASK_PC7: u8 = 1 << 7;
const BIT_MASK_PC6: u8 = 1 << 6;
const BIT_MASK_PC5: u8 = 1 << 5;
const BIT_MASK_PC4: u8 = 1 << 4;
const BIT_MASK_PC3: u8 = 1 << 3;
const BIT_MASK_PC2: u8 = 1 << 2;
const BIT_MASK_PC1: u8 = 1 << 1;
const BIT_MASK_PC0: u8 = 1 << 0;

const PIN_M1_IN_STB_A: u8 = BIT_MASK_PC4;
const PIN_M1_IN_IBF_A: u8 = BIT_MASK_PC5;
const PIN_M1_IN_INTR_A: u8 = BIT_MASK_PC3;
const PIN_M1_IN_INTE_A: u8 = BIT_MASK_PC4;

const PIN_M1_OUT_ACK_A: u8 = BIT_MASK_PC6;
const PIN_M1_OUT_OBF_A: u8 = BIT_MASK_PC7;
const PIN_M1_OUT_INTR_A: u8 = BIT_MASK_PC3;
const PIN_M1_OUT_INTE_A: u8 = BIT_MASK_PC6;

const PIN_M1_IN_STB_B: u8 = BIT_MASK_PC2;
const PIN_M1_IN_IBF_B: u8 = BIT_MASK_PC1;

const PIN_M1_OUT_ACK_B: u8 = BIT_MASK_PC2;
const PIN_M1_OUT_OBF_B: u8 = BIT_MASK_PC1;

const PIN_M1_INTE_B: u8 = BIT_MASK_PC2;
const PIN_M1_INTR_B: u8 = BIT_MASK_PC0;

const PIN_M2_ACK_A: u8 = BIT_MASK_PC6;
const PIN_M2_STB_A: u8 = BIT_MASK_PC4;
const PIN_M2_OBF_A: u8 = BIT_MASK_PC7;
const PIN_M2_IBF_A: u8 = BIT_MASK_PC5;
const PIN_M2_INTR_A: u8 = BIT_MASK_PC3;
const PIN_M2_INTE_1_A: u8 = BIT_MASK_PC6;
const PIN_M2_INTE_2_A: u8 = BIT_MASK_PC4;

const TAPE_OUT_BIT_MASK: u8 = BIT_MASK_PC0;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Mode0,
    Mode1,
    Mode2,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Direction {
    #[default]
    Input,
    Output,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct GroupConfig {
    pub mode: Mode,
    pub port_dir: Direction,
    pub port_c_dir: Direction,
}

#[derive(Clone, Copy, Debug, Default, Serialize, Deserialize)]
pub struct Config {
    pub group_a: GroupConfig,
    pub group_b: GroupConfig,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
pub struct Kr580Vv55a {
    config: Config,
    control_word: u8,

    output_latch_a: u8,
    output_latch_b: u8,
    output_latch_c: u8,

    input_latch_a: u8,
    input_latch_b: u8,

    pin_state_a: u8,
    pin_state_b: u8,
    pin_state_c: u8,

    ibf_a: bool,
    ibf_b: bool,
    obf_a_active: bool,
    obf_b_active: bool,
    intr_a: bool,
    intr_b: bool,

    inte_a_in: bool,
    inte_a_out: bool,
    inte_b: bool,
}

impl Default for Kr580Vv55a {
    fn default() -> Self {
        Self::new()
    }
}

impl Kr580Vv55a {
    pub fn new() -> Self {
        let mut chip = Self {
            config: Config::default(),
            control_word: 0,
            output_latch_a: 0,
            output_latch_b: 0,
            output_latch_c: 0,
            input_latch_a: 0,
            input_latch_b: 0,
            pin_state_a: ALL_PINS_MASK,
            pin_state_b: ALL_PINS_MASK,
            pin_state_c: ALL_PINS_MASK,
            ibf_a: false,
            ibf_b: false,
            obf_a_active: false,
            obf_b_active: false,
            intr_a: false,
            intr_b: false,
            inte_a_in: false,
            inte_a_out: false,
            inte_b: false,
        };
        chip.hardware_reset();
        chip
    }

    pub fn hardware_reset(&mut self) {
        self.apply_control_word(CTRL_RESET_ALL_INPUT_MODE0);
        self.pin_state_a = ALL_PINS_MASK;
        self.pin_state_b = ALL_PINS_MASK;
        self.pin_state_c = ALL_PINS_MASK;
    }

    #[inline]
    pub fn is_tape_out_active(&self) -> bool {
        (self.peripheral_read_c() & TAPE_OUT_BIT_MASK) != 0
    }

    fn apply_control_word(&mut self, val: u8) {
        self.control_word = val;

        self.config.group_a.mode = match (val & CTRL_GROUP_A_MODE_MASK) >> CTRL_GROUP_A_MODE_SHIFT {
            CTRL_GROUP_MODE_0 => Mode::Mode0,
            CTRL_GROUP_MODE_1 => Mode::Mode1,
            _ => Mode::Mode2,
        };

        self.config.group_a.port_dir = if val & CTRL_GROUP_A_PORT_A_DIR != 0 {
            Direction::Input
        } else {
            Direction::Output
        };

        self.config.group_a.port_c_dir = if val & CTRL_GROUP_A_PORT_C_UPPER_DIR != 0 {
            Direction::Input
        } else {
            Direction::Output
        };

        self.config.group_b.mode = if val & CTRL_GROUP_B_MODE != 0 {
            Mode::Mode1
        } else {
            Mode::Mode0
        };

        self.config.group_b.port_dir = if val & CTRL_GROUP_B_PORT_B_DIR != 0 {
            Direction::Input
        } else {
            Direction::Output
        };

        self.config.group_b.port_c_dir = if val & CTRL_GROUP_B_PORT_C_LOWER_DIR != 0 {
            Direction::Input
        } else {
            Direction::Output
        };

        self.output_latch_a = 0;
        self.output_latch_b = 0;
        self.output_latch_c = 0;

        self.ibf_a = false;
        self.ibf_b = false;
        self.obf_a_active = false;
        self.obf_b_active = false;
        self.intr_a = false;
        self.intr_b = false;

        self.inte_a_in = false;
        self.inte_a_out = false;
        self.inte_b = false;

        self.update_interrupts();
    }

    #[inline]
    fn port_c_base_value(&self) -> u8 {
        let lower_mask = if self.config.group_b.port_c_dir == Direction::Input {
            PORT_C_LOWER_NIBBLE_MASK
        } else {
            PORT_C_OUTPUT_MASK
        };
        let upper_mask = if self.config.group_a.port_c_dir == Direction::Input {
            PORT_C_UPPER_NIBBLE_MASK
        } else {
            PORT_C_OUTPUT_MASK
        };
        let input_mask = lower_mask | upper_mask;

        (self.pin_state_c & input_mask) | (self.output_latch_c & !input_mask)
    }

    pub fn cpu_read(&mut self, port: u16) -> u8 {
        match port & PORT_ADDRESS_MASK {
            PORT_A => match self.config.group_a.mode {
                Mode::Mode0 => {
                    if self.config.group_a.port_dir == Direction::Output {
                        self.output_latch_a
                    } else {
                        self.pin_state_a
                    }
                }
                Mode::Mode1 => {
                    if self.config.group_a.port_dir == Direction::Output {
                        self.output_latch_a
                    } else {
                        self.ibf_a = false;
                        self.update_interrupts();
                        self.input_latch_a
                    }
                }
                Mode::Mode2 => {
                    self.ibf_a = false;
                    self.update_interrupts();
                    self.input_latch_a
                }
            },
            PORT_B => match self.config.group_b.mode {
                Mode::Mode0 => {
                    if self.config.group_b.port_dir == Direction::Output {
                        self.output_latch_b
                    } else {
                        self.pin_state_b
                    }
                }
                Mode::Mode1 => {
                    if self.config.group_b.port_dir == Direction::Output {
                        self.output_latch_b
                    } else {
                        self.ibf_b = false;
                        self.update_interrupts();
                        self.input_latch_b
                    }
                }
                Mode::Mode2 => ALL_PINS_MASK,
            },
            PORT_C => self.cpu_read_port_c_status_word(),
            _ => ALL_PINS_MASK,
        }
    }

    pub fn cpu_write(&mut self, port: u16, val: u8) {
        match port & PORT_ADDRESS_MASK {
            PORT_A => {
                self.output_latch_a = val;
                let is_strobed_output = self.config.group_a.mode == Mode::Mode1
                    && self.config.group_a.port_dir == Direction::Output;

                if is_strobed_output || self.config.group_a.mode == Mode::Mode2 {
                    self.obf_a_active = true;
                    self.update_interrupts();
                }
            }
            PORT_B => {
                self.output_latch_b = val;
                if self.config.group_b.mode == Mode::Mode1
                    && self.config.group_b.port_dir == Direction::Output
                {
                    self.obf_b_active = true;
                    self.update_interrupts();
                }
            }
            PORT_C => {
                self.output_latch_c = val;
            }
            PORT_CONTROL => {
                if (val & CTRL_MODE_SET_FLAG) != 0 {
                    self.apply_control_word(val);
                } else {
                    let bit_idx = (val & BSR_BIT_INDEX_MASK) >> BSR_BIT_INDEX_SHIFT;
                    let bit_val = (val & BSR_BIT_VALUE_MASK) != 0;
                    let bit_mask = 1 << bit_idx;

                    if bit_val {
                        self.output_latch_c |= bit_mask;
                    } else {
                        self.output_latch_c &= !bit_mask;
                    }

                    match bit_mask {
                        BIT_MASK_PC6 => self.inte_a_out = bit_val,
                        BIT_MASK_PC4 => self.inte_a_in = bit_val,
                        BIT_MASK_PC2 => self.inte_b = bit_val,
                        _ => {}
                    }
                    self.update_interrupts();
                }
            }
            _ => {}
        }
    }

    fn cpu_read_port_c_status_word(&self) -> u8 {
        let mut status = self.port_c_base_value();

        match self.config.group_a.mode {
            Mode::Mode1 => {
                if self.config.group_a.port_dir == Direction::Input {
                    status &= !(PIN_M1_IN_IBF_A | PIN_M1_IN_INTE_A | PIN_M1_IN_INTR_A);
                    if self.ibf_a {
                        status |= PIN_M1_IN_IBF_A;
                    }
                    if self.inte_a_in {
                        status |= PIN_M1_IN_INTE_A;
                    }
                    if self.intr_a {
                        status |= PIN_M1_IN_INTR_A;
                    }
                } else {
                    status &= !(PIN_M1_OUT_OBF_A | PIN_M1_OUT_INTE_A | PIN_M1_OUT_INTR_A);
                    if !self.obf_a_active {
                        status |= PIN_M1_OUT_OBF_A;
                    }
                    if self.inte_a_out {
                        status |= PIN_M1_OUT_INTE_A;
                    }
                    if self.intr_a {
                        status |= PIN_M1_OUT_INTR_A;
                    }
                }
            }
            Mode::Mode2 => {
                status &= !(PIN_M2_OBF_A
                    | PIN_M2_INTE_1_A
                    | PIN_M2_IBF_A
                    | PIN_M2_INTE_2_A
                    | PIN_M2_INTR_A);
                if !self.obf_a_active {
                    status |= PIN_M2_OBF_A;
                }
                if self.inte_a_out {
                    status |= PIN_M2_INTE_1_A;
                }
                if self.ibf_a {
                    status |= PIN_M2_IBF_A;
                }
                if self.inte_a_in {
                    status |= PIN_M2_INTE_2_A;
                }
                if self.intr_a {
                    status |= PIN_M2_INTR_A;
                }
            }
            Mode::Mode0 => {}
        }

        if self.config.group_b.mode == Mode::Mode1 {
            if self.config.group_b.port_dir == Direction::Input {
                status &= !(PIN_M1_INTE_B | PIN_M1_IN_IBF_B | PIN_M1_INTR_B);
                if self.inte_b {
                    status |= PIN_M1_INTE_B;
                }
                if self.ibf_b {
                    status |= PIN_M1_IN_IBF_B;
                }
                if self.intr_b {
                    status |= PIN_M1_INTR_B;
                }
            } else {
                status &= !(PIN_M1_INTE_B | PIN_M1_OUT_OBF_B | PIN_M1_INTR_B);
                if self.inte_b {
                    status |= PIN_M1_INTE_B;
                }
                if !self.obf_b_active {
                    status |= PIN_M1_OUT_OBF_B;
                }
                if self.intr_b {
                    status |= PIN_M1_INTR_B;
                }
            }
        }

        status
    }

    pub fn peripheral_read_c(&self) -> u8 {
        let mut pins = self.port_c_base_value();

        match self.config.group_a.mode {
            Mode::Mode1 => {
                if self.config.group_a.port_dir == Direction::Input {
                    pins &= !(PIN_M1_IN_IBF_A | PIN_M1_IN_STB_A | PIN_M1_IN_INTR_A);
                    if self.ibf_a {
                        pins |= PIN_M1_IN_IBF_A;
                    }
                    pins |= self.pin_state_c & PIN_M1_IN_STB_A;
                    if self.intr_a {
                        pins |= PIN_M1_IN_INTR_A;
                    }
                } else {
                    pins &= !(PIN_M1_OUT_OBF_A | PIN_M1_OUT_ACK_A | PIN_M1_OUT_INTR_A);
                    if !self.obf_a_active {
                        pins |= PIN_M1_OUT_OBF_A;
                    }
                    pins |= self.pin_state_c & PIN_M1_OUT_ACK_A;
                    if self.intr_a {
                        pins |= PIN_M1_OUT_INTR_A;
                    }
                }
            }
            Mode::Mode2 => {
                pins &=
                    !(PIN_M2_OBF_A | PIN_M2_ACK_A | PIN_M2_IBF_A | PIN_M2_STB_A | PIN_M2_INTR_A);
                if !self.obf_a_active {
                    pins |= PIN_M2_OBF_A;
                }
                pins |= self.pin_state_c & PIN_M2_ACK_A;
                if self.ibf_a {
                    pins |= PIN_M2_IBF_A;
                }
                pins |= self.pin_state_c & PIN_M2_STB_A;
                if self.intr_a {
                    pins |= PIN_M2_INTR_A;
                }
            }
            Mode::Mode0 => {}
        }

        if self.config.group_b.mode == Mode::Mode1 {
            if self.config.group_b.port_dir == Direction::Input {
                pins &= !(PIN_M1_IN_STB_B | PIN_M1_IN_IBF_B | PIN_M1_INTR_B);
                pins |= self.pin_state_c & PIN_M1_IN_STB_B;
                if self.ibf_b {
                    pins |= PIN_M1_IN_IBF_B;
                }
                if self.intr_b {
                    pins |= PIN_M1_INTR_B;
                }
            } else {
                pins &= !(PIN_M1_OUT_ACK_B | PIN_M1_OUT_OBF_B | PIN_M1_INTR_B);
                pins |= self.pin_state_c & PIN_M1_OUT_ACK_B;
                if !self.obf_b_active {
                    pins |= PIN_M1_OUT_OBF_B;
                }
                if self.intr_b {
                    pins |= PIN_M1_INTR_B;
                }
            }
        }

        pins
    }

    #[inline]
    pub fn peripheral_write_a(&mut self, val: u8) {
        self.pin_state_a = val;
    }

    #[inline]
    pub fn peripheral_write_b(&mut self, val: u8) {
        self.pin_state_b = val;
    }

    pub fn peripheral_write_c(&mut self, val: u8) {
        let prev = self.pin_state_c;
        self.pin_state_c = val;
        let falls = !val & prev;

        match self.config.group_a.mode {
            Mode::Mode1 => {
                if self.config.group_a.port_dir == Direction::Input {
                    if (falls & PIN_M1_IN_STB_A) != 0 {
                        self.input_latch_a = self.pin_state_a;
                        self.ibf_a = true;
                    }
                } else {
                    if (falls & PIN_M1_OUT_ACK_A) != 0 {
                        self.obf_a_active = false;
                    }
                }
            }
            Mode::Mode2 => {
                if (falls & PIN_M2_STB_A) != 0 {
                    self.input_latch_a = self.pin_state_a;
                    self.ibf_a = true;
                }
                if (falls & PIN_M2_ACK_A) != 0 {
                    self.obf_a_active = false;
                }
            }
            Mode::Mode0 => {}
        }

        if self.config.group_b.mode == Mode::Mode1 {
            if self.config.group_b.port_dir == Direction::Input {
                if (falls & PIN_M1_IN_STB_B) != 0 {
                    self.input_latch_b = self.pin_state_b;
                    self.ibf_b = true;
                }
            } else if (falls & PIN_M1_OUT_ACK_B) != 0 {
                self.obf_b_active = false;
            }
        }

        self.update_interrupts();
    }

    pub fn peripheral_read_a(&self) -> u8 {
        match self.config.group_a.mode {
            Mode::Mode2 => {
                if (self.pin_state_c & PIN_M2_ACK_A) == 0 {
                    self.output_latch_a
                } else {
                    ALL_PINS_MASK
                }
            }
            _ => {
                if self.config.group_a.port_dir == Direction::Output {
                    self.output_latch_a
                } else {
                    ALL_PINS_MASK
                }
            }
        }
    }

    pub fn peripheral_read_b(&self) -> u8 {
        if self.config.group_b.port_dir == Direction::Output {
            self.output_latch_b
        } else {
            ALL_PINS_MASK
        }
    }

    fn update_interrupts(&mut self) {
        match self.config.group_a.mode {
            Mode::Mode1 => {
                if self.config.group_a.port_dir == Direction::Input {
                    let stb_inactive = (self.pin_state_c & PIN_M1_IN_STB_A) != 0;
                    self.intr_a = self.inte_a_in && self.ibf_a && stb_inactive;
                } else {
                    let ack_inactive = (self.pin_state_c & PIN_M1_OUT_ACK_A) != 0;
                    self.intr_a = self.inte_a_out && !self.obf_a_active && ack_inactive;
                }
            }
            Mode::Mode2 => {
                let stb_inactive = (self.pin_state_c & PIN_M2_STB_A) != 0;
                let ack_inactive = (self.pin_state_c & PIN_M2_ACK_A) != 0;

                let intr_in = self.inte_a_in && self.ibf_a && stb_inactive;
                let intr_out = self.inte_a_out && !self.obf_a_active && ack_inactive;
                self.intr_a = intr_in || intr_out;
            }
            Mode::Mode0 => {
                self.intr_a = false;
            }
        }

        if self.config.group_b.mode == Mode::Mode1 {
            if self.config.group_b.port_dir == Direction::Input {
                let stb_inactive = (self.pin_state_c & PIN_M1_IN_STB_B) != 0;
                self.intr_b = self.inte_b && self.ibf_b && stb_inactive;
            } else {
                let ack_inactive = (self.pin_state_c & PIN_M1_OUT_ACK_B) != 0;
                self.intr_b = self.inte_b && !self.obf_b_active && ack_inactive;
            }
        } else {
            self.intr_b = false;
        }
    }
}
