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

use pixels::{Pixels, SurfaceTexture};
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::PhysicalKey;
use winit::window::{Window, WindowId};

use crate::app::audio::AudioSystem;
use crate::app::keyboard::map_keycode;

use apogee_rs::core::machine::Machine;
use apogee_rs::core::video::{ColorMode, VideoRenderer};

pub struct App {
    machine: Machine,
    video: VideoRenderer,
    audio: AudioSystem,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    pub fatal_error: Option<anyhow::Error>,
}

impl App {
    pub fn new(machine: Machine, video: VideoRenderer, audio: AudioSystem) -> Self {
        Self {
            machine,
            video,
            audio,
            window: None,
            pixels: None,
            fatal_error: None,
        }
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
                if let PhysicalKey::Code(key_code) = event.physical_key
                    && !event.repeat
                    && let Some(key) = map_keycode(key_code)
                {
                    let pressed = event.state == ElementState::Pressed;
                    self.machine.update_key(key, pressed);
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

        let cpu_freq =
            apogee_rs::core::machine::MASTER_CLOCK_HZ / apogee_rs::core::machine::CPU_DIVIDER;
        let samples_per_frame = (self.audio.sample_rate as u64
            * apogee_rs::core::machine::DEFAULT_FRAME_CYCLES as u64)
            / cpu_freq as u64;
        let latency_samples = ((samples_per_frame * 3) / 2) as usize;

        if self.audio.tx.len() >= latency_samples {
            event_loop.set_control_flow(ControlFlow::Poll);
            std::thread::yield_now();
            return;
        }

        event_loop.set_control_flow(ControlFlow::Poll);

        let mut frame_ready_for_render = false;
        let mut size_changed = false;
        let mut audio_alive = true;
        let tx = &self.audio.tx;

        while tx.len() < latency_samples && audio_alive {
            let vblank_occurred = self.machine.tick(|sample| match tx.try_send(sample) {
                Ok(_) => {}
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    audio_alive = false;
                }
                Err(crossbeam_channel::TrySendError::Full(_)) => {}
            });

            if vblank_occurred {
                if self.video.render_frame(self.machine.vg75()) {
                    size_changed = true;
                }
                frame_ready_for_render = true;
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

            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }
}
