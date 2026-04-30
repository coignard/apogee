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
use clap::Parser;
use sha2::{Digest, Sha256};
use winit::event_loop::EventLoop;

use crate::app::App;
use crate::app::audio::AudioSystem;

use apogee_rs::core::machine::Machine;
use apogee_rs::core::video::{ColorMode, VideoRenderer};

const SYSTEM_ROM: &[u8] = include_bytes!("../dist/roms/apogee.rom");
const FONT_ROM: &[u8] = include_bytes!("../dist/fonts/sga.bin");

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

    verify(
        "apogee.rom",
        SYSTEM_ROM,
        "4b5c8507ff16f7712e28e0f635fd783f2a8ba7c912f82d86223a90f3656a2395",
    )?;

    verify(
        "sga.bin",
        FONT_ROM,
        "a71d0166f73952675db15088545276cf39805cab34f9c94f28de50931f8ed99f",
    )?;

    Ok(())
}

fn main() -> Result<()> {
    check_integrity()?;

    let args = Args::parse();

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

    if let Some(path) = &args.file {
        let data = fs::read(path).with_context(|| format!("could not read '{}'", path))?;
        let is_rka = path.to_lowercase().ends_with(".rka");

        machine
            .load_rom(&data, is_rka, args.autorun, args.force)
            .with_context(|| {
                if is_rka {
                    format!("invalid RKA file '{}'", path)
                } else {
                    format!("invalid ROM disk file '{}'", path)
                }
            })?;
    }

    let mut app = App::new(machine, video, audio);

    event_loop
        .run_app(&mut app)
        .context("Application execution failed")?;

    if let Some(err) = app.fatal_error {
        return Err(err);
    }

    Ok(())
}
