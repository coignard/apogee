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

use anyhow::{Result, bail};
use iz80::Cpu;

use super::bus::Bus;
use super::chips::kr580vg75::Kr580Vg75;
use super::sound::AudioMixer;

pub const MASTER_CLOCK_HZ: u32 = 16_000_000;
pub const CPU_DIVIDER: u32 = 9;
pub const CHAR_CLOCK_DIVIDER: u32 = 12;

const RESET_VECTOR: u16 = 0xF800;
const TAPE_SYNC_BYTE: u8 = 0xE6;

const AUTORUN_DELAY_CYCLES: u32 = 2_000_000;
const RKA_HEADER_SIZE: usize = 4;
const RKA_TAIL_SIZE: usize = 3;

pub const DEFAULT_CHARS_PER_ROW: u32 = 78;
pub const DEFAULT_HR_CHARS: u32 = 18;
pub const DEFAULT_DISPLAY_ROWS: u32 = 30;
pub const DEFAULT_VR_ROWS: u32 = 4;
pub const DEFAULT_LINES_PER_ROW: u32 = 10;

pub const DEFAULT_FRAME_CYCLES: u32 = ((DEFAULT_CHARS_PER_ROW + DEFAULT_HR_CHARS)
    * (DEFAULT_DISPLAY_ROWS + DEFAULT_VR_ROWS)
    * DEFAULT_LINES_PER_ROW
    * CHAR_CLOCK_DIVIDER)
    / CPU_DIVIDER;

pub const MAX_CHARS_PER_ROW: u32 = 128;
pub const MAX_HR_CHARS: u32 = 32;
pub const MAX_DISPLAY_ROWS: u32 = 64;
pub const MAX_VR_ROWS: u32 = 4;
pub const MAX_LINES_PER_ROW: u32 = 16;

pub const MAX_FRAME_CYCLES: u32 = ((MAX_CHARS_PER_ROW + MAX_HR_CHARS)
    * (MAX_DISPLAY_ROWS + MAX_VR_ROWS)
    * MAX_LINES_PER_ROW
    * CHAR_CLOCK_DIVIDER)
    / CPU_DIVIDER
    + 1;

pub struct Machine {
    cpu: Cpu,
    bus: Bus,
    audio_mixer: AudioMixer,
    cclk_acc: u32,
}

impl Machine {
    pub fn new(system_rom: Vec<u8>, sample_rate: u32) -> Self {
        let mut cpu = Cpu::new_8080();
        cpu.registers().set_pc(RESET_VECTOR);

        Self {
            cpu,
            bus: Bus::new(system_rom),
            audio_mixer: AudioMixer::new(sample_rate, MASTER_CLOCK_HZ, CPU_DIVIDER),
            cclk_acc: 0,
        }
    }

    #[inline]
    pub fn vg75(&self) -> &Kr580Vg75 {
        &self.bus.vg75
    }

    pub fn load_rom(
        &mut self,
        payload: &[u8],
        is_rka: bool,
        autorun: bool,
        force: bool,
    ) -> Result<()> {
        if is_rka {
            let offset = if payload.first() == Some(&TAPE_SYNC_BYTE) {
                1
            } else {
                0
            };

            if payload.len() < offset + RKA_HEADER_SIZE {
                bail!("file is too short to be a valid RKA");
            }

            let start_addr = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
            let end_addr = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]);

            if start_addr > end_addr && !force {
                bail!("start address is greater than end address");
            }

            let data_start = offset + RKA_HEADER_SIZE;

            let intended_len = if start_addr <= end_addr {
                (end_addr as usize - start_addr as usize) + 1
            } else {
                payload.len() - data_start
            };

            let len = if payload.len() < data_start + intended_len {
                if force {
                    payload.len() - data_start
                } else {
                    bail!("file is shorter than the expected data length");
                }
            } else {
                intended_len
            };

            let data = &payload[data_start..data_start + len];

            if !force {
                let (mut cs_hi, mut cs_lo) = (0u8, 0u8);
                for &b in data {
                    let (new_lo, carry) = cs_lo.overflowing_add(b);
                    cs_lo = new_lo;
                    cs_hi = cs_hi.wrapping_add(b).wrapping_add(u8::from(carry));
                }

                let tail = &payload[data_start + len..];

                let Some(sync_idx) = tail.iter().position(|&b| b == TAPE_SYNC_BYTE) else {
                    bail!("checksum block missing");
                };

                if tail.len() < sync_idx + RKA_TAIL_SIZE {
                    bail!("missing checksum bytes after sync");
                }

                if cs_hi != tail[sync_idx + 1] || cs_lo != tail[sync_idx + 2] {
                    bail!("checksum mismatch");
                }
            }

            if autorun {
                let mut cycles_done = 0;
                while cycles_done < AUTORUN_DELAY_CYCLES {
                    self.tick(|_| {});
                    cycles_done += DEFAULT_FRAME_CYCLES;
                }
            }

            for (i, &b) in data.iter().enumerate() {
                let write_addr = start_addr.wrapping_add(i as u16) as usize;
                if write_addr < self.bus.ram.len() {
                    self.bus.ram[write_addr] = b;
                }
            }

            if autorun {
                self.cpu.registers().set_pc(start_addr);
            }
        } else {
            self.bus.romdisk.load(payload);
        }

        Ok(())
    }

    pub fn update_key(&mut self, row: usize, col: usize, pressed: bool) {
        self.bus.keyboard.update_key(row, col, pressed);
    }

    pub fn tick<S>(&mut self, mut push_sample: S) -> bool
    where
        S: FnMut(f32),
    {
        let mut vblank_occurred = false;
        let mut frame_cycles = 0;

        let target_cycles = if self.bus.vg75.is_raster_running() {
            MAX_FRAME_CYCLES
        } else {
            DEFAULT_FRAME_CYCLES
        };

        while !vblank_occurred && frame_cycles < target_cycles {
            let halt_cycles = self.bus.vt57.halt_cycles();

            let elapsed_cycles = if halt_cycles > 0 {
                self.bus.vt57.sub_halt_cycles(halt_cycles);
                halt_cycles
            } else {
                self.execute_cpu_instruction()
            };

            frame_cycles += elapsed_cycles;

            for _ in 0..elapsed_cycles {
                self.bus.vg75.tick(&mut self.bus.vt57, &self.bus.ram);

                let vi53_mixed = self.bus.vi53.tick();
                let tape_out = self.bus.sys_vv55.is_tape_out_active();

                if let Some(sample) = self.audio_mixer.tick(vi53_mixed, tape_out) {
                    push_sample(sample);
                }

                self.cclk_acc += CPU_DIVIDER;
                while self.cclk_acc >= CHAR_CLOCK_DIVIDER {
                    self.cclk_acc -= CHAR_CLOCK_DIVIDER;

                    if self.bus.vg75.tick_char() {
                        vblank_occurred = true;
                    }
                }
            }
        }

        vblank_occurred
    }

    #[inline]
    fn execute_cpu_instruction(&mut self) -> u32 {
        let cycles_before = self.cpu.cycle_count();
        self.cpu.execute_instruction(&mut self.bus);
        let cycles_after = self.cpu.cycle_count();

        let (inte, _) = self.cpu.immutable_registers().get_interrupt_mode();
        self.bus.vg75.set_inte(inte);

        (cycles_after - cycles_before) as u32
    }
}
