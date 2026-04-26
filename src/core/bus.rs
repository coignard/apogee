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

use iz80::Machine;

use super::chips::kr580vg75::Kr580Vg75;
use super::chips::kr580vi53::Kr580Vi53;
use super::chips::kr580vt57::Kr580Vt57;
use super::chips::kr580vv55a::Kr580Vv55a;
use super::peripherals::keyboard::ApogeeKeyboard;
use super::peripherals::romdisk::RomDisk;

pub struct ApogeeBus {
    pub(crate) ram: Box<[u8; 0x10000]>,
    pub(crate) system_rom: Vec<u8>,

    pub(crate) vi53: Kr580Vi53,
    pub(crate) vt57: Kr580Vt57,
    pub(crate) vg75: Kr580Vg75,

    pub(crate) sys_vv55: Kr580Vv55a,
    pub(crate) user_vv55: Kr580Vv55a,

    pub(crate) keyboard: ApogeeKeyboard,
    pub(crate) romdisk: RomDisk,
}

impl ApogeeBus {
    pub fn new(system_rom: Vec<u8>) -> Self {
        Self {
            ram: Box::new([0; 0x10000]),
            system_rom,
            vi53: Kr580Vi53::new(),
            vt57: Kr580Vt57::new(),
            vg75: Kr580Vg75::new(),
            sys_vv55: Kr580Vv55a::new(),
            user_vv55: Kr580Vv55a::new(),
            keyboard: ApogeeKeyboard::new(),
            romdisk: RomDisk::new(),
        }
    }
}

impl Machine for ApogeeBus {
    fn peek(&mut self, addr: u16) -> u8 {
        match addr {
            0x0000..=0xEBFF => self.ram[addr as usize],
            0xEC00..=0xECFF => self.vi53.read(addr),
            0xED00..=0xEDFF => {
                let kbd_mask = self.sys_vv55.port_a_out;
                let (kbd_res, port_c_in) = self.keyboard.read_matrix(kbd_mask);
                self.sys_vv55.read(addr, 0xFF, kbd_res, port_c_in)
            }
            0xEE00..=0xEEFF => {
                let port_a_in = self.romdisk.read_data();
                self.user_vv55.read(addr, port_a_in, 0xFF, 0xFF)
            }
            0xEF00..=0xEFFF => self.vg75.read(addr),
            0xF000..=0xFFFF => {
                let idx = (addr - 0xF000) as usize;
                if !self.system_rom.is_empty() {
                    self.system_rom[idx % self.system_rom.len()]
                } else {
                    0xFF
                }
            }
        }
    }

    fn poke(&mut self, addr: u16, val: u8) {
        match addr {
            0x0000..=0xEBFF => self.ram[addr as usize] = val,
            0xEC00..=0xECFF => self.vi53.write(addr, val),
            0xED00..=0xEDFF => self.sys_vv55.write(addr, val),
            0xEE00..=0xEEFF => {
                self.user_vv55.write(addr, val);
                let port_b = self.user_vv55.port_b_out;
                let port_c = self.user_vv55.port_c_out;
                self.romdisk.update_addr(port_b, port_c);
            }
            0xEF00..=0xEFFF => self.vg75.write(addr, val),
            0xF000..=0xFFFF => self.vt57.write(addr, val),
        }
    }

    fn port_in(&mut self, _port: u16) -> u8 {
        0xFF
    }

    fn port_out(&mut self, _port: u16, _val: u8) {}
}
