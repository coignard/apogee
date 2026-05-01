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

use crate::core::chips::kr580vg75::Kr580Vg75;
use serde::{Deserialize, Serialize};

pub const CHAR_WIDTH: usize = 6;
const DEFAULT_CHARS_PER_ROW: usize = 78;
const DEFAULT_ROWS_PER_SCREEN: usize = 30;
const DEFAULT_LINES_PER_ROW: usize = 10;
const FONT_ALT_BANK_OFFSET: usize = 128;

#[derive(Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ColorMode {
    Color,
    Grayscale,
    Bw,
}

const GRAYSCALE_PALETTE: [[u8; 4]; 8] = [
    [0x00, 0x00, 0x00, 255],
    [0x82, 0x82, 0x82, 255],
    [0xC5, 0xC5, 0xC5, 255],
    [0xEE, 0xEE, 0xEE, 255],
    [0x58, 0x58, 0x58, 255],
    [0xAE, 0xAE, 0xAE, 255],
    [0xDF, 0xDF, 0xDF, 255],
    [0xFF, 0xFF, 0xFF, 255],
];

pub struct VideoRenderer {
    pub font_rom: Vec<u8>,
    pub color_mode: ColorMode,
    is_crt_blend: bool,
    width: u32,
    height: u32,
    frame_buffer: Vec<u8>,
    prev_frame_buffer: Vec<u8>,
}

impl VideoRenderer {
    pub fn new(font_rom: Vec<u8>, color_mode: ColorMode, is_crt_blend: bool) -> Self {
        let width = (DEFAULT_CHARS_PER_ROW * CHAR_WIDTH) as u32;
        let height = (DEFAULT_ROWS_PER_SCREEN * DEFAULT_LINES_PER_ROW) as u32;

        Self {
            font_rom,
            color_mode,
            is_crt_blend,
            width,
            height,
            frame_buffer: vec![0; (width * height * 4) as usize],
            prev_frame_buffer: vec![0; (width * height * 4) as usize],
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.height
    }

    #[inline]
    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    pub fn render_frame(&mut self, vg75: &Kr580Vg75) -> bool {
        let parsed_frame = vg75.parsed_frame();
        let max_rows = parsed_frame.len();
        let max_chars = parsed_frame[0].len();

        let n_rows = (vg75.n_rows() as usize).min(max_rows);
        let n_chars = (vg75.n_chars() as usize).min(max_chars);
        let n_lines = vg75.n_lines() as usize;

        let new_width = ((n_chars * CHAR_WIDTH) as u32).max(CHAR_WIDTH as u32);
        let new_height = ((n_rows * n_lines) as u32).max(1);

        let mut size_changed = false;

        if new_width != self.width || new_height != self.height {
            self.width = new_width;
            self.height = new_height;
            let buf_size = (self.width * self.height * 4) as usize;

            self.frame_buffer.resize(buf_size, 0);
            self.prev_frame_buffer.resize(buf_size, 0);

            size_changed = true;
        }

        let bg_color = [0, 0, 0, 255];

        for px in self.frame_buffer.chunks_exact_mut(4) {
            px.copy_from_slice(&bg_color);
        }

        for (row, frame_row) in parsed_frame.iter().enumerate().take(n_rows) {
            let chargen = if vg75.row_font_bank(row % 64) {
                FONT_ALT_BANK_OFFSET
            } else {
                0
            };

            for ln in 0..n_lines {
                let lc = if vg75.font_down() {
                    if ln != 0 {
                        ln - 1
                    } else {
                        n_lines.saturating_sub(1)
                    }
                } else {
                    ln
                };

                let py = row * n_lines + ln;
                if py >= self.height as usize {
                    continue;
                }

                let px_base_y = py * (self.width as usize);

                for x in 0..n_chars {
                    let sym = &frame_row[x];
                    let vsp = sym.get_vsp(ln);
                    let lten = sym.get_lten(ln);

                    let is_bw = self.color_mode == ColorMode::Bw;
                    let attr_sym = if is_bw && x < n_chars - 1 {
                        &frame_row[x + 1]
                    } else {
                        sym
                    };

                    let hglt = attr_sym.hglt();
                    let gpa0 = attr_sym.gpa0();
                    let gpa1 = attr_sym.gpa1();
                    let rvv = attr_sym.rvv();

                    let fg_color = match self.color_mode {
                        ColorMode::Grayscale => {
                            let mut index = 0;
                            if !hglt {
                                index |= 1;
                            }
                            if !gpa1 {
                                index |= 2;
                            }
                            if !gpa0 {
                                index |= 4;
                            }
                            GRAYSCALE_PALETTE[index]
                        }
                        ColorMode::Bw => {
                            if hglt {
                                [0xFF, 0xFF, 0xFF, 255]
                            } else {
                                [0xC0, 0xC0, 0xC0, 255]
                            }
                        }
                        ColorMode::Color => [
                            if hglt { 0x00 } else { 0xFF },
                            if gpa1 { 0x00 } else { 0xFF },
                            if gpa0 { 0x00 } else { 0xFF },
                            255,
                        ],
                    };

                    let char_idx = ((sym.chr as usize) + chargen) & 0xFF;
                    let row_data = if !vsp {
                        self.font_rom
                            .get(char_idx * 8 + (lc & 7))
                            .copied()
                            .unwrap_or(0xFF)
                    } else {
                        0xFF
                    };

                    let px_base = x * CHAR_WIDTH;
                    for col in 0..CHAR_WIDTH {
                        let px = px_base + col;
                        if px >= self.width as usize {
                            continue;
                        }

                        let pixel_bit = (row_data >> (5 - col)) & 1;
                        let mut pixel_on = pixel_bit == 0;

                        if lten {
                            pixel_on = true;
                        }
                        if rvv {
                            pixel_on = !pixel_on;
                        }

                        if pixel_on {
                            let px_idx = (px_base_y + px) * 4;
                            self.frame_buffer[px_idx..px_idx + 4].copy_from_slice(&fg_color);
                        }
                    }
                }
            }
        }

        if self.is_crt_blend {
            if size_changed {
                self.prev_frame_buffer.copy_from_slice(&self.frame_buffer);
            } else {
                for (curr, prev) in self
                    .frame_buffer
                    .chunks_exact_mut(4)
                    .zip(self.prev_frame_buffer.chunks_exact_mut(4))
                {
                    let r = ((curr[0] as u16 + prev[0] as u16) >> 1) as u8;
                    let g = ((curr[1] as u16 + prev[1] as u16) >> 1) as u8;
                    let b = ((curr[2] as u16 + prev[2] as u16) >> 1) as u8;

                    prev[0..3].copy_from_slice(&curr[0..3]);

                    curr[0] = r;
                    curr[1] = g;
                    curr[2] = b;
                }
            }
        }

        size_changed
    }
}
