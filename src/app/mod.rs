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
pub mod shaders;

use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use crossbeam_channel::{Receiver, Sender};
use pixels::{Pixels, SurfaceTexture};
#[cfg(not(target_os = "macos"))]
use spin_sleep::SpinSleeper;
use winit::application::ApplicationHandler;
use winit::dpi::LogicalSize;
use winit::event::{ElementState, WindowEvent};
use winit::event_loop::{ActiveEventLoop, ControlFlow};
use winit::keyboard::{KeyCode, ModifiersState, PhysicalKey};
use winit::window::{Fullscreen, Window, WindowId};

use crate::app::audio::AudioSystem;
use crate::app::keyboard::{KeyboardLayout, KeyboardTranslator};
use crate::app::shaders::{Preset, ShaderRenderer};

use apogee_rs::core::debug::{ReplayPlayer, ReplayRecorder};
use apogee_rs::core::machine::{CPU_DIVIDER, DEFAULT_FRAME_CYCLES, MASTER_CLOCK_HZ, Machine};
use apogee_rs::core::peripherals::UserPeripheral;
use apogee_rs::core::peripherals::keyboard::Key;
use apogee_rs::core::peripherals::midi::MidiInterface;
use apogee_rs::core::peripherals::romdisk::RomDisk;
use apogee_rs::core::video::{ColorMode, VideoRenderer};

const MIDI_STATUS_NOTE_OFF: u8 = 0x80;
const MIDI_STATUS_NOTE_ON: u8 = 0x90;
const MIDI_STATUS_CONTROL_CHANGE: u8 = 0xB0;
const MIDI_STATUS_MASK: u8 = 0xF0;
const MIDI_CHANNEL_MASK: u8 = 0x0F;
const MIDI_NOTE_MASK: u8 = 0x7F;
const MIDI_CC_ALL_SOUND_OFF: u8 = 120;
const MIDI_CC_ALL_NOTES_OFF: u8 = 123;
const MIDI_CHANNELS_COUNT: u8 = 16;
const MIDI_VOICE_MSG_LEN: usize = 3;
const MIDI_SYNC_LAG_FRAMES: f64 = 3.0;

const FRAME_CHANNEL_CAPACITY: usize = 2;
const AUDIO_LATENCY_FRAMES_NUMER: u64 = 3;
const AUDIO_LATENCY_FRAMES_DENOM: u64 = 2;
const WINDOW_SCALE: f64 = 2.0;

#[cfg(not(target_os = "macos"))]
pub struct MidiConn(midir::MidiOutputConnection);

#[cfg(not(target_os = "macos"))]
impl MidiConn {
    pub fn new(conn: midir::MidiOutputConnection) -> Self {
        Self(conn)
    }

    #[inline]
    pub fn send_now(&mut self, msg: &[u8]) {
        let _ = self.0.send(msg);
    }
}

#[cfg(target_os = "macos")]
pub struct MidiConn {
    port: coremidi::OutputPort,
    dest: coremidi::Destination,
}

#[cfg(target_os = "macos")]
impl MidiConn {
    pub fn new(port: coremidi::OutputPort, dest: coremidi::Destination) -> Self {
        Self { port, dest }
    }

    #[inline]
    pub fn send_now(&mut self, msg: &[u8]) {
        self.send_at(msg, 0);
    }

    #[inline]
    pub fn send_at(&mut self, msg: &[u8], host_ts: u64) {
        let buf = coremidi::PacketBuffer::new(host_ts, msg);
        let _ = self.port.send(&self.dest, &buf);
    }
}

#[cfg(target_os = "macos")]
#[link(name = "CoreAudio", kind = "framework")]
unsafe extern "C" {
    fn AudioGetCurrentHostTime() -> u64;
    fn AudioConvertNanosToHostTime(in_nanos: u64) -> u64;
}

#[cfg(target_os = "macos")]
#[inline]
fn now_host() -> u64 {
    unsafe { AudioGetCurrentHostTime() }
}

#[cfg(target_os = "macos")]
#[inline]
fn nanos_to_host(nanos: u64) -> u64 {
    unsafe { AudioConvertNanosToHostTime(nanos) }
}

