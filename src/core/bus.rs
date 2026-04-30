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
use super::peripherals::keyboard::Keyboard;
use super::peripherals::romdisk::RomDisk;

pub mod memory_map {
    pub const RAM_START: u16 = 0x0000;
    pub const RAM_END: u16 = 0xEBFF;
    pub const PIT_BASE: u16 = 0xEC00;
    pub const PIT_END: u16 = 0xECFF;
    pub const PPI_SYS_BASE: u16 = 0xED00;
    pub const PPI_SYS_END: u16 = 0xEDFF;
    pub const PPI_USR_BASE: u16 = 0xEE00;
    pub const PPI_USR_END: u16 = 0xEEFF;
    pub const CRTC_BASE: u16 = 0xEF00;
    pub const CRTC_END: u16 = 0xEFFF;
    pub const DMA_ROM_BASE: u16 = 0xF000;
    pub const DMA_ROM_END: u16 = 0xFFFF;
}

pub struct Bus {
    pub(crate) ram: Box<[u8; 0x10000]>,
    pub(crate) system_rom: Vec<u8>,

    pub(crate) vi53: Kr580Vi53,
    pub(crate) vt57: Kr580Vt57,
    pub(crate) vg75: Kr580Vg75,

    pub(crate) sys_vv55: Kr580Vv55a,
    pub(crate) user_vv55: Kr580Vv55a,

    pub(crate) keyboard: Keyboard,
    pub(crate) romdisk: RomDisk,
}

impl Bus {
    pub fn new(system_rom: Vec<u8>) -> Self {
        Self {
            ram: Box::new([0; 0x10000]),
            system_rom,
            vi53: Kr580Vi53::new(),
            vt57: Kr580Vt57::new(),
            vg75: Kr580Vg75::new(),
            sys_vv55: Kr580Vv55a::new(),
            user_vv55: Kr580Vv55a::new(),
            keyboard: Keyboard::new(),
            romdisk: RomDisk::new(),
        }
    }
}

impl Machine for Bus {
    fn peek(&mut self, addr: u16) -> u8 {
        match addr {
            memory_map::RAM_START..=memory_map::RAM_END => self.ram[addr as usize],
            memory_map::PIT_BASE..=memory_map::PIT_END => self.vi53.read(addr),
            memory_map::PPI_SYS_BASE..=memory_map::PPI_SYS_END => {
                let kbd_mask = self.sys_vv55.port_a_out;
                let (kbd_res, port_c_in) = self.keyboard.read_matrix(kbd_mask);
                self.sys_vv55.read(addr, 0xFF, kbd_res, port_c_in)
            }
            memory_map::PPI_USR_BASE..=memory_map::PPI_USR_END => {
                let port_a_in = self.romdisk.read_data();
                self.user_vv55.read(addr, port_a_in, 0xFF, 0xFF)
            }
            memory_map::CRTC_BASE..=memory_map::CRTC_END => self.vg75.read(addr),
            memory_map::DMA_ROM_BASE..=memory_map::DMA_ROM_END => {
                let idx = (addr - memory_map::DMA_ROM_BASE) as usize;
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
            memory_map::RAM_START..=memory_map::RAM_END => self.ram[addr as usize] = val,
            memory_map::PIT_BASE..=memory_map::PIT_END => self.vi53.write(addr, val),
            memory_map::PPI_SYS_BASE..=memory_map::PPI_SYS_END => self.sys_vv55.write(addr, val),
            memory_map::PPI_USR_BASE..=memory_map::PPI_USR_END => {
                self.user_vv55.write(addr, val);
                let port_b = self.user_vv55.port_b_out;
                let port_c = self.user_vv55.port_c_out;
                self.romdisk.update_addr(port_b, port_c);
            }
            memory_map::CRTC_BASE..=memory_map::CRTC_END => self.vg75.write(addr, val),
            memory_map::DMA_ROM_BASE..=memory_map::DMA_ROM_END => self.vt57.write(addr, val),
        }
    }

    fn port_in(&mut self, _port: u16) -> u8 {
        0xFF
    }

    fn port_out(&mut self, _port: u16, _val: u8) {}
}
