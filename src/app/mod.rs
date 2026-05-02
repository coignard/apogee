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
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
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
use apogee_rs::core::peripherals::keyboard::Key;
use apogee_rs::core::video::{ColorMode, VideoRenderer};

const MIDI_CHANNEL_CAPACITY: usize = 4096;
const MIDI_STATUS_CONTROL_CHANGE: u8 = 0xB0;
const MIDI_CC_ALL_SOUND_OFF: u8 = 120;
const MIDI_CC_ALL_NOTES_OFF: u8 = 123;
const MIDI_CHANNELS_COUNT: u8 = 16;

const FRAME_CHANNEL_CAPACITY: usize = 2;

struct EmulationFrame {
    width: u32,
    height: u32,
    buffer: Box<[u8]>,
}

enum EmulationCommand {
    KeyEvent { key: Key, pressed: bool },
    TogglePause,
    StepFrame,
    DumpSnapshot { rom_name: String },
    SaveReplay { rom_name: String },
    Quit,
}

enum EmulationError {
    AudioDisconnected,
}

pub struct AppConfig {
    pub debug_mode: bool,
    pub recorder: Option<ReplayRecorder>,
    pub player: Option<ReplayPlayer>,
    pub rom_name: String,
    pub midi_out: Option<midir::MidiOutputConnection>,
}

pub struct App {
    audio: AudioSystem,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,

    color_mode: ColorMode,
    initial_width: u32,
    initial_height: u32,
    current_width: u32,
    current_height: u32,

    debug_mode: bool,
    paused: bool,
    has_player: bool,
    rom_name: String,

    cmd_tx: Sender<EmulationCommand>,
    frame_rx: Receiver<EmulationFrame>,
    emu_err_rx: Receiver<EmulationError>,
    emu_thread: Option<JoinHandle<()>>,

    pub fatal_error: Option<anyhow::Error>,
}

