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

use super::kr580vt57::Kr580Vt57;
use serde::{Deserialize, Serialize};
use serde_big_array::BigArray;

pub const STATUS_INT_ENABLE: u8 = 0x40;
pub const STATUS_INT_REQUEST: u8 = 0x20;
pub const STATUS_IMPROPER_CMD: u8 = 0x08;
pub const STATUS_VIDEO_ENABLE: u8 = 0x04;
pub const STATUS_DMA_UNDERRUN: u8 = 0x02;
pub const STATUS_FIFO_OVERRUN: u8 = 0x01;

const STATUS_READ_PRESERVE_MASK: u8 = 0xC4;

const PORT_MASK: u16 = 1;
const PORT_STATUS_CMD: u16 = 1;

const CMD_RESET: u8 = 0;
const CMD_START_DISPLAY: u8 = 1;
const CMD_STOP_DISPLAY: u8 = 2;
const CMD_READ_LIGHT_PEN: u8 = 3;
const CMD_LOAD_CURSOR: u8 = 4;
const CMD_ENABLE_INT: u8 = 5;
const CMD_DISABLE_INT: u8 = 6;
const CMD_PRESET_COUNTERS: u8 = 7;

const RESET_PARAM_COUNT: usize = 4;
const PARAM_POS_CHAR: usize = 0;
const PARAM_POS_ROW: usize = 1;

const RESET_CHARS_PER_ROW_MASK: u8 = 0x7F;
const RESET_SPACED_ROWS_MASK: u8 = 0x80;
const RESET_VR_ROWS_MASK: u8 = 0xC0;
const RESET_DISPLAY_ROWS_MASK: u8 = 0x3F;
const RESET_UNDERLINE_LINE_MASK: u8 = 0xF0;
const RESET_LINES_PER_ROW_MASK: u8 = 0x0F;
const RESET_OFFSET_LINE_MASK: u8 = 0x80;
const RESET_TRANSPARENT_ATTR_MASK: u8 = 0x40;
const RESET_CURSOR_BLINK_MASK: u8 = 0x20;
const RESET_CURSOR_UNDER_MASK: u8 = 0x10;
const RESET_HR_CHARS_MASK: u8 = 0x0F;

const DEFAULT_CHARS_PER_ROW: u8 = 78;
const DEFAULT_DISPLAY_ROWS: u8 = 30;
const DEFAULT_LINES_PER_ROW: u8 = 10;
const DEFAULT_HR_CHARS: u8 = 18;
const DEFAULT_VR_ROWS: u8 = 4;
const DEFAULT_UNDERLINE_LINE: u8 = 9;

const ATTR_TRANSPARENT_MASK: u8 = 0xC0;
const ATTR_TRANSPARENT_VAL: u8 = 0x80;

const ATTR_PSEUDOGRAPHIC_MASK: u8 = 0xC0;
const ATTR_PSEUDOGRAPHIC_VAL: u8 = 0xC0;
const ATTR_PSEUDOGRAPHIC_EXCLUSION: u8 = 0x30;
const CHAR_ATTR_INDEX_MASK: u8 = 0x3C;
const CHAR_ATTR_INDEX_SHIFT: u8 = 2;

const SPECIAL_CODE_MASK: u8 = 0xF1;
const SPECIAL_CODE_VAL: u8 = 0xF1;
const SPECIAL_CODE_EOF: u8 = 0x02;

const CHAR_ATTR_UNDERLINE: u8 = 0x20;
const CHAR_ATTR_REVERSE: u8 = 0x10;
const CHAR_ATTR_BLINK: u8 = 0x02;
const CHAR_ATTR_HIGHLIGHT: u8 = 0x01;
const CHAR_ATTR_GPA0: u8 = 0x04;
const CHAR_ATTR_GPA1: u8 = 0x08;

const SYMBOL_ATTR_RVV: u8 = 0x01;
const SYMBOL_ATTR_HGLT: u8 = 0x02;
const SYMBOL_ATTR_GPA0: u8 = 0x04;
const SYMBOL_ATTR_GPA1: u8 = 0x08;

const BLINK_FAST_DIVISOR_MASK: usize = 0x08;
const BLINK_SLOW_DIVISOR_MASK: usize = 0x10;

