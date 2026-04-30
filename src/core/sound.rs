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

const VOLUME_SCALE: f32 = 0.05;

pub struct AudioMixer {
    sample_rate: u32,
    master_clock_hz: u32,
    cpu_divider: u32,
    audio_phase: u32,
    audio_sum: i32,
    audio_samples: u32,
}

impl AudioMixer {
    pub fn new(sample_rate: u32, master_clock_hz: u32, cpu_divider: u32) -> Self {
        Self {
            sample_rate,
            master_clock_hz,
            cpu_divider,
            audio_phase: 0,
            audio_sum: 0,
            audio_samples: 0,
        }
    }

    pub fn tick(&mut self, vi53_mixed: i32, tape_out_state: bool) -> Option<f32> {
        let mut mixed = vi53_mixed;
        mixed += if tape_out_state { 1 } else { -1 };

        self.audio_sum += mixed;
        self.audio_samples += 1;

        self.audio_phase += self.sample_rate * self.cpu_divider;

        if self.audio_phase >= self.master_clock_hz {
            self.audio_phase -= self.master_clock_hz;

            if self.audio_samples > 0 {
                let avg_sample = self.audio_sum as f32 / self.audio_samples as f32;
                let out_sample = avg_sample * VOLUME_SCALE;

                self.audio_sum = 0;
                self.audio_samples = 0;

                return Some(out_sample);
            }
        }
        None
    }
}
