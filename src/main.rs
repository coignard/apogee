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
mod core;

use std::fs;

use clap::Parser;
use winit::event_loop::EventLoop;

use crate::app::App;
use crate::app::video::{ColorMode, VideoRenderer};
use crate::core::machine::ApogeeMachine;

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

fn main() {
    let args = Args::parse();

    let color_mode = if args.bw {
        ColorMode::Bw
    } else if args.grayscale {
        ColorMode::Grayscale
    } else {
        ColorMode::Color
    };

    let mut machine = ApogeeMachine::new(SYSTEM_ROM.to_vec());
    let video = VideoRenderer::new(FONT_ROM.to_vec(), color_mode, args.crt);

    if let Some(path) = &args.file {
        match fs::read(path) {
            Ok(data) => {
                if let Err(err) = machine.load_rom(&data, true, args.autorun) {
                    eprintln!("error: invalid RKA file '{}': {}", path, err);
                    std::process::exit(1);
                }
            }
            Err(err) => {
                eprintln!("error: could not read '{}': {}", path, err);
                std::process::exit(1);
            }
        }
    }

    let mut app = App::new(machine, video);
    let event_loop = EventLoop::new().unwrap();
    let _ = event_loop.run_app(&mut app);
}
