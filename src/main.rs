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

mod app;

use std::fs;

use anyhow::{Context, Result, ensure};
use clap::{CommandFactory, FromArgMatches, Parser};
use sha2::{Digest, Sha256};
use winit::event_loop::EventLoop;

use crate::app::audio::AudioSystem;
use crate::app::keyboard::KeyboardLayout;
use crate::app::shaders::Preset;
use crate::app::{App, AppConfig, MachineConfig, MidiConn};

use apogee_rs::core::debug::{ReplayMetadata, ReplayPlayer, ReplayRecorder};
use apogee_rs::core::machine::Machine;
use apogee_rs::core::video::{ColorMode, VideoRenderer};

const MONITOR_ROM: &[u8] = include_bytes!("../firmware/monitor.rom");
const CHARGEN_ROM: &[u8] = include_bytes!("../firmware/chargen.rom");

const MONITOR_ROM_HASH: &str = include_str!("../firmware/monitor.rom.sha256").trim_ascii();
const CHARGEN_ROM_HASH: &str = include_str!("../firmware/chargen.rom.sha256").trim_ascii();

#[cfg(not(target_os = "macos"))]
fn list_midi_outputs(client_name: &str) {
    if let Ok(midi_out) = midir::MidiOutput::new(client_name) {
        for (i, port) in midi_out.ports().iter().enumerate() {
            if let Ok(name) = midi_out.port_name(port) {
                println!("{}: {}", i, name);
            }
        }
    }
}

#[cfg(not(target_os = "macos"))]
fn open_midi_output(client_name: &str, midi_arg: &str) -> Option<MidiConn> {
    let midi_out = midir::MidiOutput::new(client_name).ok()?;
    let ports = midi_out.ports();
    let target_port = if midi_arg.is_empty() {
        ports.first().cloned()
    } else {
        ports
            .iter()
            .find(|p| midi_out.port_name(p).is_ok_and(|name| name == *midi_arg))
            .or_else(|| {
                midi_arg
                    .parse::<usize>()
                    .ok()
                    .and_then(|idx| ports.get(idx))
            })
            .cloned()
    };

    if let Some(port) = target_port {
        let conn_name = midi_out
            .port_name(&port)
            .unwrap_or_else(|_| format!("{} MIDI Out", client_name));
        midi_out.connect(&port, &conn_name).ok().map(MidiConn::new)
    } else {
        #[cfg(unix)]
        {
            use midir::os::unix::VirtualOutput;
            midi_out.create_virtual(midi_arg).ok().map(MidiConn::new)
        }
        #[cfg(not(unix))]
        {
            None
        }
    }
}

#[cfg(target_os = "macos")]
fn list_midi_outputs(_client_name: &str) {
    for i in 0..coremidi::Destinations::count() {
        if let Some(dest) = coremidi::Destination::from_index(i) {
            let name = dest
                .display_name()
                .unwrap_or_else(|| String::from("Unknown Device"));
            println!("{}: {}", i, name);
        }
    }
}

#[cfg(target_os = "macos")]
fn open_midi_output(client_name: &str, midi_arg: &str) -> Option<MidiConn> {
    let count = coremidi::Destinations::count();
    if count == 0 {
        return None;
    }

    let index = if midi_arg.is_empty() {
        Some(0)
    } else {
        (0..count)
            .find(|&i| {
                coremidi::Destination::from_index(i)
                    .and_then(|d| d.display_name())
                    .is_some_and(|name| name == *midi_arg)
            })
            .or_else(|| midi_arg.parse::<usize>().ok().filter(|&idx| idx < count))
    };

    let dest = coremidi::Destination::from_index(index?)?;
    let client = coremidi::Client::new(client_name).ok()?;
    let port = client
        .output_port(&format!("{} MIDI Out", client_name))
        .ok()?;
    Some(MidiConn::new(port, dest))
}

fn check_integrity() -> Result<()> {
    let verify = |name: &str, data: &[u8], expected: &str| -> Result<()> {
        let hash = Sha256::digest(data);
        let actual = hex::encode(hash);
        ensure!(
            actual == expected,
            "integrity check failed for asset '{}'",
            name
        );
        Ok(())
    };

    verify("monitor.rom", MONITOR_ROM, MONITOR_ROM_HASH)?;
    verify("chargen.rom", CHARGEN_ROM, CHARGEN_ROM_HASH)?;

    Ok(())
}