#[cfg(not(target_os = "macos"))]
fn run_midi_thread(
    mut midi_conn: MidiConn,
    rx: Receiver<MidiThreadMsg>,
    cpu_freq: f64,
    frame_duration_secs: f64,
    sync_lag_threshold: Duration,
) {
    let sleeper = SpinSleeper::default();
    let mut anchor: Option<(Instant, u64)> = None;
    let mut active_notes = [0u128; MIDI_CHANNELS_COUNT as usize];
    let mut queue = std::collections::VecDeque::new();
    let mut fast_forward = false;

    loop {
        let msg = if let Some(m) = queue.pop_front() {
            m
        } else {
            match rx.recv() {
                Ok(m) => m,
                Err(_) => break,
            }
        };

        match msg {
            MidiThreadMsg::SetFastForward(ff) => {
                fast_forward = ff;
                if !ff {
                    anchor = None;
                }
            }
            MidiThreadMsg::HardReset => {
                silence_active_notes(&mut active_notes, &mut midi_conn);
                anchor = None;
                queue.clear();
            }
            MidiThreadMsg::Event(midi_data, target_cycle) => {
                if fast_forward {
                    track_note(&midi_data, &mut active_notes);
                    midi_conn.send_now(&midi_data);
                    continue;
                }

                let now = Instant::now();
                let (anchor_time, anchor_cycle) = *anchor.get_or_insert_with(|| {
                    let latency_secs = frame_duration_secs
                        * (AUDIO_LATENCY_FRAMES_NUMER as f64 / AUDIO_LATENCY_FRAMES_DENOM as f64);
                    let delay = Duration::from_secs_f64(latency_secs);
                    (now + delay, target_cycle)
                });

                let delta_cycles = target_cycle.saturating_sub(anchor_cycle);
                let target_time =
                    anchor_time + Duration::from_secs_f64(delta_cycles as f64 / cpu_freq);

                let mut aborted = false;
                loop {
                    let current_now = Instant::now();
                    if current_now >= target_time {
                        break;
                    }

                    let remaining = target_time.duration_since(current_now);
                    if remaining > Duration::from_millis(2) {
                        let deadline = target_time - Duration::from_millis(2);
                        match rx.recv_deadline(deadline) {
                            Ok(MidiThreadMsg::HardReset) => {
                                silence_active_notes(&mut active_notes, &mut midi_conn);
                                anchor = None;
                                queue.clear();
                                aborted = true;
                                break;
                            }
                            Ok(MidiThreadMsg::SetFastForward(true)) => {
                                fast_forward = true;
                                anchor = None;
                                aborted = true;
                                track_note(&midi_data, &mut active_notes);
                                midi_conn.send_now(&midi_data);
                                break;
                            }
                            Ok(MidiThreadMsg::SetFastForward(false)) => {
                                fast_forward = false;
                            }
                            Ok(event @ MidiThreadMsg::Event(..)) => {
                                queue.push_back(event);
                            }
                            Err(crossbeam_channel::RecvTimeoutError::Timeout) => {}
                            Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
                                aborted = true;
                                break;
                            }
                        }
                    } else {
                        sleeper.sleep_until(target_time);
                        break;
                    }
                }

                if !aborted {
                    if Instant::now().duration_since(target_time) > sync_lag_threshold {
                        track_note(&midi_data, &mut active_notes);

                        while let Some(stale) = queue.pop_front().or_else(|| rx.try_recv().ok()) {
                            match stale {
                                MidiThreadMsg::Event(d, _) => {
                                    track_note(&d, &mut active_notes);
                                }
                                MidiThreadMsg::SetFastForward(ff) => {
                                    fast_forward = ff;
                                }
                                MidiThreadMsg::HardReset => break,
                            }
                        }

                        silence_active_notes(&mut active_notes, &mut midi_conn);
                        anchor = None;
                    } else {
                        track_note(&midi_data, &mut active_notes);
                        midi_conn.send_now(&midi_data);
                    }
                }
            }
        }
    }

    silence_active_notes(&mut active_notes, &mut midi_conn);
}

