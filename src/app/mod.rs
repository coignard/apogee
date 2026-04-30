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
pub mod video;

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
use crate::app::video::{ColorMode, SCREEN_HEIGHT, SCREEN_WIDTH, VideoRenderer};
use crate::core::machine::Machine;

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

        let size = LogicalSize::new((SCREEN_WIDTH * 2) as f64, (SCREEN_HEIGHT * 2) as f64);

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

        self.pixels = Some(
            Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface)
                .expect("Failed to create pixels surface"),
        );
        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key) = event.physical_key
                    && !event.repeat
                    && let Some((row, col)) = map_keycode(key)
                {
                    let pressed = event.state == ElementState::Pressed;
                    self.machine.update_key(row, col, pressed);
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

        let cpu_freq = crate::core::machine::MASTER_CLOCK_HZ / crate::core::machine::CPU_DIVIDER;
        let samples_per_frame = (self.audio.sample_rate as u64
            * crate::core::machine::DEFAULT_FRAME_CYCLES as u64)
            / cpu_freq as u64;
        let latency_samples = ((samples_per_frame * 3) / 2) as usize;

        if self.audio.tx.len() >= latency_samples {
            event_loop.set_control_flow(ControlFlow::Poll);
            std::thread::yield_now();
            return;
        }

        event_loop.set_control_flow(ControlFlow::Poll);

        let mut frame_ready_for_render = false;
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
                frame_ready_for_render = true;
            }
        }

        if !audio_alive {
            self.fatal_error = Some(anyhow::anyhow!("Audio device disconnected"));
            event_loop.exit();
            return;
        }

        if frame_ready_for_render {
            self.video.render_frame(self.machine.vg75());
            if let Some(w) = &self.window {
                w.request_redraw();
            }
        }
    }
}