impl App {
    pub fn new(
        machine: Machine,
        video: VideoRenderer,
        audio: AudioSystem,
        config: AppConfig,
    ) -> Self {
        let color_mode = video.color_mode;
        let initial_width = video.width();
        let initial_height = video.height();

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

        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<EmulationCommand>();
        let (frame_tx, frame_rx) =
            crossbeam_channel::bounded::<EmulationFrame>(FRAME_CHANNEL_CAPACITY);
        let (emu_err_tx, emu_err_rx) = crossbeam_channel::bounded::<EmulationError>(1);

        let audio_tx = audio.tx.clone();
        let sample_rate = audio.sample_rate;
        let has_player = config.player.is_some();

        let emu_thread = std::thread::Builder::new()
            .name("emulation".into())
            .spawn(move || {
                run_emulation(
                    machine,
                    video,
                    audio_tx,
                    sample_rate,
                    midi_tx,
                    config.recorder,
                    config.player,
                    cmd_rx,
                    frame_tx,
                    emu_err_tx,
                );
            })
            .expect("Failed to spawn emulation thread");

        Self {
            audio,
            window: None,
            pixels: None,
            color_mode,
            initial_width,
            initial_height,
            current_width: initial_width,
            current_height: initial_height,
            debug_mode: config.debug_mode,
            paused: false,
            has_player,
            rom_name: config.rom_name,
            cmd_tx,
            frame_rx,
            emu_err_rx,
            emu_thread: Some(emu_thread),
            fatal_error: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        if self.window.is_some() {
            return;
        }

        let title = match self.color_mode {
            ColorMode::Color => "Апогей БК-01Ц",
            ColorMode::Grayscale | ColorMode::Bw => "Апогей БК-01",
        };

        let width = self.initial_width;
        let height = self.initial_height;

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
                                let _ = self.cmd_tx.send(EmulationCommand::TogglePause);
                            }
                            KeyCode::F9 if pressed && self.paused => {
                                let _ = self.cmd_tx.send(EmulationCommand::StepFrame);
                            }
                            KeyCode::F10 if pressed => {
                                let _ = self.cmd_tx.send(EmulationCommand::DumpSnapshot {
                                    rom_name: self.rom_name.clone(),
                                });
                                let _ = self.cmd_tx.send(EmulationCommand::SaveReplay {
                                    rom_name: self.rom_name.clone(),
                                });
                            }
                            _ => {}
                        }
                    }

                    if let Some(key) = map_keycode(key_code)
                        && !self.has_player
                    {
                        let _ = self
                            .cmd_tx
                            .send(EmulationCommand::KeyEvent { key, pressed });
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                if let Some(pixels) = &mut self.pixels
                    && let Err(err) = pixels.render()
                {
                    self.fatal_error =
                        Some(anyhow::Error::new(err).context("Pixels render failed"));
                    event_loop.exit();
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

        if let Ok(emu_err) = self.emu_err_rx.try_recv() {
            self.fatal_error = Some(match emu_err {
                EmulationError::AudioDisconnected => {
                    anyhow::anyhow!("Audio device disconnected")
                }
            });
            event_loop.exit();
            return;
        }

        let mut latest_frame = None;
        while let Ok(frame) = self.frame_rx.try_recv() {
            latest_frame = Some(frame);
        }

        if let Some(frame) = latest_frame {
            let size_changed =
                frame.width != self.current_width || frame.height != self.current_height;

            if size_changed {
                self.current_width = frame.width;
                self.current_height = frame.height;

                if let Some(pixels) = &mut self.pixels
                    && let Err(err) = pixels.resize_buffer(frame.width, frame.height)
                {
                    self.fatal_error =
                        Some(anyhow::Error::new(err).context("Pixels resize buffer failed"));
                    event_loop.exit();
                    return;
                }
            }

            if let Some(pixels) = &mut self.pixels {
                pixels.frame_mut().copy_from_slice(&frame.buffer);
            }

            if let Some(window) = &self.window {
                if size_changed {
                    let w = frame.width as f64 * 2.0;
                    let h = frame.height as f64 * 2.0;
                    let _ = window.request_inner_size(LogicalSize::new(w, h));
                }
                window.request_redraw();
            }
        }

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.cmd_tx.send(EmulationCommand::SaveReplay {
            rom_name: self.rom_name.clone(),
        });
        let _ = self.cmd_tx.send(EmulationCommand::Quit);

        if let Some(handle) = self.emu_thread.take() {
            let _ = handle.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_emulation(
    mut machine: Machine,
    mut video: VideoRenderer,
    audio_tx: Sender<f32>,
    sample_rate: u32,
    midi_tx: Option<Sender<(Vec<u8>, u64)>>,
    mut recorder: Option<ReplayRecorder>,
    mut player: Option<ReplayPlayer>,
    cmd_rx: Receiver<EmulationCommand>,
    frame_tx: Sender<EmulationFrame>,
    emu_err_tx: Sender<EmulationError>,
) {
    let mut midi_stream = midly::stream::MidiStream::new();
    let mut midi_encode_buf = Vec::with_capacity(3);

    let cpu_freq = MASTER_CLOCK_HZ / CPU_DIVIDER;
    let samples_per_frame = (sample_rate as u64 * DEFAULT_FRAME_CYCLES as u64) / cpu_freq as u64;
    let latency_samples = ((samples_per_frame * 3) / 2) as usize;

    let mut paused = false;
    let mut step_frame = false;

    loop {
        loop {
            let cmd = if paused && !step_frame {
                match cmd_rx.recv() {
                    Ok(cmd) => cmd,
                    Err(_) => return,
                }
            } else {
                match cmd_rx.try_recv() {
                    Ok(cmd) => cmd,
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                    Err(crossbeam_channel::TryRecvError::Disconnected) => return,
                }
            };

            match cmd {
                EmulationCommand::KeyEvent { key, pressed } => {
                    machine.update_key(key, pressed);
                    if let Some(rec) = &mut recorder {
                        rec.push_key(machine.cycle_count(), key, pressed);
                    }
                }
                EmulationCommand::TogglePause => {
                    paused = !paused;
                }
                EmulationCommand::StepFrame => {
                    step_frame = true;
                }
                EmulationCommand::DumpSnapshot { rom_name } => {
                    let frame = machine.cycle_count();
                    let snap_name = format!("{}_frame_{}", rom_name, frame);
                    dump_snapshot(&machine, &video, &snap_name);

                    if let Some(rec) = &mut recorder {
                        rec.push_snapshot(frame, snap_name);
                    }
                }
                EmulationCommand::SaveReplay { rom_name } => {
                    if let Some(rec) = &recorder {
                        let _ = rec.save(&format!("{}.json", rom_name));
                    }
                }
                EmulationCommand::Quit => return,
            }
        }

        if step_frame {
            let mut vblank_occurred = false;
            while !vblank_occurred {
                match emu_cycle(
                    &mut machine,
                    &audio_tx,
                    &midi_tx,
                    &mut midi_stream,
                    &mut midi_encode_buf,
                    &mut player,
                ) {
                    Ok(v) => vblank_occurred = v,
                    Err(()) => {
                        let _ = emu_err_tx.send(EmulationError::AudioDisconnected);
                        return;
                    }
                }
            }
            video.render_frame(machine.vg75());
            send_frame(&video, &frame_tx);
            step_frame = false;
            continue;
        }

        if audio_tx.len() >= latency_samples {
            std::thread::yield_now();
            continue;
        }

        while audio_tx.len() < latency_samples {
            match emu_cycle(
                &mut machine,
                &audio_tx,
                &midi_tx,
                &mut midi_stream,
                &mut midi_encode_buf,
                &mut player,
            ) {
                Ok(vblank_occurred) => {
                    if vblank_occurred {
                        video.render_frame(machine.vg75());
                        send_frame(&video, &frame_tx);
                    }
                }
                Err(()) => {
                    let _ = emu_err_tx.send(EmulationError::AudioDisconnected);
                    return;
                }
            }
        }
    }
}

#[inline]
fn emu_cycle(
    machine: &mut Machine,
    audio_tx: &Sender<f32>,
    midi_tx: &Option<Sender<(Vec<u8>, u64)>>,
    midi_stream: &mut midly::stream::MidiStream,
    midi_encode_buf: &mut Vec<u8>,
    player: &mut Option<ReplayPlayer>,
) -> Result<bool, ()> {
    if let Some(player) = player {
        let _ = player.apply_pending_events(machine);
    }

    let mut audio_alive = true;
    let vblank_occurred = machine.tick(|sample| {
        if let Err(crossbeam_channel::TrySendError::Disconnected(_)) = audio_tx.try_send(sample) {
            audio_alive = false;
        }
    });

    if let Some(tx) = midi_tx {
        machine.drain_midi_out(|events| {
            for &(byte, cycle) in events {
                midi_stream.feed(&[byte], |live_event| {
                    midi_encode_buf.clear();
                    if live_event.write_std(&mut *midi_encode_buf).is_ok() {
                        let _ = tx.try_send((midi_encode_buf.clone(), cycle));
                    }
                });
            }
        });
    } else {
        machine.drain_midi_out(|_| {});
    }

    if !audio_alive {
        Err(())
    } else {
        Ok(vblank_occurred)
    }
}

#[inline]
fn send_frame(video: &VideoRenderer, frame_tx: &Sender<EmulationFrame>) {
    let frame = EmulationFrame {
        width: video.width(),
        height: video.height(),
        buffer: video.frame_buffer().into(),
    };
    let _ = frame_tx.try_send(frame);
}

fn dump_snapshot(machine: &Machine, video: &VideoRenderer, name: &str) {
    let json_name = format!("{}.json", name);
    let png_name = format!("{}.png", name);

    if let Ok(file) = std::fs::File::create(&json_name) {
        let writer = std::io::BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(writer, &machine.state());
    }

    let w = video.width();
    let h = video.height();
    let buffer = video.frame_buffer();

    let _ = image::save_buffer(&png_name, buffer, w, h, image::ExtendedColorType::Rgba8);
}