#[cfg(target_os = "macos")]
fn run_midi_thread(
    mut midi_conn: MidiConn,
    rx: Receiver<MidiThreadMsg>,
    cpu_freq: f64,
    frame_duration_secs: f64,
    sync_lag_threshold: Duration,
) {
    let lag_threshold_host = nanos_to_host(sync_lag_threshold.as_nanos() as u64);

    let mut anchor: Option<(u64, u64)> = None;
    let mut active_notes = [0u128; MIDI_CHANNELS_COUNT as usize];
    let mut fast_forward = false;

    while let Ok(msg) = rx.recv() {
        match msg {
            MidiThreadMsg::SetFastForward(ff) => {
                fast_forward = ff;
                if !ff {
                    anchor = None;
                }
            }
            MidiThreadMsg::HardReset => {
                let _ = coremidi::flush();
                silence_active_notes(&mut active_notes, &mut midi_conn);
                anchor = None;
            }
            MidiThreadMsg::Event(midi_data, target_cycle) => {
                if fast_forward {
                    track_note(&midi_data, &mut active_notes);
                    midi_conn.send_now(&midi_data);
                    continue;
                }

                let (anchor_host, anchor_cycle) = *anchor.get_or_insert_with(|| {
                    let latency_secs = frame_duration_secs
                        * (AUDIO_LATENCY_FRAMES_NUMER as f64 / AUDIO_LATENCY_FRAMES_DENOM as f64);
                    let latency_host = nanos_to_host((latency_secs * 1.0e9) as u64);
                    (now_host() + latency_host, target_cycle)
                });

                let delta_cycles = target_cycle.saturating_sub(anchor_cycle);
                let delta_nanos = (delta_cycles as f64 / cpu_freq * 1.0e9) as u64;
                let target_host = anchor_host + nanos_to_host(delta_nanos);

                let now = now_host();
                if now > target_host && now - target_host > lag_threshold_host {
                    track_note(&midi_data, &mut active_notes);
                    while let Ok(stale) = rx.try_recv() {
                        match stale {
                            MidiThreadMsg::Event(d, _) => track_note(&d, &mut active_notes),
                            MidiThreadMsg::SetFastForward(ff) => fast_forward = ff,
                            MidiThreadMsg::HardReset => break,
                        }
                    }
                    let _ = coremidi::flush();
                    silence_active_notes(&mut active_notes, &mut midi_conn);
                    anchor = None;
                } else {
                    track_note(&midi_data, &mut active_notes);
                    midi_conn.send_at(&midi_data, target_host);
                }
            }
        }
    }

    let _ = coremidi::flush();
    silence_active_notes(&mut active_notes, &mut midi_conn);
}

#[derive(Clone)]
pub struct MachineConfig {
    pub monitor_rom: Arc<[u8]>,
    pub sample_rate: u32,
    pub rka: Option<(Arc<[u8]>, bool, bool)>,
    pub romdisk: Option<Arc<[u8]>>,
    pub midi_enabled: bool,
    pub rom_name: String,
}

impl MachineConfig {
    pub fn new_machine(&self) -> Machine {
        let mut machine = Machine::new(self.monitor_rom.to_vec(), self.sample_rate);

        if let Some((rka, autorun, force)) = &self.rka {
            machine
                .load_rka(rka, *autorun, *force)
                .expect("RKA validation was performed earlier");
        }

        if let Some(rom) = &self.romdisk {
            let mut romdisk = RomDisk::new();
            romdisk.load(rom);
            machine.plug_user_peripheral(UserPeripheral::RomDisk(romdisk));
        } else if self.midi_enabled {
            machine.plug_user_peripheral(UserPeripheral::Midi(MidiInterface::new()));
        }

        machine
    }
}

struct EmulationFrame {
    width: u32,
    height: u32,
    buffer: Box<[u8]>,
}

enum EmulationCommand {
    KeyEvent { key: Key, pressed: bool },
    TogglePause,
    StepFrame,
    SetFastForward(bool),
    HardReset,
    DumpSnapshot,
    SaveReplay,
    Quit,
}

enum EmulationError {
    AudioDisconnected,
}

enum MidiThreadMsg {
    Event(Vec<u8>, u64),
    HardReset,
    SetFastForward(bool),
}

pub struct AppConfig {
    pub debug_mode: bool,
    pub recorder: Option<ReplayRecorder>,
    pub player: Option<ReplayPlayer>,
    pub midi_out: Option<MidiConn>,
    pub keyboard_layout: KeyboardLayout,
    pub nostalgie: Option<Preset>,
}

