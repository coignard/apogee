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
use crate::core::bus::memory_map;
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
    MemoryOverflow,
    FileTooShort,
    ChecksumMissing,
    ChecksumMismatch { expected: u16, got: u16 },
}

impl fmt::Display for MachineError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidRkaLength => write!(f, "file is too short to be a valid RKA"),
            Self::MemoryOverflow => write!(f, "program will not fit into available memory"),
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
    pub fn new(monitor_rom: Vec<u8>, sample_rate: u32) -> Self {
        let mut cpu = Cpu::new_8080();
        cpu.registers().set_pc(RESET_VECTOR);

        Self {
            cpu,
            bus: Bus::new(monitor_rom),
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
    pub fn font_banks(&self) -> &[bool; 64] {
        &self.bus.font_banks.rows
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

    pub fn validate_rka(payload: &[u8], force: bool) -> Result<(u16, &[u8]), MachineError> {
        if payload.len() < RKA_HEADER_SIZE {
            return Err(MachineError::InvalidRkaLength);
        }

        let start_addr = u16::from_be_bytes([payload[0], payload[1]]);
        let end_addr = u16::from_be_bytes([payload[2], payload[3]]);

        if end_addr < start_addr {
            return Err(MachineError::InvalidRkaLength);
        }

        let size = (end_addr - start_addr) as usize + 1;

        if start_addr as usize + size > memory_map::RAM_END as usize + 1 {
            return Err(MachineError::MemoryOverflow);
        }

        let payload_data = &payload[RKA_HEADER_SIZE..];

        if payload_data.len() < size && !force {
            return Err(MachineError::FileTooShort);
        }

        let len = size.min(payload_data.len());
        let (data, tail) = payload_data.split_at(len);

        if !force {
            let mut cs_lo = 0u8;
            let mut cs_hi = 0u8;

            if let Some((&last_byte, body)) = data.split_last() {
                for &b in body {
                    let (new_lo, carry) = cs_lo.overflowing_add(b);
                    cs_lo = new_lo;
                    cs_hi = cs_hi.wrapping_add(b).wrapping_add(u8::from(carry));
                }
                cs_lo = cs_lo.wrapping_add(last_byte);
            }

            let expected_cs = u16::from_be_bytes([cs_hi, cs_lo]);

            let stored_cs_slice = tail
                .windows(3)
                .find(|w| w[0] == TAPE_SYNC_BYTE)
                .map(|w| &w[1..3])
                .or_else(|| {
                    let offset = tail.len().checked_sub(2)?;
                    tail.get(offset..)
                })
                .unwrap_or(&[]);

            let stored_cs = stored_cs_slice
                .try_into()
                .map(u16::from_be_bytes)
                .map_err(|_| MachineError::ChecksumMissing)?;

            if stored_cs != expected_cs {
                return Err(MachineError::ChecksumMismatch {
                    expected: expected_cs,
                    got: stored_cs,
                });
            }
        }

        Ok((start_addr, data))
    }

    pub fn load_rka(
        &mut self,
        payload: &[u8],
        autorun: bool,
        force: bool,
    ) -> Result<(), MachineError> {
        let (start_addr, data) = Self::validate_rka(payload, force)?;

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
            let vg75_drq = self.bus.vg75.drq();
            self.bus.vt57.set_drq(2, vg75_drq);

            let elapsed_cycles = if self.bus.vt57.hrq() {
                self.bus.vt57.set_hlda(true);

                if let Some((channel, addr, _tc)) = self.bus.vt57.dma_transfer_cycle()
                    && channel == 2
                {
                    let byte = self.bus.ram[addr as usize];
                    self.bus.vg75.dack(byte);
                }
                4
            } else {
                self.bus.vt57.set_hlda(false);
                self.execute_cpu_instruction()
            };

            frame_cycles += elapsed_cycles;
            self.total_cycles += elapsed_cycles as u64;

            for _ in 0..elapsed_cycles {
                let vi53_mixed = self.bus.vi53.tick();
                let tape_out = (self.bus.sys_vv55.peripheral_read_c() & 0x01) != 0;

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

            let (inte, _) = self.cpu.immutable_registers().get_interrupt_mode();
            let cur_row = self.bus.vg75.current_row();

            if self.bus.font_banks.previous_row > cur_row {
                self.bus.font_banks.previous_row = 0;
            }

            for r in self.bus.font_banks.previous_row..=cur_row {
                if r < self.bus.font_banks.rows.len() {
                    self.bus.font_banks.rows[r] = inte;
                }
            }
            self.bus.font_banks.previous_row = cur_row;
        }

        vblank_occurred
    }

    #[inline]
    fn execute_cpu_instruction(&mut self) -> u32 {
        self.bus.current_cycle = self.total_cycles;
        let cycles_before = self.cpu.cycle_count();
        self.cpu.execute_instruction(&mut self.bus);
        let cycles_after = self.cpu.cycle_count();

        if self.cpu.is_halted() {
            4
        } else {
            (cycles_after - cycles_before) as u32
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rka_checksum_validation() {
        let payload_data: [u8; 3] = [0x10, 0x20, 0x30];

        let mut valid_dump = Vec::new();
        valid_dump.extend_from_slice(&0x0000u16.to_be_bytes());
        valid_dump.extend_from_slice(&0x0002u16.to_be_bytes());
        valid_dump.extend_from_slice(&payload_data);
        valid_dump.extend_from_slice(&[0x00, 0x00, TAPE_SYNC_BYTE, 0x30, 0x60]);

        assert!(Machine::validate_rka(&valid_dump, false).is_ok());

        let mut dump_invalid_checksum = valid_dump.clone();
        let len = dump_invalid_checksum.len();
        dump_invalid_checksum[len - 2..].copy_from_slice(&[0x99, 0x99]);

        assert!(matches!(
            Machine::validate_rka(&dump_invalid_checksum, false),
            Err(MachineError::ChecksumMismatch {
                expected: 0x3060,
                got: 0x9999
            })
        ));

        let mut dump_memory_overflow = Vec::new();
        dump_memory_overflow.extend_from_slice(&memory_map::RAM_END.to_be_bytes());
        dump_memory_overflow.extend_from_slice(&(memory_map::RAM_END + 1).to_be_bytes());
        dump_memory_overflow.extend_from_slice(&[0x00, 0x00]);

        assert!(matches!(
            Machine::validate_rka(&dump_memory_overflow, false),
            Err(MachineError::MemoryOverflow)
        ));
    }

    #[test]
    fn test_rka_assets_checksums() {
        let load_asset = |name: &str| -> Vec<u8> {
            let path = format!("{}/tests/assets/{}", env!("CARGO_MANIFEST_DIR"), name);
            std::fs::read(&path).unwrap_or_else(|_| panic!("Missing test asset: {}", path))
        };

        let valid_files = ["dvigalka.rka", "proverka.rka"];
        for file in valid_files {
            let data = load_asset(file);
            assert!(Machine::validate_rka(&data, false).is_ok());
        }

        let invalid_files = [
            ("ducks.rka", 0x2061),
            ("kindzadza.rka", 0x2FED),
            ("kletavt.rka", 0x4E9A),
            ("rc.rka", 0x77D4),
            ("robocop.rka", 0x4B5E),
            ("tia2.rka", 0xF163),
        ];

        for (file, expected_got) in invalid_files {
            let data = load_asset(file);
            let result = Machine::validate_rka(&data, false);

            match result {
                Err(MachineError::ChecksumMismatch { got, .. }) => {
                    assert_eq!(got, expected_got);
                }
                _ => panic!("Expected ChecksumMismatch for file {}", file),
            }
        }
    }
}
