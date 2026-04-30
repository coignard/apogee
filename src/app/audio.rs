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

use anyhow::{Context, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use crossbeam_channel::bounded;

const DC_BLOCKER_ALPHA: f32 = 0.999;

pub struct AudioSystem {
    pub sample_rate: u32,
    pub tx: crossbeam_channel::Sender<f32>,
    pub err_rx: crossbeam_channel::Receiver<anyhow::Error>,
    _stream: cpal::Stream,
}

impl AudioSystem {
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();
        let device = host
            .default_output_device()
            .context("No default audio output device available")?;
        let config = device
            .default_output_config()
            .context("Failed to get default audio output config")?;

        let sample_rate = config.sample_rate();
        let channels = config.channels() as usize;

        let channel_capacity = (sample_rate / 2) as usize;

        let (tx, rx) = bounded::<f32>(channel_capacity);
        let (err_tx, err_rx) = bounded::<anyhow::Error>(1);

        let mut dc_blocker = 0.0;
        let mut prev_mixed = 0.0;
        let mut last_mixed = 0.0;

        let stream = device
            .build_output_stream(
                &config.into(),
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    for frame in data.chunks_mut(channels) {
                        if let Ok(mixed) = rx.try_recv() {
                            last_mixed = mixed;
                        }

                        dc_blocker = last_mixed - prev_mixed + DC_BLOCKER_ALPHA * dc_blocker;
                        prev_mixed = last_mixed;

                        for sample in frame.iter_mut() {
                            *sample = dc_blocker;
                        }
                    }
                },
                move |err| {
                    let _ = err_tx.try_send(anyhow::Error::new(err).context("Audio stream error"));
                },
                None,
            )
            .context("Failed to build audio stream")?;

        stream.play().context("Failed to play audio stream")?;

        Ok(Self {
            sample_rate,
            tx,
            err_rx,
            _stream: stream,
        })
    }
}