pub struct App {
    audio: AudioSystem,
    window: Option<Arc<Window>>,
    pixels: Option<Pixels<'static>>,
    shader: Option<ShaderRenderer>,

    color_mode: ColorMode,
    initial_width: u32,
    initial_height: u32,
    current_width: u32,
    current_height: u32,

    debug_mode: bool,
    paused: bool,
    f9_pressed_since: Option<Instant>,
    is_fast_forwarding: bool,

    has_player: bool,
    keyboard_translator: KeyboardTranslator,
    modifiers: ModifiersState,

    cmd_tx: Sender<EmulationCommand>,
    frame_rx: Receiver<EmulationFrame>,
    emu_err_rx: Receiver<EmulationError>,
    emu_thread: Option<JoinHandle<()>>,

    pub fatal_error: Option<anyhow::Error>,
}

impl App {
    pub fn new(
        machine_config: MachineConfig,
        video: VideoRenderer,
        audio: AudioSystem,
        config: AppConfig,
    ) -> Self {
        let color_mode = video.color_mode;
        let initial_width = video.width();
        let initial_height = video.height();

        let cpu_freq = MASTER_CLOCK_HZ as f64 / CPU_DIVIDER as f64;
        let frame_duration_secs = DEFAULT_FRAME_CYCLES as f64 / cpu_freq;
        let sync_lag_threshold =
            Duration::from_secs_f64(frame_duration_secs * MIDI_SYNC_LAG_FRAMES);

        let (midi_tx, midi_thread) = match config.midi_out {
            Some(midi_conn) => {
                let (tx, rx) = crossbeam_channel::unbounded::<MidiThreadMsg>();
                let handle = std::thread::Builder::new()
                    .name("midi".into())
                    .spawn(move || {
                        run_midi_thread(
                            midi_conn,
                            rx,
                            cpu_freq,
                            frame_duration_secs,
                            sync_lag_threshold,
                        )
                    })
                    .expect("Failed to spawn MIDI thread");
                (Some(tx), Some(handle))
            }
            None => (None, None),
        };

        let (cmd_tx, cmd_rx) = crossbeam_channel::unbounded::<EmulationCommand>();
        let (frame_tx, frame_rx) =
            crossbeam_channel::bounded::<EmulationFrame>(FRAME_CHANNEL_CAPACITY);
        let (emu_err_tx, emu_err_rx) = crossbeam_channel::bounded::<EmulationError>(1);

        let audio_tx = audio.tx.clone();
        let sample_rate = audio.sample_rate;
        let has_player = config.player.is_some();
        let recorder_opt = config.recorder;
        let player_opt = config.player;
        let keyboard_translator = KeyboardTranslator::new(config.keyboard_layout);

        let emu_thread = std::thread::Builder::new()
            .name("emulation".into())
            .spawn(move || {
                run_emulation(
                    machine_config,
                    video,
                    audio_tx,
                    sample_rate,
                    midi_tx,
                    midi_thread,
                    recorder_opt,
                    player_opt,
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
            shader: config
                .nostalgie
                .map(|preset| ShaderRenderer::new(initial_width, initial_height, preset)),
            color_mode,
            initial_width,
            initial_height,
            current_width: initial_width,
            current_height: initial_height,
            debug_mode: config.debug_mode,
            paused: false,
            f9_pressed_since: None,
            is_fast_forwarding: false,
            has_player,
            keyboard_translator,
            modifiers: ModifiersState::empty(),
            cmd_tx,
            frame_rx,
            emu_err_rx,
            emu_thread: Some(emu_thread),
            fatal_error: None,
        }
    }

    fn toggle_fullscreen(&self) {
        if let Some(window) = &self.window {
            let next = if window.fullscreen().is_some() {
                None
            } else {
                Some(Fullscreen::Borderless(None))
            };
            window.set_fullscreen(next);
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

        let size = LogicalSize::new(
            f64::from(width) * WINDOW_SCALE,
            f64::from(height) * WINDOW_SCALE,
        );

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

        if let Some(shader) = &mut self.shader {
            if let Some(pixels) = &self.pixels {
                shader.set_format(pixels.surface_texture_format());
            }
            let inner = window.inner_size();
            shader.set_surface_size(inner.width, inner.height);
            shader.set_virtual_resolution(width, height);
        }

        self.window = Some(window);
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::ModifiersChanged(modifiers) => {
                self.modifiers = modifiers.state();
            }
            WindowEvent::Resized(new_size) if new_size.width > 0 && new_size.height > 0 => {
                if let Some(pixels) = &mut self.pixels
                    && let Err(err) = pixels.resize_surface(new_size.width, new_size.height)
                {
                    self.fatal_error =
                        Some(anyhow::Error::new(err).context("Pixels resize surface failed"));
                    event_loop.exit();
                    return;
                }
                if let Some(shader) = &mut self.shader {
                    shader.set_surface_size(new_size.width, new_size.height);
                }
                if let Some(win) = &self.window {
                    win.request_redraw();
                }
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if let PhysicalKey::Code(key_code) = event.physical_key
                    && event.state == ElementState::Pressed
                    && !event.repeat
                {
                    let alt_enter = key_code == KeyCode::Enter && self.modifiers.alt_key();
                    if key_code == KeyCode::F11 || alt_enter {
                        self.toggle_fullscreen();
                        return;
                    }
                }
                if let PhysicalKey::Code(key_code) = event.physical_key
                    && !event.repeat
                {
                    let pressed = event.state == ElementState::Pressed;

                    match (key_code, pressed, self.debug_mode) {
                        (KeyCode::F7, true, _) => {
                            self.has_player = false;
                            let _ = self.cmd_tx.send(EmulationCommand::HardReset);
                        }
                        (KeyCode::F8, true, true) => {
                            self.paused = !self.paused;
                            let _ = self.cmd_tx.send(EmulationCommand::TogglePause);
                        }
                        (KeyCode::F9, true, true) if self.f9_pressed_since.is_none() => {
                            self.f9_pressed_since = Some(Instant::now());
                            if self.paused {
                                let _ = self.cmd_tx.send(EmulationCommand::StepFrame);
                            } else {
                                self.is_fast_forwarding = true;
                                let _ = self.cmd_tx.send(EmulationCommand::SetFastForward(true));
                            }
                        }
                        (KeyCode::F9, false, true) => {
                            self.f9_pressed_since = None;
                            if self.is_fast_forwarding {
                                self.is_fast_forwarding = false;
                                let _ = self.cmd_tx.send(EmulationCommand::SetFastForward(false));
                            }
                        }
                        (KeyCode::F10, true, true) => {
                            let _ = self.cmd_tx.send(EmulationCommand::DumpSnapshot);
                            let _ = self.cmd_tx.send(EmulationCommand::SaveReplay);
                        }
                        _ => {}
                    }
                }

                if !event.repeat && !self.has_player {
                    let actions = self.keyboard_translator.process_key(&event);
                    for (key, key_pressed) in actions {
                        let _ = self.cmd_tx.send(EmulationCommand::KeyEvent {
                            key,
                            pressed: key_pressed,
                        });
                    }
                }
            }
            WindowEvent::RedrawRequested => {
                let render_result = match (&mut self.shader, &self.pixels) {
                    (Some(shader), Some(pixels)) => {
                        pixels.render_with(|encoder, target, context| {
                            shader.render(encoder, target, context)
                        })
                    }
                    (None, Some(pixels)) => pixels.render(),
                    _ => Ok(()),
                };
                if let Err(err) = render_result {
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

        if let Some(since) = self.f9_pressed_since
            && self.paused
            && !self.is_fast_forwarding
            && since.elapsed() > Duration::from_millis(500)
        {
            self.is_fast_forwarding = true;
            let _ = self.cmd_tx.send(EmulationCommand::SetFastForward(true));
        }

        let mut latest_frame = None;
        while let Ok(frame) = self.frame_rx.try_recv() {
            latest_frame = Some(frame);
        }

        if let Some(frame) = latest_frame {
            let prev_width = self.current_width;
            let prev_height = self.current_height;
            let size_changed = frame.width != prev_width || frame.height != prev_height;

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

                if let Some(shader) = &mut self.shader {
                    shader.set_virtual_resolution(frame.width, frame.height);
                }
            }

            if let Some(pixels) = &mut self.pixels {
                pixels.frame_mut().copy_from_slice(&frame.buffer);
            }

            if let Some(window) = &self.window {
                if size_changed {
                    let current = window.inner_size().to_logical::<f64>(window.scale_factor());
                    let w = current.width * f64::from(frame.width) / f64::from(prev_width);
                    let h = current.height * f64::from(frame.height) / f64::from(prev_height);
                    let _ = window.request_inner_size(LogicalSize::new(w, h));
                }
                window.request_redraw();
            }
        }

        event_loop.set_control_flow(ControlFlow::Poll);
    }

    fn exiting(&mut self, _event_loop: &ActiveEventLoop) {
        let _ = self.cmd_tx.send(EmulationCommand::Quit);

        if let Some(handle) = self.emu_thread.take() {
            let _ = handle.join();
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn run_emulation(
    machine_config: MachineConfig,
    mut video: VideoRenderer,
    audio_tx: Sender<f32>,
    sample_rate: u32,
    midi_tx: Option<Sender<MidiThreadMsg>>,
    midi_thread: Option<JoinHandle<()>>,
    mut recorder_opt: Option<ReplayRecorder>,
    mut player_opt: Option<ReplayPlayer>,
    cmd_rx: Receiver<EmulationCommand>,
    frame_tx: Sender<EmulationFrame>,
    emu_err_tx: Sender<EmulationError>,
) {
    let mut midi_stream = midly::stream::MidiStream::new();

    let cpu_freq = MASTER_CLOCK_HZ / CPU_DIVIDER;
    let samples_per_frame = (sample_rate as u64 * DEFAULT_FRAME_CYCLES as u64) / cpu_freq as u64;
    let latency_samples =
        ((samples_per_frame * AUDIO_LATENCY_FRAMES_NUMER) / AUDIO_LATENCY_FRAMES_DENOM) as usize;

    let mut paused = false;
    let mut step_frame = false;
    let mut fast_forward = false;
    let mut ff_skip_counter = 0;

    let mut machine = machine_config.new_machine();

    let save_replay = |rec: &Option<ReplayRecorder>| {
        if let Some(r) = rec
            && !r.is_empty()
        {
            let _ = r.save(&format!("{}.json", machine_config.rom_name));
        }
    };

    'run: loop {
        loop {
            let cmd = if paused && !step_frame && !fast_forward {
                match cmd_rx.recv() {
                    Ok(cmd) => cmd,
                    Err(_) => break 'run,
                }
            } else {
                match cmd_rx.try_recv() {
                    Ok(cmd) => cmd,
                    Err(crossbeam_channel::TryRecvError::Empty) => break,
                    Err(crossbeam_channel::TryRecvError::Disconnected) => break 'run,
                }
            };

            match cmd {
                EmulationCommand::KeyEvent { key, pressed } => {
                    machine.update_key(key, pressed);
                    if let Some(rec) = &mut recorder_opt {
                        rec.push_key(machine.cycle_count(), key, pressed);
                    }
                }
                EmulationCommand::TogglePause => {
                    paused = !paused;
                }
                EmulationCommand::StepFrame => {
                    step_frame = true;
                }
                EmulationCommand::SetFastForward(ff) => {
                    fast_forward = ff;
                    if let Some(tx) = &midi_tx {
                        let _ = tx.send(MidiThreadMsg::SetFastForward(ff));
                    }
                }
                EmulationCommand::HardReset => {
                    save_replay(&recorder_opt);
                    recorder_opt = None;
                    player_opt = None;
                    machine = machine_config.new_machine();
                    midi_stream = midly::stream::MidiStream::new();
                    if let Some(tx) = &midi_tx {
                        let _ = tx.send(MidiThreadMsg::HardReset);
                    }
                }
                EmulationCommand::DumpSnapshot => {
                    let frame = machine.cycle_count();
                    let snap_name = format!("{}_frame_{}", machine_config.rom_name, frame);
                    dump_snapshot(&machine, &video, &snap_name);

                    if let Some(rec) = &mut recorder_opt {
                        rec.push_snapshot(frame, snap_name);
                    }
                }
                EmulationCommand::SaveReplay => {
                    save_replay(&recorder_opt);
                }
                EmulationCommand::Quit => {
                    save_replay(&recorder_opt);
                    break 'run;
                }
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
                    &mut player_opt,
                    false,
                ) {
                    Ok(v) => vblank_occurred = v,
                    Err(()) => {
                        let _ = emu_err_tx.send(EmulationError::AudioDisconnected);
                        break 'run;
                    }
                }
            }
            video.render_frame(machine.vg75(), machine.font_banks());
            send_frame(&video, &frame_tx);
            step_frame = false;
            continue;
        }

        if fast_forward {
            match emu_cycle(
                &mut machine,
                &audio_tx,
                &midi_tx,
                &mut midi_stream,
                &mut player_opt,
                true,
            ) {
                Ok(vblank) => {
                    if vblank {
                        ff_skip_counter += 1;
                        if ff_skip_counter >= 5 {
                            ff_skip_counter = 0;
                            if !frame_tx.is_full() {
                                video.render_frame(machine.vg75(), machine.font_banks());
                                send_frame(&video, &frame_tx);
                            }
                        }
                    }
                }
                Err(()) => {
                    let _ = emu_err_tx.send(EmulationError::AudioDisconnected);
                    break 'run;
                }
            }
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
                &mut player_opt,
                false,
            ) {
                Ok(vblank_occurred) => {
                    if vblank_occurred {
                        video.render_frame(machine.vg75(), machine.font_banks());
                        send_frame(&video, &frame_tx);
                    }
                }
                Err(()) => {
                    let _ = emu_err_tx.send(EmulationError::AudioDisconnected);
                    break 'run;
                }
            }
        }
    }

    drop(midi_tx);
    if let Some(handle) = midi_thread {
        let _ = handle.join();
    }
}

#[inline]
fn emu_cycle(
    machine: &mut Machine,
    audio_tx: &Sender<f32>,
    midi_tx: &Option<Sender<MidiThreadMsg>>,
    midi_stream: &mut midly::stream::MidiStream,
    player: &mut Option<ReplayPlayer>,
    fast_forward: bool,
) -> Result<bool, ()> {
    if let Some(player) = player {
        let _ = player.apply_pending_events(machine);
    }

    let mut audio_alive = true;
    let vblank_occurred = machine.tick(|sample| {
        if !fast_forward {
            match audio_tx.try_send(sample) {
                Err(crossbeam_channel::TrySendError::Disconnected(_)) => {
                    audio_alive = false;
                }
                Err(crossbeam_channel::TrySendError::Full(_)) => {}
                Ok(_) => {}
            }
        }
    });

    if let Some(tx) = midi_tx {
        machine.drain_midi_out(|events| {
            for &(byte, cycle) in events {
                midi_stream.feed(&[byte], |live_event| {
                    let mut buf = Vec::with_capacity(3);
                    if live_event.write_std(&mut buf).is_ok() {
                        let _ = tx.send(MidiThreadMsg::Event(buf, cycle));
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

fn track_note(msg: &[u8], active_notes: &mut [u128; MIDI_CHANNELS_COUNT as usize]) {
    if msg.len() >= MIDI_VOICE_MSG_LEN {
        let status = msg[0] & MIDI_STATUS_MASK;
        let ch = (msg[0] & MIDI_CHANNEL_MASK) as usize;
        let note = msg[1] & MIDI_NOTE_MASK;

        match status {
            MIDI_STATUS_NOTE_ON if msg[2] > 0 => active_notes[ch] |= 1u128 << note,
            MIDI_STATUS_NOTE_OFF | MIDI_STATUS_NOTE_ON => active_notes[ch] &= !(1u128 << note),
            _ => {}
        }
    }
}

fn silence_active_notes(
    active_notes: &mut [u128; MIDI_CHANNELS_COUNT as usize],
    midi_conn: &mut MidiConn,
) {
    for ch in 0..MIDI_CHANNELS_COUNT {
        let mut bits = active_notes[ch as usize];
        while bits != 0 {
            let note = bits.trailing_zeros() as u8;
            midi_conn.send_now(&[MIDI_STATUS_NOTE_OFF | ch, note, 0]);
            bits &= !(1u128 << note);
        }
        active_notes[ch as usize] = 0;
        midi_conn.send_now(&[MIDI_STATUS_CONTROL_CHANGE | ch, MIDI_CC_ALL_NOTES_OFF, 0]);
        midi_conn.send_now(&[MIDI_STATUS_CONTROL_CHANGE | ch, MIDI_CC_ALL_SOUND_OFF, 0]);
    }
}
