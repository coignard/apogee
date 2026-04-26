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

pub struct RomDisk {
    data: Vec<u8>,
    cur_addr: usize,
    old_a15: bool,
    mask: usize,
}

impl RomDisk {
    pub fn new() -> Self {
        Self {
            data: Vec::new(),
            cur_addr: 0,
            old_a15: false,
            mask: 0,
        }
    }

    pub fn load(&mut self, payload: &[u8]) {
        self.data = payload.to_vec();

        let size_kb = self.data.len() / 1024;
        self.mask = match size_kb {
            0..=512 => 0x0F,
            513..=1024 => 0x1F,
            1025..=2048 => 0x3F,
            2049..=4096 => 0x7F,
            _ => 0xFF,
        };

        self.cur_addr = 0;
        self.old_a15 = false;
    }

    pub fn read_data(&self) -> u8 {
        if self.cur_addr < self.data.len() {
            self.data[self.cur_addr]
        } else {
            0xFF
        }
    }

    pub fn update_addr(&mut self, port_b: u8, port_c: u8) {
        let new_a15 = (port_c & 0x80) != 0;
        let c_val = (port_c & 0x7F) as usize;

        let mut addr = self.cur_addr;
        addr = (addr & !0xFF) | (port_b as usize);
        addr = (addr & !0x7F00) | (c_val << 8);

        if new_a15 && !self.old_a15 {
            addr = (addr & 0x7FFF) | ((addr & self.mask) << 15);
        }

        self.old_a15 = new_a15;
        self.cur_addr = addr;
    }
}

impl Default for RomDisk {
    fn default() -> Self {
        Self::new()
    }
}
