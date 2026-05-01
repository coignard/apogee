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

const MATRIX_SIZE: usize = 9;
const MODIFIER_ROW: usize = 8;
const MODIFIER_PORT_C_MASK: u8 = 0xE0;
const PORT_C_FIXED_BITS: u8 = 0x0F;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Key {
    Home,
    End,
    Escape,
    F1,
    F2,
    F3,
    F4,
    F5,
    Tab,
    PageDown,
    Enter,
    Backspace,
    Left,
    Up,
    Right,
    Down,
    Num0,
    Num1,
    Num2,
    Num3,
    Num4,
    Num5,
    Num6,
    Num7,
    Num8,
    Num9,
    Equal,
    Semicolon,
    Comma,
    Minus,
    Period,
    Slash,
    Backquote,
    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,
    BracketLeft,
    Backslash,
    BracketRight,
    Quote,
    Space,
    Shift,
    Ctrl,
    Alt,
}

impl Key {
    pub const fn coords(self) -> (usize, usize) {
        match self {
            Self::Home => (0, 0),
            Self::End => (0, 1),
            Self::Escape => (0, 2),
            Self::F1 => (0, 3),
            Self::F2 => (0, 4),
            Self::F3 => (0, 5),
            Self::F4 => (0, 6),
            Self::F5 => (0, 7),
            Self::Tab => (1, 0),
            Self::PageDown => (1, 1),
            Self::Enter => (1, 2),
            Self::Backspace => (1, 3),
            Self::Left => (1, 4),
            Self::Up => (1, 5),
            Self::Right => (1, 6),
            Self::Down => (1, 7),
            Self::Num0 => (2, 0),
            Self::Num1 => (2, 1),
            Self::Num2 => (2, 2),
            Self::Num3 => (2, 3),
            Self::Num4 => (2, 4),
            Self::Num5 => (2, 5),
            Self::Num6 => (2, 6),
            Self::Num7 => (2, 7),
            Self::Num8 => (3, 0),
            Self::Num9 => (3, 1),
            Self::Equal => (3, 2),
            Self::Semicolon => (3, 3),
            Self::Comma => (3, 4),
            Self::Minus => (3, 5),
            Self::Period => (3, 6),
            Self::Slash => (3, 7),
            Self::Backquote => (4, 0),
            Self::A => (4, 1),
            Self::B => (4, 2),
            Self::C => (4, 3),
            Self::D => (4, 4),
            Self::E => (4, 5),
            Self::F => (4, 6),
            Self::G => (4, 7),
            Self::H => (5, 0),
            Self::I => (5, 1),
            Self::J => (5, 2),
            Self::K => (5, 3),
            Self::L => (5, 4),
            Self::M => (5, 5),
            Self::N => (5, 6),
            Self::O => (5, 7),
            Self::P => (6, 0),
            Self::Q => (6, 1),
            Self::R => (6, 2),
            Self::S => (6, 3),
            Self::T => (6, 4),
            Self::U => (6, 5),
            Self::V => (6, 6),
            Self::W => (6, 7),
            Self::X => (7, 0),
            Self::Y => (7, 1),
            Self::Z => (7, 2),
            Self::BracketLeft => (7, 3),
            Self::Backslash => (7, 4),
            Self::BracketRight => (7, 5),
            Self::Quote => (7, 6),
            Self::Space => (7, 7),
            Self::Shift => (8, 5),
            Self::Ctrl => (8, 6),
            Self::Alt => (8, 7),
        }
    }
}

#[derive(Clone, Copy, Serialize)]
pub struct Keyboard {
    pub matrix: [u8; MATRIX_SIZE],
}

impl Keyboard {
    pub fn new() -> Self {
        Self {
            matrix: [0xFF; MATRIX_SIZE],
        }
    }

    pub fn update_key(&mut self, key: Key, pressed: bool) {
        let (row, col) = key.coords();
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
