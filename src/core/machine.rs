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
use serde::Serialize;
use std::fmt;

use super::audio::AudioMixer;
use super::bus::Bus;
use super::chips::kr580vg75::Kr580Vg75;
pub use super::peripherals::keyboard::Key;
use crate::core::peripherals::UserPeripheral;

pub const MASTER_CLOCK_HZ: u32 = 16_000_000;
pub const CPU_DIVIDER: u32 = 9;
pub const CHAR_CLOCK_DIVIDER: u32 = 12;

const RESET_VECTOR: u16 = 0xF800;
const TAPE_SYNC_BYTE: u8 = 0xE6;

const AUTORUN_DELAY_CYCLES: u32 = 2_000_000;
const RKA_HEADER_SIZE: usize = 4;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MachineError {
    InvalidRkaLength,
    InvalidAddressRange,
    FileTooShort,
    ChecksumMissing,
    ChecksumMismatch { expected: u16, got: u16 },
}

impl fmt::Display for MachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRkaLength => write!(f, "file is too short to be a valid RKA"),
            Self::InvalidAddressRange => write!(f, "start address is greater than end address"),
            Self::FileTooShort => write!(f, "file is shorter than the expected data length"),
            Self::ChecksumMissing => write!(f, "checksum block missing"),
            Self::ChecksumMismatch { expected, got } => {
                write!(
                    f,
                    "checksum mismatch: expected={:04X}, got={:04X}",
                    expected, got
                )
            }
        }
    }
}

impl std::error::Error for MachineError {}

#[derive(Serialize)]
pub struct MachineState<'a> {
    #[serde(rename = "frame")]
    pub cycle: u64,
    pub pc: u16,
    pub bus: &'a Bus,
}

pub struct Machine {
    cpu: Cpu,
    bus: Bus,
    audio_mixer: AudioMixer,
    cclk_acc: u32,
    total_cycles: u64,
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
            total_cycles: 0,
        }
    }

    #[inline]
    pub fn vg75(&self) -> &Kr580Vg75 {
        &self.bus.vg75
    }

    #[inline]
    pub fn cycle_count(&self) -> u64 {
        self.total_cycles
    }

    pub fn state(&self) -> MachineState<'_> {
        MachineState {
            cycle: self.total_cycles,
            pc: self.cpu.immutable_registers().pc(),
            bus: &self.bus,
        }
    }

    pub fn validate_rka(payload: &[u8], force: bool) -> Result<(), MachineError> {
        if payload.len() < RKA_HEADER_SIZE {
            return Err(MachineError::InvalidRkaLength);
        }

        let (header, payload_data) = payload.split_at(RKA_HEADER_SIZE);
        let start_addr = u16::from_be_bytes(header[0..2].try_into().unwrap());
        let end_addr = u16::from_be_bytes(header[2..4].try_into().unwrap());

        if start_addr > end_addr && !force {
            return Err(MachineError::InvalidAddressRange);
        }

        let expected_len = if start_addr <= end_addr {
            (end_addr - start_addr) as usize + 1
        } else {
            payload_data.len()
        };

        if payload_data.len() < expected_len && !force {
            return Err(MachineError::FileTooShort);
        }

        let len = expected_len.min(payload_data.len());
        let (data, tail) = payload_data.split_at(len);

        if !force {
            let (mut cs_hi_excluding_last, mut cs_lo) = (0u8, 0u8);
            let mut cs_hi_including_last = 0u8;

            if let Some((&last_byte, body)) = data.split_last() {
                for &b in body {
                    let (new_lo, carry) = cs_lo.overflowing_add(b);
                    cs_lo = new_lo;
                    cs_hi_excluding_last = cs_hi_excluding_last
                        .wrapping_add(b)
                        .wrapping_add(u8::from(carry));
                }

                let (final_lo, carry) = cs_lo.overflowing_add(last_byte);
                cs_hi_including_last = cs_hi_excluding_last
                    .wrapping_add(last_byte)
                    .wrapping_add(u8::from(carry));
                cs_lo = final_lo;
            }

            let checksum_excluding_last = u16::from_be_bytes([cs_hi_excluding_last, cs_lo]);
            let checksum_including_last = u16::from_be_bytes([cs_hi_including_last, cs_lo]);

            let stored_cs = tail
                .windows(3)
                .find(|w| w[0] == TAPE_SYNC_BYTE)
                .map(|w| &w[1..3])
                .unwrap_or_else(|| &tail[tail.len().saturating_sub(2)..])
                .try_into()
                .map(u16::from_be_bytes)
                .map_err(|_| MachineError::ChecksumMissing)?;

            if stored_cs != checksum_excluding_last && stored_cs != checksum_including_last {
                return Err(MachineError::ChecksumMismatch {
                    expected: checksum_excluding_last,
                    got: stored_cs,
                });
            }
        }

        Ok(())
    }

    pub fn load_rka(
        &mut self,
        payload: &[u8],
        autorun: bool,
        force: bool,
    ) -> Result<(), MachineError> {
        Self::validate_rka(payload, force)?;

        let (header, payload_data) = payload.split_at(RKA_HEADER_SIZE);
        let start_addr = u16::from_be_bytes(header[0..2].try_into().unwrap());
        let end_addr = u16::from_be_bytes(header[2..4].try_into().unwrap());

        let expected_len = if start_addr <= end_addr {
            (end_addr - start_addr) as usize + 1
        } else {
            payload_data.len()
        };

        let len = expected_len.min(payload_data.len());
        let (data, _) = payload_data.split_at(len);

        if autorun {
            for _ in (0..AUTORUN_DELAY_CYCLES).step_by(DEFAULT_FRAME_CYCLES as usize) {
                self.tick(|_| {});
            }
        }

        for (i, &b) in data.iter().enumerate() {
            let addr = start_addr.wrapping_add(i as u16) as usize;
            self.bus.ram[addr] = b;
        }

        if autorun {
            self.cpu.registers().set_pc(start_addr);
        }

        Ok(())
    }

    pub fn plug_user_peripheral(&mut self, peripheral: UserPeripheral) {
        self.bus.user_slot = peripheral;
    }

    #[inline]
    pub fn drain_midi_out<F: FnMut(&[(u8, u64)])>(&mut self, mut f: F) {
        if let UserPeripheral::Midi(midi) = &mut self.bus.user_slot
            && !midi.out_buffer.is_empty()
        {
            f(&midi.out_buffer);
            midi.out_buffer.clear();
        }
    }

    pub fn update_key(&mut self, key: Key, pressed: bool) {
        self.bus.keyboard.update_key(key, pressed);
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
            self.total_cycles += elapsed_cycles as u64;

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
        self.bus.current_cycle = self.total_cycles;
        let cycles_before = self.cpu.cycle_count();
        self.cpu.execute_instruction(&mut self.bus);
        let cycles_after = self.cpu.cycle_count();

        let (inte, _) = self.cpu.immutable_registers().get_interrupt_mode();
        self.bus.vg75.set_inte(inte);

        (cycles_after - cycles_before) as u32
    }
}
