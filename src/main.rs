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
use crate::app::{App, AppConfig};

use apogee_rs::core::debug::{ReplayMetadata, ReplayPlayer, ReplayRecorder};
use apogee_rs::core::machine::Machine;
use apogee_rs::core::peripherals::UserPeripheral;
use apogee_rs::core::peripherals::midi::MidiInterface;
use apogee_rs::core::video::{ColorMode, VideoRenderer};

const SYSTEM_ROM: &[u8] = include_bytes!("../dist/roms/apogee.rom");
const FONT_ROM: &[u8] = include_bytes!("../dist/fonts/sga.bin");

const SYSTEM_ROM_HASH: &str = include_str!("../dist/roms/apogee.rom.sha256").trim_ascii();
const FONT_ROM_HASH: &str = include_str!("../dist/fonts/sga.bin.sha256").trim_ascii();

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

    verify("apogee.rom", SYSTEM_ROM, SYSTEM_ROM_HASH)?;
    verify("sga.bin", FONT_ROM, FONT_ROM_HASH)?;

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
    #[arg(short, long, conflicts_with = "bw", help_heading = "Display options")]
    grayscale: bool,

    /// Blend consecutive frames to simulate CRT
    #[arg(short, long, help_heading = "Display options")]
    crt: bool,

    /// Connect to a MIDI output port by name or index
    #[arg(long, num_args = 0..=1, default_missing_value = "", help_heading = "MIDI options")]
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
    #[arg(long, conflicts_with = "record", help_heading = "Debug options")]
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
        if let Ok(midi_out) = midir::MidiOutput::new("Apogee BK-01") {
            for (i, port) in midi_out.ports().iter().enumerate() {
                if let Ok(name) = midi_out.port_name(port) {
                    println!("{}: {}", i, name);
                }
            }
        }
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
        "a ROM disk cannot be plugged in simultaneously with the MIDI interface"
    );

    let mut rom_sha256 = String::from(SYSTEM_ROM_HASH);
    let mut rom_name = String::from("monitor");

    if let Some(path) = &rka_path {
        let rka_data = fs::read(path).with_context(|| format!("could not read '{}'", path))?;
        Machine::validate_rka(&rka_data, args.force)
            .with_context(|| format!("invalid RKA file '{}'", path))?;

        rom_sha256 = hex::encode(Sha256::digest(&rka_data));
        rom_name = std::path::Path::new(path)
            .file_stem()
            .unwrap_or(std::ffi::OsStr::new("unknown"))
            .to_string_lossy()
            .into_owned();
    }

    let event_loop = EventLoop::new().context("Failed to create winit event loop")?;

    let color_mode = if args.bw {
        ColorMode::Bw
    } else if args.grayscale {
        ColorMode::Grayscale
    } else {
        ColorMode::Color
    };

    let audio = AudioSystem::new().context("Failed to initialize audio system")?;
    let mut machine = Machine::new(SYSTEM_ROM.to_vec(), audio.sample_rate);
    let video = VideoRenderer::new(FONT_ROM.to_vec(), color_mode, args.crt);

    if let Some(path) = &rka_path {
        let rka_data = fs::read(path).unwrap();
        machine
            .load_rka(&rka_data, args.autorun, args.force)
            .context("Unexpected error occured while loading RKA data into memory")?;
    }

    let mut midi_conn = None;

    if let Some(rom_path_resolved) = &rom_path {
        let data = fs::read(rom_path_resolved)
            .with_context(|| format!("could not read '{}'", rom_path_resolved))?;
        let mut romdisk = apogee_rs::core::peripherals::romdisk::RomDisk::new();
        romdisk.load(&data);
        machine.plug_user_peripheral(UserPeripheral::RomDisk(romdisk));
    } else if let Some(midi_arg) = &args.midi {
        if let Ok(midi_out) = midir::MidiOutput::new("Apogee BK-01") {
            let ports = midi_out.ports();

            let target_port = if midi_arg.is_empty() {
                ports.first()
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
            };

            if let Some(port) = target_port {
                let conn_name = midi_out
                    .port_name(port)
                    .unwrap_or_else(|_| "Apogee BK-01 MIDI Out".to_string());
                midi_conn = midi_out.connect(port, &conn_name).ok();
            } else {
                #[cfg(unix)]
                {
                    use midir::os::unix::VirtualOutput;
                    midi_conn = midi_out.create_virtual(midi_arg).ok();
                }
            }
        }

        machine.plug_user_peripheral(UserPeripheral::Midi(MidiInterface::new()));
    }

    let recorder = args.record.then(|| {
        ReplayRecorder::new(ReplayMetadata {
            rom_name: rom_name.clone(),
            rom_sha256: rom_sha256.clone(),
            autorun: args.autorun,
            sample_rate: audio.sample_rate,
            color_mode,
            is_crt: args.crt,
        })
    });

    let player = if let Some(path) = &args.play {
        let player = ReplayPlayer::from_file(path)?;
        ensure!(
            player.replay.metadata.rom_sha256 == rom_sha256,
            "replay SHA256 '{}' does not match loaded ROM '{}'",
            player.replay.metadata.rom_sha256,
            rom_sha256
        );
        Some(player)
    } else {
        None
    };

    let mut app = App::new(
        machine,
        video,
        audio,
        AppConfig {
            debug_mode: args.debug,
            recorder,
            player,
            rom_name,
            midi_out: midi_conn,
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
