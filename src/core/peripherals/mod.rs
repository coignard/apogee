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

pub mod keyboard;
pub mod midi;
pub mod romdisk;

use serde::Serialize;

use crate::core::peripherals::midi::MidiInterface;
use crate::core::peripherals::romdisk::RomDisk;

#[derive(Serialize, Default)]
pub enum UserPeripheral {
    RomDisk(RomDisk),
    Midi(MidiInterface),
    #[default]
    None,
}

impl UserPeripheral {
    #[inline]
    pub fn read_port_a(&self) -> u8 {
        match self {
            Self::RomDisk(disk) => disk.read_data(),
            _ => 0xFF,
        }
    }

    #[inline]
    pub fn update(&mut self, port_a: u8, port_b: u8, port_c: u8, cycle_count: u64) -> bool {
        match self {
            Self::RomDisk(disk) => {
                disk.update_addr(port_b, port_c);
                false
            }
            Self::Midi(midi) => midi.update(port_a, port_c, cycle_count),
            Self::None => false,
        }
    }
}