#[derive(Parser, Debug)]
#[command(
    name = "apogee",
    version,
    override_usage = "apogee [options] [file]",
    disable_help_flag = true,
    disable_version_flag = true,
    next_line_help = true,
    help_template = "Usage: {usage}\n\n{all-args}"
)]
struct Args {
    #[arg(value_name = "file", hide = true)]
    file: Option<String>,

    /// Path to a program image (.rka) to load into RAM at startup
    #[arg(long, value_name = "file", help_heading = "General options")]
    rka: Option<String>,

    /// Path to a ROM disk image (.rom) to plug into the user port
    #[arg(long, value_name = "file", help_heading = "General options")]
    rom: Option<String>,

    /// Run the loaded program immediately after startup
    #[arg(short = 'a', long = "autorun", help_heading = "General options")]
    autorun: bool,

    /// Skip validation and load anyway
    #[arg(short = 'f', long = "force", help_heading = "General options")]
    force: bool,

    /// Select keyboard layout
    /// Default: smart
    /// Possible values: smart, qwerty, jcuken
    #[arg(
        long,
        value_name = "layout",
        value_enum,
        default_value_t = KeyboardLayout::Smart,
        hide_default_value = true,
        hide_possible_values = true,
        help_heading = "Keyboard options",
        verbatim_doc_comment
    )]
    keyboard_layout: KeyboardLayout,

    /// Print this message and exit
    #[arg(
        short = 'h',
        long = "help",
        action = clap::ArgAction::Help,
        help_heading = "General options"
    )]
    help: Option<bool>,

    /// Print version information and exit
    #[arg(
        short = 'V',
        long = "version",
        action = clap::ArgAction::Version,
        help_heading = "General options"
    )]
    version: Option<bool>,

    /// Use a black-and-white display with no shading
    #[arg(long, conflicts_with = "grayscale", help_heading = "Display options")]
    bw: bool,

    /// Use a grayscale display with luminance shading
    #[arg(long, conflicts_with = "bw", help_heading = "Display options")]
    grayscale: bool,

    /// Blend consecutive frames to simulate gigascreen
    #[arg(long, help_heading = "Display options")]
    gigascreen: bool,

    /// Enable the CRT shader
    /// Default: built-in profile
    #[arg(
        long,
        value_name = "preset",
        num_args = 0..=1,
        require_equals = true,
        default_missing_value = "@default",
        help_heading = "Display options",
        verbatim_doc_comment
    )]
    nostalgie: Option<String>,

    /// Connect to a MIDI output port by name or index
    /// Default: first available port
    #[arg(
        long,
        value_name = "port",
        num_args = 0..=1,
        default_missing_value = "",
        hide_default_value = true,
        help_heading = "MIDI options",
        verbatim_doc_comment
    )]
    midi: Option<String>,

    /// List available MIDI output ports and exit
    #[arg(long, help_heading = "MIDI options")]
    midi_list: bool,

    /// Enable debug hotkeys
    #[arg(long, help_heading = "Debug options")]
    debug: bool,

    /// Enable replay recording mode
    #[arg(long, requires = "debug", help_heading = "Debug options")]
    record: bool,

    /// Play a recorded replay from a file
    #[arg(
        long,
        value_name = "file",
        conflicts_with = "record",
        help_heading = "Debug options"
    )]
    play: Option<String>,
}