const MAX_FIFO_LEN: usize = 16;
const MAX_ROWS: usize = 64;
const MAX_CHARS: usize = 80;

const CHAR_ATTR_VSP: [[bool; 2]; 12] = [
    [true, false],
    [true, false],
    [false, true],
    [false, true],
    [true, false],
    [false, false],
    [false, false],
    [false, true],
    [true, true],
    [false, false],
    [false, false],
    [false, false],
];

const CHAR_ATTR_LTEN: [bool; 12] = [
    false, false, false, false, true, false, false, true, true, false, true, false,
];

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum Vg75Cmd {
    Reset,
    LoadCursor,
    ReadLpen,
    None,
}

#[derive(Clone, Copy, Default, Serialize, Deserialize, Debug)]
pub struct ParsedSymbol {
    pub chr: u8,
    pub attrs: u8,
    pub vsp: u16,
    pub lten: u16,
}

impl ParsedSymbol {
    #[inline]
    pub fn rvv(&self) -> bool {
        (self.attrs & SYMBOL_ATTR_RVV) != 0
    }
    #[inline]
    pub fn set_rvv(&mut self, val: bool) {
        if val {
            self.attrs |= SYMBOL_ATTR_RVV;
        } else {
            self.attrs &= !SYMBOL_ATTR_RVV;
        }
    }
    #[inline]
    pub fn hglt(&self) -> bool {
        (self.attrs & SYMBOL_ATTR_HGLT) != 0
    }
    #[inline]
    pub fn set_hglt(&mut self, val: bool) {
        if val {
            self.attrs |= SYMBOL_ATTR_HGLT;
        } else {
            self.attrs &= !SYMBOL_ATTR_HGLT;
        }
    }
    #[inline]
    pub fn gpa0(&self) -> bool {
        (self.attrs & SYMBOL_ATTR_GPA0) != 0
    }
    #[inline]
    pub fn set_gpa0(&mut self, val: bool) {
        if val {
            self.attrs |= SYMBOL_ATTR_GPA0;
        } else {
            self.attrs &= !SYMBOL_ATTR_GPA0;
        }
    }
    #[inline]
    pub fn gpa1(&self) -> bool {
        (self.attrs & SYMBOL_ATTR_GPA1) != 0
    }
    #[inline]
    pub fn set_gpa1(&mut self, val: bool) {
        if val {
            self.attrs |= SYMBOL_ATTR_GPA1;
        } else {
            self.attrs &= !SYMBOL_ATTR_GPA1;
        }
    }
    #[inline]
    pub fn get_vsp(&self, line: usize) -> bool {
        ((self.vsp >> line) & 1) != 0
    }
    #[inline]
    pub fn set_vsp(&mut self, line: usize, val: bool) {
        if val {
            self.vsp |= 1 << line;
        } else {
            self.vsp &= !(1 << line);
        }
    }
    #[inline]
    pub fn get_lten(&self, line: usize) -> bool {
        ((self.lten >> line) & 1) != 0
    }
    #[inline]
    pub fn set_lten(&mut self, line: usize, val: bool) {
        if val {
            self.lten |= 1 << line;
        } else {
            self.lten &= !(1 << line);
        }
    }
}

mod frame_hash_serde {
    use super::{MAX_CHARS, MAX_ROWS, ParsedSymbol};
    use serde::{Deserialize, Deserializer, Serializer};
    use sha2::{Digest, Sha256};

    pub fn serialize<S>(
        frame: &[[ParsedSymbol; MAX_CHARS]; MAX_ROWS],
        serializer: S,
    ) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut hasher = Sha256::new();
        for row in frame {
            for sym in row {
                hasher.update([sym.chr, sym.attrs]);
                hasher.update(sym.vsp.to_le_bytes());
                hasher.update(sym.lten.to_le_bytes());
            }
        }
        let hash = hasher.finalize();
        serializer.serialize_str(&hex::encode(hash))
    }

    pub fn deserialize<'de, D>(
        deserializer: D,
    ) -> Result<Box<[[ParsedSymbol; MAX_CHARS]; MAX_ROWS]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let _hash = String::deserialize(deserializer)?;
        Ok(vec![[ParsedSymbol::default(); MAX_CHARS]; MAX_ROWS]
            .into_boxed_slice()
            .try_into()
            .unwrap())
    }
}

