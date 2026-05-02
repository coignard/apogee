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

pub mod audio;
pub mod keyboard;

use std::sync::Arc;
use std::time::{Duration, Instant};

use crossbeam_channel::Sender;
use pixels::{Pixels, SurfaceTexture};
use spin_sleep::SpinSleeper;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::window::{Window, WindowId};

use crate::app::audio::AudioSystem;
use crate::app::keyboard::map_keycode;

use apogee_rs::core::debug::{ReplayPlayer, ReplayRecorder};
use apogee_rs::core::machine::{CPU_DIVIDER, DEFAULT_FRAME_CYCLES, MASTER_CLOCK_HZ, Machine};
use apogee_rs::core::video::{ColorMode, VideoRenderer};

const MIDI_CHANNEL_CAPACITY: usize = 4096;
const MIDI_STATUS_CONTROL_CHANGE: u8 = 0xB0;
const MIDI_CC_ALL_SOUND_OFF: u8 = 120;
const MIDI_CC_ALL_NOTES_OFF: u8 = 123;
const MIDI_CHANNELS_COUNT: u8 = 16;

pub struct AppConfig {
    pub debug_mode: bool,
    pub recorder: Option<ReplayRecorder>,
    pub player: Option<ReplayPlayer>,
    pub rom_name: String,
    pub midi_out: Option<midir::MidiOutputConnection>,
}

pub struct App {
    machine: Machine,
    video: VideoRenderer,
    audio: AudioSystem,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,

    pub debug_mode: bool,
    pub paused: bool,
    pub step_frame: bool,
    pub recorder: Option<ReplayRecorder>,
    pub player: Option<ReplayPlayer>,
    pub rom_name: String,

    midi_tx: Option<Sender<(Vec<u8>, u64)>>,
    midi_stream: midly::stream::MidiStream,
    midi_encode_buf: Vec<u8>,

    pub fatal_error: Option<anyhow::Error>,
}

impl App {
    pub fn new(
        machine: Machine,
        video: VideoRenderer,
        audio: AudioSystem,
        config: AppConfig,
    ) -> Self {
        let cpu_freq = MASTER_CLOCK_HZ as f64 / CPU_DIVIDER as f64;
        let frame_duration_secs = DEFAULT_FRAME_CYCLES as f64 / cpu_freq;
        let sync_lag_threshold = Duration::from_secs_f64(frame_duration_secs * 3.0);

        let midi_tx = config.midi_out.map(|mut midi_conn| {
            let (tx, rx) = crossbeam_channel::bounded::<(Vec<u8>, u64)>(MIDI_CHANNEL_CAPACITY);
            std::thread::spawn(move || {
                let sleeper = SpinSleeper::default();
                let mut anchor: Option<(Instant, u64)> = None;

                while let Ok((msg, target_cycle)) = rx.recv() {
                    let now = Instant::now();
                    let (anchor_time, anchor_cycle) = *anchor.get_or_insert((now, target_cycle));

                    let delta_cycles = target_cycle.saturating_sub(anchor_cycle);
                    let target_time =
                        anchor_time + Duration::from_secs_f64(delta_cycles as f64 / cpu_freq);

                    if target_time > now {
                        sleeper.sleep_until(target_time);
                    } else if now.duration_since(target_time) > sync_lag_threshold {
                        anchor = Some((now, target_cycle));
                    }

                    let _ = midi_conn.send(&msg);
                }

                for channel in 0..MIDI_CHANNELS_COUNT {
                    let _ = midi_conn.send(&[
                        MIDI_STATUS_CONTROL_CHANGE | channel,
                        MIDI_CC_ALL_NOTES_OFF,
                        0,
                    ]);
                    let _ = midi_conn.send(&[
                        MIDI_STATUS_CONTROL_CHANGE | channel,
                        MIDI_CC_ALL_SOUND_OFF,
                        0,
                    ]);
                }
            });
            tx
        });

        Self {
            machine,
            video,
            audio,
            window: None,
            pixels: None,
            debug_mode: config.debug_mode,
            paused: false,
            step_frame: false,
            recorder: config.recorder,
            player: config.player,
            rom_name: config.rom_name,
            fatal_error: None,
            midi_tx,
            midi_stream: midly::stream::MidiStream::new(),
            midi_encode_buf: Vec::with_capacity(3),
        }
    }

    fn cycle(&mut self) -> Result<bool, ()> {
        if let Some(player) = &mut self.player {
            let _ = player.apply_pending_events(&mut self.machine);
        }

        let mut audio_alive = true;
        let vblank_occurred = self.machine.tick(|sample| {
            if let Err(crossbeam_channel::TrySendError::Disconnected(_)) =
                self.audio.tx.try_send(sample)
            {
                audio_alive = false;
            }
        });

        self.process_midi_events();

        if !audio_alive {
            Err(())
        } else {
            Ok(vblank_occurred)
        }
    }

    fn process_midi_events(&mut self) {
        if let Some(tx) = &self.midi_tx {
            let stream = &mut self.midi_stream;
            let encode_buf = &mut self.midi_encode_buf;

            self.machine.drain_midi_out(|events| {
                for &(byte, cycle) in events {
                    stream.feed(&[byte], |live_event| {
                        encode_buf.clear();
                        if live_event.write_std(&mut *encode_buf).is_ok() {
                            let _ = tx.try_send((encode_buf.clone(), cycle));
                        }
                    });
                }
            });
        } else {
            self.machine.drain_midi_out(|_| {});
        }
    }