fn main() -> Result<()> {
    check_integrity()?;

    let mut cmd = Args::command();

    if !std::env::args_os().any(|arg| arg == "--debug") {
        cmd = cmd
            .mut_arg("debug", |a| a.hide(true))
            .mut_arg("record", |a| a.hide(true))
            .mut_arg("play", |a| a.hide(true));
    }

    let matches = cmd.get_matches();
    let args = Args::from_arg_matches(&matches).unwrap_or_else(|e| e.exit());

    if args.midi_list {
        list_midi_outputs("Apogee BK-01");
        return Ok(());
    }

    let (rka_path, rom_path) = match args.file {
        Some(file) => {
            let path = std::path::Path::new(&file);
            match path
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.to_ascii_lowercase())
                .as_deref()
            {
                Some("rka") => (args.rka.or(Some(file)), args.rom),
                Some("rom") => (args.rka, args.rom.or(Some(file))),
                _ => anyhow::bail!(
                    "unsupported file extension for '{}': only .rka and .rom are allowed",
                    file
                ),
            }
        }
        None => (args.rka, args.rom),
    };

    ensure!(
        rom_path.is_none() || args.midi.is_none(),
        "a ROM disk cannot be plugged in simultaneously with the MIDI"
    );

    let (rka_data, rom_sha256, rom_name) = if let Some(path) = &rka_path {
        let data = fs::read(path).with_context(|| format!("could not read '{}'", path))?;
        Machine::validate_rka(&data, args.force)
            .with_context(|| format!("invalid RKA file '{}'", path))?;

        let sha256 = hex::encode(Sha256::digest(&data));
        let name = std::path::Path::new(path)
            .file_stem()
            .unwrap_or(std::ffi::OsStr::new("unknown"))
            .to_string_lossy()
            .into_owned();

        (Some(data), sha256, name)
    } else {
        (
            None,
            String::from(MONITOR_ROM_HASH),
            String::from("monitor"),
        )
    };

    let rom_payload = if let Some(rom_path) = &rom_path {
        let data = fs::read(rom_path).with_context(|| format!("could not read '{}'", rom_path))?;
        Some(std::sync::Arc::from(data))
    } else {
        None
    };

    let player = if let Some(path) = &args.play {
        let player = ReplayPlayer::from_file(path)?;
        player.verify_rom_hash(&rom_sha256)?;
        Some(player)
    } else {
        None
    };

    let autorun = player
        .as_ref()
        .map(|p| p.replay.metadata.autorun)
        .unwrap_or(args.autorun);

    let color_mode = player
        .as_ref()
        .map(|p| p.replay.metadata.color_mode)
        .unwrap_or_else(|| {
            if args.bw {
                ColorMode::Bw
            } else if args.grayscale {
                ColorMode::Grayscale
            } else {
                ColorMode::Color
            }
        });

    let gigascreen = player
        .as_ref()
        .map(|p| p.replay.metadata.gigascreen)
        .unwrap_or(args.gigascreen);

    let rka_payload = rka_data.map(|data| (std::sync::Arc::from(data), autorun, args.force));

    let event_loop = EventLoop::new().context("Failed to create winit event loop")?;

    let audio = AudioSystem::new().context("Failed to initialize audio system")?;
    let video = VideoRenderer::new(CHARGEN_ROM.to_vec(), color_mode, gigascreen);

    let sample_rate = player
        .as_ref()
        .map(|p| p.replay.metadata.sample_rate)
        .unwrap_or(audio.sample_rate);

    let midi_conn = if rom_payload.is_none()
        && let Some(midi_arg) = &args.midi
    {
        open_midi_output("Apogee BK-01", midi_arg)
    } else {
        None
    };

    let machine_config = MachineConfig {
        monitor_rom: std::sync::Arc::from(MONITOR_ROM),
        sample_rate,
        rka: rka_payload,
        romdisk: rom_payload,
        midi_enabled: midi_conn.is_some() || args.midi.is_some(),
        rom_name: rom_name.clone(),
    };

    let recorder = args.record.then(|| {
        ReplayRecorder::new(ReplayMetadata {
            rom_name,
            rom_sha256,
            autorun,
            sample_rate,
            color_mode,
            gigascreen,
        })
    });

    let nostalgie = match args.nostalgie.as_deref() {
        None => None,
        Some("@default") => Some(Preset::default_preset()),
        Some(path) => {
            let json = std::fs::read_to_string(path)
                .with_context(|| format!("Failed to read nostalgie preset '{path}'"))?;
            let preset = Preset::from_json(&json)
                .with_context(|| format!("Failed to parse nostalgie preset '{path}'"))?;
            Some(preset)
        }
    };

    let mut app = App::new(
        machine_config,
        video,
        audio,
        AppConfig {
            debug_mode: args.debug,
            recorder,
            player,
            midi_out: midi_conn,
            keyboard_layout: args.keyboard_layout,
            nostalgie,
        },
    );

    event_loop
        .run_app(&mut app)
        .context("Application execution failed")?;

    if let Some(err) = app.fatal_error.take() {
        return Err(err);
    }

    Ok(())
}
