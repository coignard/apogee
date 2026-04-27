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

const CHAR_WIDTH: usize = 6;
const CHARS_PER_ROW: usize = 78;
const ROWS_PER_SCREEN: usize = 30;
const LINES_PER_ROW: usize = 10;
const FONT_ALT_BANK_OFFSET: usize = 128;

pub const SCREEN_WIDTH: u32 = (CHARS_PER_ROW * CHAR_WIDTH) as u32;
pub const SCREEN_HEIGHT: u32 = (ROWS_PER_SCREEN * LINES_PER_ROW) as u32;

#[derive(Clone, Copy, PartialEq, Eq)]
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
    frame_buffer: Vec<u8>,
    prev_frame_buffer: Vec<u8>,
}

impl VideoRenderer {
    pub fn new(font_rom: Vec<u8>, color_mode: ColorMode, is_crt_blend: bool) -> Self {
        Self {
            font_rom,
            color_mode,
            is_crt_blend,
            frame_buffer: vec![0; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
            prev_frame_buffer: vec![0; (SCREEN_WIDTH * SCREEN_HEIGHT * 4) as usize],
        }
    }

    #[inline]
    pub fn frame_buffer(&self) -> &[u8] {
        &self.frame_buffer
    }

    pub fn render_frame(&mut self, vg75: &Kr580Vg75) {
        let bg_color = [0, 0, 0, 255];

        for px in self.frame_buffer.chunks_exact_mut(4) {
            px.copy_from_slice(&bg_color);
        }

        let n_rows = vg75.n_rows() as usize;
        let n_lines = vg75.n_lines() as usize;
        let n_chars = vg75.n_chars() as usize;

        for row in 0..n_rows {
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
                if py >= SCREEN_HEIGHT as usize {
                    continue;
                }

                let px_base_y = py * (SCREEN_WIDTH as usize);

                for x in 0..n_chars {
                    let sym = &vg75.parsed_frame()[row % 64][x];
                    let vsp = sym.get_vsp(ln);
                    let lten = sym.get_lten(ln);

                    let is_bw = self.color_mode == ColorMode::Bw;
                    let attr_sym = if is_bw && x < n_chars - 1 {
                        &vg75.parsed_frame()[row % 64][x + 1]
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
                        if px >= SCREEN_WIDTH as usize {
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
            for (curr, prev) in self
                .frame_buffer
                .chunks_exact_mut(4)
                .zip(self.prev_frame_buffer.chunks_exact_mut(4))
            {
                let r = (curr[0] >> 1) + (prev[0] >> 1);
                let g = (curr[1] >> 1) + (prev[1] >> 1);
                let b = (curr[2] >> 1) + (prev[2] >> 1);

                prev[0] = curr[0];
                prev[1] = curr[1];
                prev[2] = curr[2];

                curr[0] = r;
                curr[1] = g;
                curr[2] = b;
            }
        }
    }
}