#[derive(Serialize, Deserialize)]
pub struct Kr580Vg75 {
    cmd: Vg75Cmd,
    status: u8,
    param_num: usize,
    reset_param: [u8; RESET_PARAM_COUNT],

    raster_running: bool,

    n_chars: u8,
    n_rows: u8,
    n_lines: u8,
    n_hr_chars: u8,
    n_vr_rows: u8,
    und_line: u8,
    transparent_attr: bool,
    font_down: bool,
    cursor_blink: bool,
    cursor_under: bool,
    spaced_rows: bool,

    burst_count: u8,
    burst_space: u8,
    cur_burst_pos: u8,
    dma_timer: u32,

    cursor_x: u8,
    cursor_y: u8,

    row_buffer: Vec<u8>,
    fifo: [u8; MAX_FIFO_LEN],
    fifo_write_pos: usize,
    next_to_fifo: bool,
    dma_stopped_for_row: bool,
    dma_paused: bool,
    need_extra_byte: bool,
    is_end_of_screen: bool,
    was_dma_underrun: bool,

    attr_underline: bool,
    attr_reverse: bool,
    attr_blink: bool,
    attr_highlight: bool,
    attr_gpa0: bool,
    attr_gpa1: bool,
    is_blanked_to_end_of_screen: bool,

    #[serde(with = "frame_hash_serde", rename = "parsed_frame_hash")]
    parsed_frame: Box<[[ParsedSymbol; MAX_CHARS]; MAX_ROWS]>,

    cpu_inte: bool,
    cur_font_bank: bool,
    prev_row: usize,

    #[serde(with = "BigArray")]
    row_font_banks: [bool; MAX_ROWS],

    crt_x: u32,
    crt_scan_line: u32,
    crt_scan_row: u32,
    crt_cur_row: u32,
    frame_count: usize,
}

impl Default for Kr580Vg75 {
    fn default() -> Self {
        Self::new()
    }
}

impl Kr580Vg75 {
    pub fn new() -> Self {
        Self {
            cmd: Vg75Cmd::None,
            status: 0,
            param_num: 0,
            reset_param: [0; RESET_PARAM_COUNT],

            raster_running: false,

            n_chars: DEFAULT_CHARS_PER_ROW,
            n_rows: DEFAULT_DISPLAY_ROWS,
            n_lines: DEFAULT_LINES_PER_ROW,
            n_hr_chars: DEFAULT_HR_CHARS,
            n_vr_rows: DEFAULT_VR_ROWS,
            und_line: DEFAULT_UNDERLINE_LINE,
            transparent_attr: false,
            font_down: false,
            cursor_blink: true,
            cursor_under: false,
            spaced_rows: false,

            burst_count: 1,
            burst_space: 0,
            cur_burst_pos: 0,
            dma_timer: 0,

            cursor_x: 0,
            cursor_y: 0,

            row_buffer: Vec::with_capacity(MAX_CHARS),
            fifo: [0; MAX_FIFO_LEN],
            fifo_write_pos: 0,
            next_to_fifo: bool::default(),
            dma_stopped_for_row: false,
            dma_paused: true,
            need_extra_byte: false,
            is_end_of_screen: false,
            was_dma_underrun: false,

            attr_underline: false,
            attr_reverse: false,
            attr_blink: false,
            attr_highlight: false,
            attr_gpa0: false,
            attr_gpa1: false,
            is_blanked_to_end_of_screen: false,

            parsed_frame: vec![[ParsedSymbol::default(); MAX_CHARS]; MAX_ROWS]
                .into_boxed_slice()
                .try_into()
                .unwrap(),

            cpu_inte: false,
            cur_font_bank: false,
            prev_row: 0,
            row_font_banks: [false; MAX_ROWS],

            crt_x: 0,
            crt_scan_line: 0,
            crt_scan_row: 0,
            crt_cur_row: 0,
            frame_count: 0,
        }
    }

    #[inline]
    pub fn is_display_enabled(&self) -> bool {
        (self.status & STATUS_VIDEO_ENABLE) != 0
    }

