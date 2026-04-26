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

use iz80::Cpu;

use super::bus::ApogeeBus;
use super::sound::AudioMixer;

pub const CPU_FREQ: u32 = 1_777_777;
pub const CPU_DIV: u32 = 9;
pub const CCLK_DIV: u32 = 12;

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
        cpu.registers().set_pc(0xF800);

        Self {
            cpu,
            bus: ApogeeBus::new(system_rom),
            audio_mixer: AudioMixer::new(44100, CPU_FREQ),
            cclk_acc: 0,
            pending_cycles: 0,
        }
    }

    pub fn set_sample_rate(&mut self, sample_rate: u32) {
        self.audio_mixer.set_sample_rate(sample_rate);
    }

    pub fn load_rom(&mut self, payload: &[u8], is_rka: bool) {
        if is_rka {
            let mut start_addr = 0x0000;
            let mut data = payload;

            let offset = if payload.first() == Some(&0xE6) { 1 } else { 0 };
            if payload.len() >= offset + 4 {
                start_addr = u16::from_be_bytes([payload[offset], payload[offset + 1]]);
                let end_addr = u16::from_be_bytes([payload[offset + 2], payload[offset + 3]]);
                let len = (end_addr.wrapping_sub(start_addr)).wrapping_add(1) as usize;

                let data_start = offset + 4;
                let data_end = (data_start + len).min(payload.len());
                data = &payload[data_start..data_end];
            }

            for (i, &b) in data.iter().enumerate() {
                let write_addr = start_addr.wrapping_add(i as u16) as usize;
                if write_addr < self.bus.ram.len() {
                    self.bus.ram[write_addr] = b;
                }
            }
        } else {
            self.bus.romdisk.load(payload);
        }
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

        let new_cycles = (elapsed_secs * CPU_FREQ as f32) as u32;
        self.pending_cycles = self.pending_cycles.saturating_add(new_cycles);

        while self.pending_cycles > 0 {
            let halt_cycles = self.bus.vt57.halt_cycles();
            if halt_cycles > 0 {
                let step = halt_cycles.min(self.pending_cycles);

                if self.advance_system(step, &mut push_sample, &mut render_frame) {
                    render_requested = true;
                }

                self.bus.vt57.sub_halt_cycles(step);
                self.pending_cycles -= step;
            } else {
                let step = self.step_cpu();

                if self.advance_system(step, &mut push_sample, &mut render_frame) {
                    render_requested = true;
                }
                self.pending_cycles = self.pending_cycles.saturating_sub(step);
            }
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
            let beeper = (self.bus.sys_vv55.port_c_out & 1) != 0;

            if let Some(sample) = self.audio_mixer.tick(vi53_mixed, beeper) {
                push_sample(sample);
            }

            self.cclk_acc += CPU_DIV;
            while self.cclk_acc >= CCLK_DIV {
                self.cclk_acc -= CCLK_DIV;

                if self.bus.vg75.tick(&mut self.bus.vt57, &self.bus.ram) {
                    render_frame(&self.bus.vg75);
                    render_requested = true;
                }
            }
        }

        render_requested
    }
}
