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
use std::time::{Duration, Instant};

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
use crate::core::machine::ApogeeMachine;

const MAX_QUEUED_AUDIO_SAMPLES: usize = 2048;
const THROTTLE_WAIT_MS: u64 = 1;

pub struct App {
    machine: ApogeeMachine,
    video: VideoRenderer,
    audio: AudioSystem,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    last_time: Instant,
}

impl App {
    pub fn new(mut machine: ApogeeMachine, video: VideoRenderer) -> Self {
        let audio = AudioSystem::new();
        machine.set_sample_rate(audio.sample_rate);

        Self {
            machine,
            video,
            audio,
            window: None,
            pixels: None,
            last_time: Instant::now(),
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
        self.pixels = Some(Pixels::new(SCREEN_WIDTH, SCREEN_HEIGHT, surface).unwrap());
        self.window = Some(window);
        self.last_time = Instant::now();
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
                        eprintln!("pixels.render failed: {err}");
                        event_loop.exit();
                    }
                }
            }
            _ => (),
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let now = Instant::now();
        let dt = now.duration_since(self.last_time).as_secs_f32().min(0.05);
        self.last_time = now;

        if self.audio.tx.len() > MAX_QUEUED_AUDIO_SAMPLES {
            event_loop.set_control_flow(ControlFlow::WaitUntil(
                Instant::now() + Duration::from_millis(THROTTLE_WAIT_MS),
            ));
            return;
        }

        event_loop.set_control_flow(ControlFlow::Poll);

        let mut frame_rendered = false;
        let tx = &self.audio.tx;
        let video = &mut self.video;

        self.machine.run(
            dt,
            |sample| {
                let _ = tx.try_send(sample);
            },
            |vg75| {
                video.render_frame(vg75);
                frame_rendered = true;
            },
        );

        if frame_rendered
            && let Some(w) = &self.window {
                w.request_redraw();
            }
    }
}
