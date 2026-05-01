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

use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::fs::File;
use std::io::BufWriter;

use crate::core::machine::Machine;
use crate::core::peripherals::keyboard::Key;
use crate::core::video::ColorMode;

#[derive(Serialize, Deserialize)]
pub struct ReplayMetadata {
    pub rom_name: String,
    pub rom_sha256: String,
    pub autorun: bool,
    pub sample_rate: u32,
    pub color_mode: ColorMode,
    pub is_crt: bool,
}

#[derive(Serialize, Deserialize)]
pub enum ReplayAction {
    KeyDown { key: Key },
    KeyUp { key: Key },
    TakeSnapshot { name: String },
}

#[derive(Serialize, Deserialize)]
pub struct ReplayEvent {
    #[serde(rename = "frame")]
    pub cycle: u64,
    pub action: ReplayAction,
}

#[derive(Serialize, Deserialize)]
pub struct Replay {
    pub metadata: ReplayMetadata,
    pub events: Vec<ReplayEvent>,
}

pub struct ReplayRecorder {
    pub replay: Replay,
}

impl ReplayRecorder {
    pub fn new(metadata: ReplayMetadata) -> Self {
        Self {
            replay: Replay {
                metadata,
                events: Vec::new(),
            },
        }
    }

    pub fn push_key(&mut self, cycle: u64, key: Key, pressed: bool) {
        let action = if pressed {
            ReplayAction::KeyDown { key }
        } else {
            ReplayAction::KeyUp { key }
        };
        self.replay.events.push(ReplayEvent { cycle, action });
    }

    pub fn push_snapshot(&mut self, cycle: u64, name: String) {
        self.replay.events.push(ReplayEvent {
            cycle,
            action: ReplayAction::TakeSnapshot { name },
        });
    }

    pub fn save(&self, filename: &str) -> anyhow::Result<()> {
        let file = File::create(filename)?;
        let writer = BufWriter::new(file);
        serde_json::to_writer_pretty(writer, &self.replay)?;
        Ok(())
    }
}

pub struct ReplayPlayer {
    pub replay: Replay,
    current_event_idx: usize,
}

impl ReplayPlayer {
    pub fn from_file(path: &str) -> anyhow::Result<Self> {
        let file =
            File::open(path).with_context(|| format!("Failed to open replay file: {}", path))?;
        let reader = std::io::BufReader::new(file);
        let replay: Replay =
            serde_json::from_reader(reader).with_context(|| "Failed to parse replay JSON")?;

        Ok(Self {
            replay,
            current_event_idx: 0,
        })
    }

    pub fn verify_rom_hash(&self, loaded_rom_sha256: &str) -> anyhow::Result<()> {
        anyhow::ensure!(
            self.replay.metadata.rom_sha256 == loaded_rom_sha256,
            "replay SHA256 mismatch: expected {}, got {}",
            self.replay.metadata.rom_sha256,
            loaded_rom_sha256
        );
        Ok(())
    }

    pub fn apply_pending_events(&mut self, machine: &mut Machine) -> Vec<String> {
        let mut requested_snapshots = Vec::new();
        let current_cycle = machine.cycle_count();

        while self.current_event_idx < self.replay.events.len() {
            let event = &self.replay.events[self.current_event_idx];

            if current_cycle >= event.cycle {
                match &event.action {
                    ReplayAction::KeyDown { key } => {
                        machine.update_key(*key, true);
                    }
                    ReplayAction::KeyUp { key } => {
                        machine.update_key(*key, false);
                    }
                    ReplayAction::TakeSnapshot { name } => {
                        requested_snapshots.push(name.clone());
                    }
                }
                self.current_event_idx += 1;
            } else {
                break;
            }
        }
        requested_snapshots
    }

    pub fn is_finished(&self) -> bool {
        self.current_event_idx >= self.replay.events.len()
    }
}
