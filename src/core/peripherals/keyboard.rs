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

const MATRIX_SIZE: usize = 9;
const MODIFIER_ROW: usize = 8;
const MODIFIER_PORT_C_MASK: u8 = 0xE0;
const PORT_C_FIXED_BITS: u8 = 0x0F;

#[derive(Clone, Copy)]
pub struct Keyboard {
    pub matrix: [u8; MATRIX_SIZE],
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            matrix: [0xFF; MATRIX_SIZE],
        }
    }

    pub fn update_key(&mut self, row: usize, col: usize, pressed: bool) {
        let mask = 1 << col;
        if pressed {
            self.matrix[row] &= !mask;
        } else {
            self.matrix[row] |= mask;
        }
    }

    pub fn read_matrix(&self, kbd_mask: u8) -> (u8, u8) {
        let mut kbd_res = 0xFF;
        for k in 0..8 {
            if (kbd_mask & (1 << k)) == 0 {
                kbd_res &= self.matrix[k];
            }
        }
        let port_c_in = (self.matrix[MODIFIER_ROW] & MODIFIER_PORT_C_MASK) | PORT_C_FIXED_BITS;
        (kbd_res, port_c_in)
    }
}

impl Default for Keyboard {
    fn default() -> Self {
        Self::new()
    }
}