    #[inline]
    pub fn is_raster_running(&self) -> bool {
        self.raster_running
    }

    #[inline]
    pub fn is_ints_enabled(&self) -> bool {
        (self.status & STATUS_INT_ENABLE) != 0
    }

    #[inline]
    pub fn n_rows(&self) -> u8 {
        self.n_rows
    }

    #[inline]
    pub fn n_lines(&self) -> u8 {
        self.n_lines
    }

    #[inline]
    pub fn n_chars(&self) -> u8 {
        self.n_chars
    }

    #[inline]
    pub fn font_down(&self) -> bool {
        self.font_down
    }

    #[inline]
    pub fn row_font_bank(&self, row: usize) -> bool {
        self.row_font_banks[row]
    }

    #[inline]
    pub fn parsed_frame(&self) -> &[[ParsedSymbol; MAX_CHARS]; MAX_ROWS] {
        &self.parsed_frame
    }

    pub fn set_inte(&mut self, state: bool) {
        if self.cpu_inte == state {
            return;
        }

        self.cpu_inte = state;
        let cur_row = self.crt_cur_row as usize;

        if self.prev_row > cur_row {
            self.prev_row = 0;
        }
        for i in self.prev_row..cur_row {
            if i < self.row_font_banks.len() {
                self.row_font_banks[i] = self.cur_font_bank;
            }
        }
        if cur_row < self.row_font_banks.len() {
            self.row_font_banks[cur_row] = state;
        }
        self.cur_font_bank = state;
        self.prev_row = cur_row;
    }

    fn finalize_font_banks(&mut self) {
        let cur_row = self.crt_cur_row as usize;
        if self.prev_row >= cur_row {
            self.prev_row = 0;
        }
        for i in self.prev_row..=cur_row {
            if i < self.row_font_banks.len() {
                self.row_font_banks[i] = self.cur_font_bank;
            }
        }
        self.prev_row = cur_row;
    }