    fn dump_snapshot(&self, name: &str) {
        let json_name = format!("{}.json", name);
        let png_name = format!("{}.png", name);

        if let Ok(file) = std::fs::File::create(&json_name) {
            let writer = std::io::BufWriter::new(file);
            let _ = serde_json::to_writer_pretty(writer, &self.machine.state());
        }

        let w = self.video.width();
        let h = self.video.height();
        let buffer = self.video.frame_buffer();

        let _ = image::save_buffer(&png_name, buffer, w, h, image::ExtendedColorType::Rgba8);
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let title = match self.video.color_mode {
            ColorMode::Color => "Апогей БК-01Ц",
            ColorMode::Grayscale | ColorMode::Bw => "Апогей БК-01",
        };

        let width = self.video.width();
        let height = self.video.height();

        let size = LogicalSize::new((width * 2) as f64, (height * 2) as f64);

        let window = Arc::new(
            event_loop
                .create_window(
                    Window::default_attributes()
                        .with_title(title)
                        .with_inner_size(size),
                )
                .expect("Failed to create window"),
        );

        let surface = SurfaceTexture::new(
            window.inner_size().width,
            window.inner_size().height,
            window.clone(),
        );

        self.pixels =
            Some(Pixels::new(width, height, surface).expect("Failed to create pixels surface"));
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::Resized(new_size) if new_size.width > 0 && new_size.height > 0 => {
                if let Some(pixels) = &mut self.pixels
                    && let Err(err) = pixels.resize_surface(new_size.width, new_size.height)
                {
                    self.fatal_error =
                        Some(anyhow::Error::new(err).context("Pixels resize surface failed"));
                    event_loop.exit();
                    return;
                }
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                let pressed = event.state == ElementState::Pressed;

                if let PhysicalKey::Code(key_code) = event.physical_key
                    && !event.repeat
                {
                    if self.debug_mode {
                        match key_code {
                            KeyCode::F8 if pressed => {
                                self.paused = !self.paused;
                            }
                            KeyCode::F9 if pressed && self.paused => {
                                self.step_frame = true;
                            }
                            KeyCode::F10 if pressed => {
                                let frame = self.machine.cycle_count();
                                let snap_name = format!("{}_frame_{}", self.rom_name, frame);

                                self.dump_snapshot(&snap_name);

                                if let Some(rec) = &mut self.recorder {
                                    rec.push_snapshot(frame, snap_name.clone());
                                    let _ = rec.save(&format!("{}.json", self.rom_name));
                                }
                            }
                            _ => {}
                        }
                    }

                    if let Some(key) = map_keycode(key_code)
                        && self.player.is_none()
                    {
                        self.machine.update_key(key, pressed);

                        if let Some(rec) = &mut self.recorder {
                            rec.push_key(self.machine.cycle_count(), key, pressed);
                        }
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(pixels) = &mut self.pixels {
                    pixels
                        .frame_mut()
                        .copy_from_slice(self.video.frame_buffer());

                    if let Err(err) = pixels.render() {
                        self.fatal_error =
                            Some(anyhow::Error::new(err).context("Pixels render failed"));
                        event_loop.exit();
                    }
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        if let Ok(err) = self.audio.err_rx.try_recv() {
            self.fatal_error = Some(err);
            event_loop.exit();
            return;
        }

        if self.paused && !self.step_frame {
            event_loop.set_control_flow(ControlFlow::Wait);
            return;
        }

        event_loop.set_control_flow(ControlFlow::Poll);

        let mut frame_ready_for_render = false;
        let mut size_changed = false;
        let mut audio_alive = true;

        let cpu_freq =
            apogee_rs::core::machine::MASTER_CLOCK_HZ / apogee_rs::core::machine::CPU_DIVIDER;
        let samples_per_frame = (self.audio.sample_rate as u64
            * apogee_rs::core::machine::DEFAULT_FRAME_CYCLES as u64)
            / cpu_freq as u64;
        let latency_samples = ((samples_per_frame * 3) / 2) as usize;

        if self.step_frame {
            let mut vblank_occurred = false;
            while !vblank_occurred {
                match self.cycle() {
                    Ok(v) => vblank_occurred = v,
                    Err(_) => {
                        audio_alive = false;
                        break;
                    }
                }
            }
            if audio_alive && self.video.render_frame(self.machine.vg75()) {
                size_changed = true;
            }
            frame_ready_for_render = true;
            self.step_frame = false;
        } else {
            if self.audio.tx.len() >= latency_samples {
                std::thread::yield_now();
                return;
            }

            while self.audio.tx.len() < latency_samples && audio_alive {
                match self.cycle() {
                    Ok(vblank_occurred) => {
                        if vblank_occurred {
                            if self.video.render_frame(self.machine.vg75()) {
                                size_changed = true;
                            }
                            frame_ready_for_render = true;
                        }
                    }
                    Err(_) => {
                        audio_alive = false;
                    }
                }
            }
        }

        if !audio_alive {
            self.fatal_error = Some(anyhow::anyhow!("Audio device disconnected"));
            event_loop.exit();
            return;
        }

        if frame_ready_for_render {
            if size_changed {
                let w = self.video.width();
                let h = self.video.height();

                if let Some(pixels) = &mut self.pixels
                    && let Err(err) = pixels.resize_buffer(w, h)
                {
                    self.fatal_error =
                        Some(anyhow::Error::new(err).context("Pixels resize buffer failed"));
                    event_loop.exit();
                    return;
                }
            }

            if let Some(window) = &self.window {
                if size_changed {
                    let w = self.video.width() as f64 * 2.0;
                    let h = self.video.height() as f64 * 2.0;
                    let _ = window.request_inner_size(LogicalSize::new(w, h));
                }
                window.request_redraw();
            }
        }
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let Some(rec) = &self.recorder else { return };

        let filename = format!("{}.json", self.rom_name);

        if let Err(err) = rec.save(&filename) {
            self.fatal_error
                .get_or_insert_with(|| err.context(format!("Failed to save replay to {filename}")));
        }
    }
}
