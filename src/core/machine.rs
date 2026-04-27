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

use super::bus::ApogeeBus;
use super::sound::AudioMixer;

pub const CPU_FREQ_HZ: u32 = 1_777_777;
pub const CPU_DIVIDER: u32 = 9;
pub const CHAR_CLOCK_DIVIDER: u32 = 12;

const RESET_VECTOR: u16 = 0xF800;
const TAPE_SYNC_BYTE: u8 = 0xE6;

const DEFAULT_SAMPLE_RATE: u32 = 44_100;
const AUTORUN_DELAY_CYCLES: u32 = 2_000_000;
const MAX_HALT_STEP_CYCLES: u32 = 100;
const RKA_HEADER_SIZE: usize = 4;
const RKA_TAIL_SIZE: usize = 3;

pub struct ApogeeMachine {
    cpu: Cpu,
    bus: ApogeeBus,
    audio_mixer: AudioMixer,
    cclk_acc: u32,
    pending_cycles: u32,
}

impl ApogeeMachine {
    pub fn new(system_rom: Vec<u8>) -> Self {
        let mut cpu = Cpu::new_8080();
        cpu.registers().set_pc(RESET_VECTOR);

        Self {
            cpu,
            bus: ApogeeBus::new(system_rom),
            audio_mixer: AudioMixer::new(DEFAULT_SAMPLE_RATE, CPU_FREQ_HZ),
            cclk_acc: 0,
            pending_cycles: 0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.audio_mixer.set_sample_rate(sample_rate);
    }

    pub fn load_rom(&mut self, payload: &[u8], is_rka: bool, autorun: bool, force: bool) -> Result<()> {
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
                    let halt_cycles = self.bus.vt57.halt_cycles();
                    let is_halted = halt_cycles > 0;

                    let step = if is_halted {
                        halt_cycles.min(MAX_HALT_STEP_CYCLES)
                    } else {
                        self.step_cpu()
                    };

                    self.advance_system(step, &mut |_| {}, &mut |_| {});

                    if is_halted {
                        self.bus.vt57.sub_halt_cycles(step);
                    }
                    cycles_done += step;
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

    pub fn run<S, R>(&mut self, elapsed_secs: f32, mut push_sample: S, mut render_frame: R) -> bool
    where
        S: FnMut(f32),
        R: FnMut(&crate::core::chips::kr580vg75::Kr580Vg75),
    {
        let mut render_requested = false;

        let new_cycles = (elapsed_secs * CPU_FREQ_HZ as f32) as u32;
        self.pending_cycles = self.pending_cycles.saturating_add(new_cycles);

        while self.pending_cycles > 0 {
            let halt_cycles = self.bus.vt57.halt_cycles();
            let is_halted = halt_cycles > 0;

            let step = if is_halted {
                halt_cycles.min(self.pending_cycles)
            } else {
                self.step_cpu()
            };

            if self.advance_system(step, &mut push_sample, &mut render_frame) {
                render_requested = true;
            }

            if is_halted {
                self.bus.vt57.sub_halt_cycles(step);
            }
            self.pending_cycles = self.pending_cycles.saturating_sub(step);
        }

        render_requested
    }

    fn step_cpu(&mut self) -> u32 {
        let cycles_before = self.cpu.cycle_count();
        self.cpu.execute_instruction(&mut self.bus);
        let cycles_after = self.cpu.cycle_count();

        let (inte, _) = self.cpu.immutable_registers().get_interrupt_mode();
        self.bus.vg75.set_inte(inte);

        (cycles_after - cycles_before) as u32
    }

    fn advance_system<S, R>(
        &mut self,
        cpu_cycles: u32,
        push_sample: &mut S,
        render_frame: &mut R,
    ) -> bool
    where
        S: FnMut(f32),
        R: FnMut(&crate::core::chips::kr580vg75::Kr580Vg75),
    {
        let mut render_requested = false;

        for _ in 0..cpu_cycles {
            let vi53_mixed = self.bus.vi53.tick();
            let beeper = self.bus.sys_vv55.is_beeper_active();

            if let Some(sample) = self.audio_mixer.tick(vi53_mixed, beeper) {
                push_sample(sample);
            }

            self.cclk_acc += CPU_DIVIDER;
            while self.cclk_acc >= CHAR_CLOCK_DIVIDER {
                self.cclk_acc -= CHAR_CLOCK_DIVIDER;

                if self.bus.vg75.tick(&mut self.bus.vt57, &self.bus.ram) {
                    render_frame(&self.bus.vg75);
                    render_requested = true;
                }
            }
        }

        render_requested
    }
}