    pub fn read(&mut self, port: u16) -> u8 {
        if (port & PORT_MASK) == PORT_STATUS_CMD {
            let s = self.status;
            self.status = s & STATUS_READ_PRESERVE_MASK;
            s
        } else {
            match self.cmd {
                Vg75Cmd::Reset => {
                    let val = self.reset_param[self.param_num];
                    self.param_num += 1;
                    if self.param_num == RESET_PARAM_COUNT {
                        self.cmd = Vg75Cmd::None;
                        self.status |= STATUS_IMPROPER_CMD;
                        self.param_num = 0;
                    }
                    val
                }
                Vg75Cmd::LoadCursor => {
                    if self.param_num == PARAM_POS_CHAR {
                        self.param_num = PARAM_POS_ROW;
                        self.cursor_x
                    } else {
                        self.param_num = PARAM_POS_CHAR;
                        self.cmd = Vg75Cmd::None;
                        self.status |= STATUS_IMPROPER_CMD;
                        self.cursor_y
                    }
                }
                Vg75Cmd::ReadLpen => {
                    if self.param_num == PARAM_POS_CHAR {
                        self.param_num = PARAM_POS_ROW;
                        0
                    } else {
                        self.param_num = PARAM_POS_CHAR;
                        self.cmd = Vg75Cmd::None;
                        0
                    }
                }
                _ => self.status & 0x7F,
            }
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        if (port & PORT_MASK) == PORT_STATUS_CMD {
            let cmd = val >> 5;
            self.status &= !STATUS_IMPROPER_CMD;
            match cmd {
                CMD_RESET => {
                    self.cmd = Vg75Cmd::Reset;
                    self.param_num = 0;
                    self.status = (self.status & !(STATUS_INT_ENABLE | STATUS_VIDEO_ENABLE))
                        | STATUS_IMPROPER_CMD;
                    self.start_raster_if_not_started();
                }
                CMD_START_DISPLAY => {
                    self.status |= STATUS_INT_ENABLE | STATUS_VIDEO_ENABLE;
                    self.burst_count = 1 << (val & 0x03);
                    let space = (val >> 2) & 0x07;
                    self.burst_space = if space > 0 { space * 8 - 1 } else { 0 };
                    self.start_raster_if_not_started();
                }
                CMD_STOP_DISPLAY => {
                    self.status &= !STATUS_VIDEO_ENABLE;
                    self.start_raster_if_not_started();
                }
                CMD_READ_LIGHT_PEN => {
                    self.cmd = Vg75Cmd::ReadLpen;
                    self.param_num = PARAM_POS_CHAR;
                    self.start_raster_if_not_started();
                }
                CMD_LOAD_CURSOR => {
                    self.cmd = Vg75Cmd::LoadCursor;
                    self.param_num = PARAM_POS_CHAR;
                    self.status |= STATUS_IMPROPER_CMD;
                    self.start_raster_if_not_started();
                }
                CMD_ENABLE_INT => {
                    self.status |= STATUS_INT_ENABLE;
                    self.start_raster_if_not_started();
                }
                CMD_DISABLE_INT => {
                    self.status &= !STATUS_INT_ENABLE;
                    self.start_raster_if_not_started();
                }
                CMD_PRESET_COUNTERS => {
                    self.raster_running = false;
                    self.crt_scan_row = 0;
                    self.crt_scan_line = 0;
                    self.crt_x = 0;
                    self.crt_cur_row = 0;
                    self.is_end_of_screen = false;
                    self.was_dma_underrun = false;
                    self.row_buffer.clear();
                    self.fifo_write_pos = 0;
                    self.next_to_fifo = false;
                    self.dma_stopped_for_row = false;
                    self.dma_paused = true;
                    self.need_extra_byte = false;
                    self.dma_timer = 0;
                    self.cur_burst_pos = 0;
                    self.attr_underline = false;
                    self.attr_reverse = false;
                    self.attr_blink = false;
                    self.attr_highlight = false;
                    self.attr_gpa0 = false;
                    self.attr_gpa1 = false;
                    self.is_blanked_to_end_of_screen = false;
                    self.prev_row = 0;
                }
                _ => {}
            }
        } else {
            match self.cmd {
                Vg75Cmd::Reset => {
                    self.reset_param[self.param_num] = val;
                    self.param_num += 1;
                    if self.param_num == RESET_PARAM_COUNT {
                        self.cmd = Vg75Cmd::None;
                        self.status &= !STATUS_IMPROPER_CMD;
                        let rp = self.reset_param;
                        self.spaced_rows = (rp[0] & RESET_SPACED_ROWS_MASK) != 0;
                        self.n_chars = (rp[0] & RESET_CHARS_PER_ROW_MASK) + 1;
                        self.n_vr_rows = ((rp[1] & RESET_VR_ROWS_MASK) >> 6) + 1;
                        self.n_rows = (rp[1] & RESET_DISPLAY_ROWS_MASK) + 1;
                        self.und_line = (rp[2] & RESET_UNDERLINE_LINE_MASK) >> 4;
                        self.n_lines = (rp[2] & RESET_LINES_PER_ROW_MASK) + 1;
                        self.font_down = (rp[3] & RESET_OFFSET_LINE_MASK) != 0;
                        self.transparent_attr = (rp[3] & RESET_TRANSPARENT_ATTR_MASK) == 0;
                        self.cursor_blink = (rp[3] & RESET_CURSOR_BLINK_MASK) == 0;
                        self.cursor_under = (rp[3] & RESET_CURSOR_UNDER_MASK) != 0;
                        self.n_hr_chars = ((rp[3] & RESET_HR_CHARS_MASK) + 1) * 2;
                    }
                }
                Vg75Cmd::LoadCursor => {
                    if self.param_num == PARAM_POS_CHAR {
                        self.cursor_x = val & 0x7F;
                        self.param_num = PARAM_POS_ROW;
                    } else {
                        self.cursor_y = val & 0x3F;
                        self.param_num = PARAM_POS_CHAR;
                        self.cmd = Vg75Cmd::None;
                        self.status &= !STATUS_IMPROPER_CMD;
                    }
                }
                _ => {}
            }
        }
    }

    fn start_raster_if_not_started(&mut self) {
        if !self.raster_running {
            self.raster_running = true;
            self.crt_scan_row = 0;
            self.crt_scan_line = 0;
            self.crt_x = 0;
            self.prepare_next_frame();
        }
    }

    pub fn tick(&mut self, vt57: &mut Kr580Vt57, ram: &[u8; 0x10000]) {
        if !self.raster_running
            || !self.is_display_enabled()
            || self.dma_paused
            || self.is_end_of_screen
            || self.was_dma_underrun
        {
            return;
        }

        if self.dma_timer > 0 {
            self.dma_timer -= 1;
            return;
        }

        if !vt57.is_enabled() {
            return;
        }

        let c = ram[vt57.ch2_addr() as usize];
        vt57.step_ch2();
        vt57.add_halt_cycles(4);

        let mut is_paused = false;

        if self.need_extra_byte {
            self.need_extra_byte = false;
            is_paused = true;
        } else if self.next_to_fifo {
            self.fifo[self.fifo_write_pos] = c & 0x7F;
            self.fifo_write_pos = (self.fifo_write_pos + 1) % MAX_FIFO_LEN;
            if self.fifo_write_pos == 0 {
                self.status |= STATUS_FIFO_OVERRUN;
            }
            self.next_to_fifo = false;

            if self.row_buffer.len() == self.n_chars as usize {
                is_paused = true;
            }
        } else {
            if self.row_buffer.len() < self.n_chars as usize {
                self.row_buffer.push(c);
            } else {
                is_paused = true;
            }

            if !is_paused {
                if (c & SPECIAL_CODE_MASK) == SPECIAL_CODE_VAL {
                    if (c & SPECIAL_CODE_EOF) != 0 {
                        self.is_end_of_screen = true;
                    }
                    self.dma_stopped_for_row = true;

                    if self.row_buffer.len() == self.n_chars as usize
                        || self.cur_burst_pos == self.burst_count - 1
                    {
                        is_paused = true;
                    } else {
                        self.need_extra_byte = true;
                    }
                } else if self.transparent_attr
                    && (c & ATTR_TRANSPARENT_MASK) == ATTR_TRANSPARENT_VAL
                {
                    self.next_to_fifo = true;
                } else if self.row_buffer.len() == self.n_chars as usize {
                    is_paused = true;
                }
            }
        }

        self.dma_paused = is_paused;

        if self.dma_paused {
            return;
        }

        self.cur_burst_pos = (self.cur_burst_pos + 1) % self.burst_count;

        if self.cur_burst_pos == 0 {
            self.dma_timer = 3 + self.burst_space as u32;
        } else {
            self.dma_timer = if self.cur_burst_pos == 1 { 7 } else { 3 };
        }
    }

    pub fn tick_char(&mut self) -> bool {
        if !self.raster_running {
            return false;
        }

        self.crt_x += 1;
        let total_chars = (self.n_chars as u32) + (self.n_hr_chars as u32);

        if self.crt_x >= total_chars {
            self.crt_x = 0;
            self.crt_scan_line += 1;

            if self.crt_scan_line >= self.n_lines as u32 {
                self.crt_scan_line = 0;
                self.crt_scan_row += 1;

                if self.crt_scan_row <= self.n_rows as u32 {
                    self.next_row();
                }

                if self.crt_scan_row == self.n_rows as u32 {
                    if self.is_ints_enabled() {
                        self.status |= STATUS_INT_REQUEST;
                    }
                    self.finalize_font_banks();
                    return true;
                } else if self.crt_scan_row >= (self.n_rows as u32) + (self.n_vr_rows as u32) {
                    self.next_frame();
                }
            }
        }
        false
    }

    fn next_row(&mut self) {
        if self.is_display_enabled()
            && self.crt_cur_row < self.n_rows as u32
            && !self.dma_paused
            && !self.is_end_of_screen
        {
            self.was_dma_underrun = true;
            self.status |= STATUS_DMA_UNDERRUN;
        }

        self.display_buffer();

        self.crt_cur_row = self.crt_scan_row;
        self.row_buffer.clear();
        self.fifo_write_pos = 0;
        self.next_to_fifo = false;
        self.dma_stopped_for_row = false;

        self.dma_paused = self.crt_cur_row >= self.n_rows as u32
            || self.is_end_of_screen
            || self.was_dma_underrun;
        self.need_extra_byte = false;
        self.cur_burst_pos = 0;
        self.dma_timer = 0;
    }

    fn prepare_next_frame(&mut self) {
        self.frame_count = self.frame_count.wrapping_add(1);
        self.attr_underline = false;
        self.attr_reverse = false;
        self.attr_blink = false;
        self.attr_highlight = false;
        self.attr_gpa0 = false;
        self.attr_gpa1 = false;
        self.is_blanked_to_end_of_screen = false;
        self.was_dma_underrun = false;

        self.crt_cur_row = 0;
        self.crt_scan_row = 0;
        self.crt_scan_line = 0;
        self.is_end_of_screen = false;

        self.row_buffer.clear();
        self.fifo_write_pos = 0;
        self.next_to_fifo = false;
        self.dma_stopped_for_row = false;
        self.dma_paused = false;
        self.need_extra_byte = false;
        self.cur_burst_pos = 0;
        self.dma_timer = 0;
    }

    fn next_frame(&mut self) {
        self.prepare_next_frame();
    }

    fn display_buffer(&mut self) {
        let mut is_blanked_to_end_of_row = false;
        let mut fifo_read_pos = 0;

        for i in 0..(self.n_chars as usize) {
            let mut c = if let Some(&char_code) = self.row_buffer.get(i) {
                char_code
            } else {
                if !self.dma_stopped_for_row && !self.is_end_of_screen && self.is_display_enabled()
                {
                    self.was_dma_underrun = true;
                    self.status |= STATUS_DMA_UNDERRUN;
                }
                0
            };

            if self.was_dma_underrun
                || is_blanked_to_end_of_row
                || self.is_blanked_to_end_of_screen
                || !self.is_display_enabled()
            {
                c = 0;
            }

            if self.transparent_attr && (c & ATTR_TRANSPARENT_MASK) == ATTR_TRANSPARENT_VAL {
                self.attr_underline = (c & CHAR_ATTR_UNDERLINE) != 0;
                self.attr_reverse = (c & CHAR_ATTR_REVERSE) != 0;
                self.attr_blink = (c & CHAR_ATTR_BLINK) != 0;
                self.attr_highlight = (c & CHAR_ATTR_HIGHLIGHT) != 0;
                self.attr_gpa0 = (c & CHAR_ATTR_GPA0) != 0;
                self.attr_gpa1 = (c & CHAR_ATTR_GPA1) != 0;

                c = self.fifo[fifo_read_pos];
                fifo_read_pos = (fifo_read_pos + 1) % MAX_FIFO_LEN;
            }

            let mut sym = ParsedSymbol {
                chr: c & 0x7F,
                ..Default::default()
            };

            if !self.is_display_enabled()
                || is_blanked_to_end_of_row
                || self.is_blanked_to_end_of_screen
                || self.was_dma_underrun
            {
                sym.set_rvv(false);
                sym.set_hglt(false);
                sym.set_gpa0(false);
                sym.set_gpa1(false);
                for j in 0..(self.n_lines as usize) {
                    sym.set_vsp(j, true);
                    sym.set_lten(j, false);
                }
            } else if c < 0x80 {
                sym.set_rvv(self.attr_reverse);
                sym.set_hglt(self.attr_highlight);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
                for j in 0..(self.n_lines as usize) {
                    sym.set_vsp(
                        j,
                        self.attr_blink && (self.frame_count & BLINK_SLOW_DIVISOR_MASK) != 0,
                    );
                    sym.set_lten(j, false);
                    if self.und_line > 7
                        && (j == 0 || j == (self.n_lines as usize).saturating_sub(1))
                    {
                        sym.set_vsp(j, true);
                    }
                }
                if self.attr_underline && (self.und_line as usize) < 16 {
                    let lten_val = if self.attr_blink {
                        (self.frame_count & BLINK_SLOW_DIVISOR_MASK) == 0
                    } else {
                        true
                    };
                    sym.set_lten(self.und_line as usize, lten_val);
                }
            } else if (c & ATTR_TRANSPARENT_MASK) == ATTR_TRANSPARENT_VAL {
                self.attr_underline = (c & CHAR_ATTR_UNDERLINE) != 0;
                self.attr_reverse = (c & CHAR_ATTR_REVERSE) != 0;
                self.attr_blink = (c & CHAR_ATTR_BLINK) != 0;
                self.attr_highlight = (c & CHAR_ATTR_HIGHLIGHT) != 0;
                self.attr_gpa0 = (c & CHAR_ATTR_GPA0) != 0;
                self.attr_gpa1 = (c & CHAR_ATTR_GPA1) != 0;
                for j in 0..(self.n_lines as usize) {
                    sym.set_vsp(j, true);
                    sym.set_lten(j, false);
                }
                sym.set_rvv(false);
                sym.set_hglt(self.attr_highlight);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
            } else if (c & ATTR_PSEUDOGRAPHIC_MASK) == ATTR_PSEUDOGRAPHIC_VAL
                && (c & ATTR_PSEUDOGRAPHIC_EXCLUSION) != ATTR_PSEUDOGRAPHIC_EXCLUSION
            {
                let cccc = ((c & CHAR_ATTR_INDEX_MASK) >> CHAR_ATTR_INDEX_SHIFT) as usize;
                for j in 0..(self.n_lines as usize) {
                    if j < self.und_line as usize {
                        sym.set_vsp(
                            j,
                            CHAR_ATTR_VSP[cccc][0]
                                || ((c & CHAR_ATTR_BLINK) != 0
                                    && (self.frame_count & BLINK_SLOW_DIVISOR_MASK) != 0),
                        );
                        sym.set_lten(j, false);
                    } else if j > self.und_line as usize {
                        sym.set_vsp(
                            j,
                            CHAR_ATTR_VSP[cccc][1]
                                || ((c & CHAR_ATTR_BLINK) != 0
                                    && (self.frame_count & BLINK_SLOW_DIVISOR_MASK) != 0),
                        );
                        sym.set_lten(j, false);
                    } else {
                        let vsp_val = (c & CHAR_ATTR_BLINK) != 0
                            && (self.frame_count & BLINK_SLOW_DIVISOR_MASK) != 0;
                        sym.set_vsp(j, vsp_val);
                        sym.set_lten(j, CHAR_ATTR_LTEN[cccc] && !vsp_val);
                    }
                }
                sym.set_hglt((c & CHAR_ATTR_HIGHLIGHT) != 0);
                sym.set_rvv(self.attr_reverse);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
            } else {
                if (c & SPECIAL_CODE_EOF) != 0 {
                    self.is_blanked_to_end_of_screen = true;
                } else {
                    is_blanked_to_end_of_row = true;
                }
                sym.set_rvv(self.attr_reverse);
                sym.set_hglt(self.attr_highlight);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
                for j in 0..(self.n_lines as usize) {
                    sym.set_vsp(j, true);
                    sym.set_lten(j, false);
                }
                if self.attr_underline && (self.und_line as usize) < 16 {
                    sym.set_lten(self.und_line as usize, true);
                }
            }

            if (self.crt_cur_row as usize) < MAX_ROWS && i < MAX_CHARS {
                self.parsed_frame[self.crt_cur_row as usize][i] = sym;
            }
        }

        if self.is_display_enabled()
            && self.crt_cur_row as u8 == self.cursor_y
            && (self.cursor_x as usize) < MAX_CHARS
        {
            let cx = self.cursor_x as usize;
            if cx < self.n_chars as usize && (self.crt_cur_row as usize) < MAX_ROWS {
                if self.cursor_under {
                    if (self.und_line as usize) < 16 {
                        let blink_state =
                            !self.cursor_blink || (self.frame_count & BLINK_FAST_DIVISOR_MASK) != 0;
                        self.parsed_frame[self.crt_cur_row as usize][cx]
                            .set_lten(self.und_line as usize, blink_state);
                    }
                } else {
                    if !self.cursor_blink || (self.frame_count & BLINK_FAST_DIVISOR_MASK) != 0 {
                        let current_rvv = self.parsed_frame[self.crt_cur_row as usize][cx].rvv();
                        self.parsed_frame[self.crt_cur_row as usize][cx].set_rvv(!current_rvv);
                    }
                }
            }
        }
    }
}
