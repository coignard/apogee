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

use apogee_rs::core::debug::ReplayPlayer;
use apogee_rs::core::machine::Machine;
use apogee_rs::core::video::VideoRenderer;
use assert_json_diff::assert_json_eq;
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};
use test_generator::test_resources;

const SYSTEM_ROM: &[u8] = include_bytes!("../dist/roms/apogee.rom");
const FONT_ROM: &[u8] = include_bytes!("../dist/fonts/sga.bin");

#[test_resources("tests/replays/*.json")]
fn replay_matches_snapshot(replay_path_str: &str) {
    let mut player = ReplayPlayer::from_file(replay_path_str)
        .unwrap_or_else(|_| panic!("Failed to parse replay JSON: {:?}", replay_path_str));

    let base_name = Path::new(&player.replay.metadata.rom_name)
        .file_stem()
        .expect("rom_name should have a valid stem")
        .to_string_lossy()
        .into_owned();

    let sample_rate = player.replay.metadata.sample_rate;
    let autorun = player.replay.metadata.autorun;
    let color_mode = player.replay.metadata.color_mode;
    let is_crt = player.replay.metadata.is_crt;

    let mut machine = Machine::new(SYSTEM_ROM.to_vec(), sample_rate);
    let mut video = VideoRenderer::new(FONT_ROM.to_vec(), color_mode, is_crt);

    if base_name != "monitor" {
        let rka_path = PathBuf::from("tests/assets")
            .join(&base_name)
            .with_extension("rka");

        let rka_data =
            fs::read(&rka_path).unwrap_or_else(|_| panic!("Failed to read ROM at {:?}", rka_path));

        let loaded_hash = hex::encode(Sha256::digest(&rka_data));

        assert_eq!(
            loaded_hash, player.replay.metadata.rom_sha256,
            "rom hash mismatch for '{}'",
            base_name
        );

        machine
            .load_rka(&rka_data, autorun, true)
            .expect("Failed to load RKA");
    }

    let update_snapshots = std::env::var("UPDATE_SNAPSHOTS").is_ok();

    while !player.is_finished() {
        let snapshots = player.apply_pending_events(&mut machine);

        for snap in snapshots {
            let json_path = format!("tests/dumps/{}/{}.json", base_name, snap);
            let state = machine.state();

            if update_snapshots {
                let file = fs::File::create(&json_path)
                    .unwrap_or_else(|_| panic!("Failed to overwrite JSON: {}", json_path));
                serde_json::to_writer_pretty(file, &state).expect("Failed to write updated JSON");
            } else {
                let expected_json_file = fs::File::open(&json_path)
                    .unwrap_or_else(|_| panic!("Failed to open expected JSON: {}", json_path));

                let expected_state: serde_json::Value = serde_json::from_reader(expected_json_file)
                    .expect("Failed to parse expected JSON");

                let actual_state =
                    serde_json::to_value(&state).expect("Failed to serialize machine state");

                assert_json_eq!(expected_state, actual_state);
            }

            let png_path = format!("tests/dumps/{}/{}.png", base_name, snap);
            let expected_img = image::open(&png_path)
                .unwrap_or_else(|_| panic!("Failed to open expected PNG: {}", png_path))
                .into_rgba8();

            let expected_pixels = expected_img.as_raw();
            let actual_pixels = video.frame_buffer();

            assert_eq!(
                actual_pixels.len(),
                expected_pixels.len(),
                "framebuffer size mismatch at snapshot '{}'",
                snap
            );

            if actual_pixels != expected_pixels {
                let mut diffs = 0;
                let mut first_diff = None;

                for (i, (&a, &e)) in actual_pixels.iter().zip(expected_pixels.iter()).enumerate() {
                    if a != e {
                        diffs += 1;
                        if first_diff.is_none() {
                            first_diff = Some((i, a, e));
                        }
                    }
                }

                if let Some((idx, act, exp)) = first_diff {
                    panic!(
                        "pixel mismatch at snapshot '{}' (ROM: {}): {} bytes differ, first at [{}]: actual={}, expected={}",
                        snap, base_name, diffs, idx, act, exp
                    );
                }
            }
        }

        let vblank_occurred = machine.tick(|_| {});

        if vblank_occurred {
            video.render_frame(machine.vg75());
        }
    }
}
