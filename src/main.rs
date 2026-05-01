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

use crate::app::App;
use crate::app::audio::AudioSystem;

use apogee_rs::core::debug::{ReplayMetadata, ReplayPlayer, ReplayRecorder};
use apogee_rs::core::machine::Machine;
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
    /// Path to a program image (.rka) to load at startup
    #[arg(value_name = "file", hide = true)]
    file: Option<String>,

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

    let mut rom_sha256 = String::from(SYSTEM_ROM_HASH);
    let mut rom_name = String::from("monitor");
    let mut rom_data_to_load = None;

    if let Some(path) = &args.file {
        let path_obj = std::path::Path::new(path);
        let ext = path_obj.extension();

        let is_rka = ext.is_some_and(|e| e.eq_ignore_ascii_case("rka"));
        let is_rom = ext.is_some_and(|e| e.eq_ignore_ascii_case("rom"));

        ensure!(
            is_rka || is_rom,
            "unsupported file extension for '{}': only .rka and .rom are allowed",
            path
        );

        let data = fs::read(path).with_context(|| format!("could not read '{}'", path))?;

        if is_rka {
            Machine::validate_rka(&data, args.force)
                .with_context(|| format!("invalid RKA file '{}'", path))?;
        }

        rom_sha256 = hex::encode(Sha256::digest(&data));
        rom_name = path_obj
            .file_stem()
            .unwrap_or(std::ffi::OsStr::new("unknown"))
            .to_string_lossy()
            .into_owned();

        rom_data_to_load = Some((data, is_rka));
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

    if let Some((data, is_rka)) = rom_data_to_load {
        machine
            .load_rom(&data, is_rka, args.autorun, args.force)
            .context("Unexpected error occured while loading ROM data into memory")?;
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
        machine, video, audio, args.debug, recorder, player, rom_name,
    );

    event_loop
        .run_app(&mut app)
        .context("Application execution failed")?;

    if let Some(err) = app.fatal_error.take() {
        return Err(err);
    }

    Ok(())
}
