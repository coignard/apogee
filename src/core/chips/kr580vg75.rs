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

use serde::{Deserialize, Serialize};

pub const STATUS_INT_ENABLE: u8 = 0x40;
pub const STATUS_INT_REQUEST: u8 = 0x20;
pub const STATUS_LIGHT_PEN: u8 = 0x10;
pub const STATUS_IMPROPER_CMD: u8 = 0x08;
pub const STATUS_VIDEO_ENABLE: u8 = 0x04;
pub const STATUS_DMA_UNDERRUN: u8 = 0x02;
pub const STATUS_FIFO_OVERRUN: u8 = 0x01;

const STATUS_READ_PRESERVE_MASK: u8 = STATUS_INT_ENABLE | STATUS_VIDEO_ENABLE;
const STATUS_VALID_BITS_MASK: u8 = 0x7F;

const PORT_MASK: u16 = 1;
const PORT_STATUS_CMD: u16 = 1;
const OPEN_BUS_VALUE: u8 = 0xFF;

const COMMAND_SHIFT: u8 = 5;
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
const RESET_VR_ROWS_SHIFT: u8 = 6;
const RESET_DISPLAY_ROWS_MASK: u8 = 0x3F;
const RESET_UNDERLINE_LINE_MASK: u8 = 0xF0;
const RESET_UNDERLINE_LINE_SHIFT: u8 = 4;
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

const CHAR_MSB_MASK: u8 = 0x80;
const CHAR_CODE_MASK: u8 = 0x7F;

const FIELD_ATTRIBUTE_MASK: u8 = 0xC0;
const FIELD_ATTRIBUTE_VAL: u8 = 0x80;

const CHAR_ATTRIBUTE_MASK: u8 = 0xC0;
const CHAR_ATTRIBUTE_VAL: u8 = 0xC0;
const CHAR_ATTRIBUTE_EXCLUSION: u8 = 0x30;
const CHAR_ATTR_INDEX_MASK: u8 = 0x3C;
const CHAR_ATTR_INDEX_SHIFT: u8 = 2;

const SPECIAL_CODE_MASK: u8 = 0xF0;
const SPECIAL_CODE_VAL: u8 = 0xF0;
const SPECIAL_CODE_EOF_BIT: u8 = 0x02;
const SPECIAL_CODE_STOP_DMA_BIT: u8 = 0x01;

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

const BLINK_DIV_16_MASK: usize = 0x08;
const BLINK_DIV_32_MASK: usize = 0x10;

const BURST_COUNT_MASK: u8 = 0x03;
const BURST_SPACE_SHIFT: u8 = 2;
const BURST_SPACE_MASK: u8 = 0x07;
const BURST_SPACE_MULTIPLIER: u8 = 8;

const CURSOR_X_MASK: u8 = 0x7F;
const CURSOR_Y_MASK: u8 = 0x3F;

const MAX_FIFO_LEN: usize = 16;
const MAX_ROWS: usize = 64;
const MAX_CHARS: usize = 80;
const MAX_LINES_PER_ROW: usize = 16;
const UNDERLINE_MSB_THRESHOLD: u8 = 7;

#[derive(Clone, Copy)]
pub struct CharAttrBehavior {
    pub vsp_above: bool,
    pub vsp_below: bool,
    pub vsp_at_underline: bool,
    pub lten_at_underline: bool,
}

const CHAR_ATTR_DEFS: [CharAttrBehavior; 16] = [
    CharAttrBehavior {
        vsp_above: true,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: true,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: true,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: true,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: true,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: true,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: true,
        vsp_at_underline: false,
        lten_at_underline: true,
    },
    CharAttrBehavior {
        vsp_above: true,
        vsp_below: true,
        vsp_at_underline: false,
        lten_at_underline: true,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: true,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: true,
        vsp_below: true,
        vsp_at_underline: true,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
    CharAttrBehavior {
        vsp_above: false,
        vsp_below: false,
        vsp_at_underline: false,
        lten_at_underline: false,
    },
];

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
enum Vg75Cmd {
    Reset,
    LoadCursor,
    ReadLpen,
    None,
}

#[derive(Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
enum DmaState {
    #[default]
    Idle,
    Requesting,
    WaitingSpace,
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

    dma_state: DmaState,
    chars_in_current_burst: u8,
    cclk_wait_timer: u8,

    cursor_x: u8,
    cursor_y: u8,
    lpen_x: u8,
    lpen_y: u8,

    display_row_buffer: Vec<u8>,
    fill_row_buffer: Vec<u8>,

    display_fifo: [u8; MAX_FIFO_LEN],
    fill_fifo: [u8; MAX_FIFO_LEN],
    fill_fifo_pos: usize,
    display_fifo_pos: usize,

    next_to_fifo: bool,
    dma_stopped_for_row: bool,
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

            dma_state: DmaState::Idle,
            chars_in_current_burst: 0,
            cclk_wait_timer: 0,

            cursor_x: 0,
            cursor_y: 0,
            lpen_x: 0,
            lpen_y: 0,

            display_row_buffer: Vec::with_capacity(MAX_CHARS),
            fill_row_buffer: Vec::with_capacity(MAX_CHARS),

            display_fifo: [0; MAX_FIFO_LEN],
            fill_fifo: [0; MAX_FIFO_LEN],
            fill_fifo_pos: 0,
            display_fifo_pos: 0,

            next_to_fifo: false,
            dma_stopped_for_row: false,
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
    pub fn parsed_frame(&self) -> &[[ParsedSymbol; MAX_CHARS]; MAX_ROWS] {
        &self.parsed_frame
    }

    #[inline]
    pub fn drq(&self) -> bool {
        self.dma_state == DmaState::Requesting
    }

    #[inline]
    pub fn current_row(&self) -> usize {
        self.crt_cur_row as usize
    }

    pub fn trigger_light_pen(&mut self) {
        self.lpen_x = self.crt_x as u8;
        self.lpen_y = self.crt_scan_row as u8;
        self.status |= STATUS_LIGHT_PEN;
    }

    pub fn read(&mut self, port: u16) -> u8 {
        if (port & PORT_MASK) == PORT_STATUS_CMD {
            let s = self.status;
            self.status = s & STATUS_READ_PRESERVE_MASK;
            s & STATUS_VALID_BITS_MASK
        } else {
            match self.cmd {
                Vg75Cmd::ReadLpen => {
                    if self.param_num == PARAM_POS_CHAR {
                        self.param_num = PARAM_POS_ROW;
                        self.lpen_x
                    } else {
                        self.param_num = PARAM_POS_CHAR;
                        self.cmd = Vg75Cmd::None;
                        self.lpen_y
                    }
                }
                _ => {
                    self.status |= STATUS_IMPROPER_CMD;
                    OPEN_BUS_VALUE
                }
            }
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        if (port & PORT_MASK) == PORT_STATUS_CMD {
            let cmd = val >> COMMAND_SHIFT;

            if self.cmd != Vg75Cmd::None {
                self.status |= STATUS_IMPROPER_CMD;
            }

            match cmd {
                CMD_RESET => {
                    self.cmd = Vg75Cmd::Reset;
                    self.param_num = 0;
                    self.status &= !(STATUS_INT_ENABLE | STATUS_VIDEO_ENABLE | STATUS_INT_REQUEST);
                    self.start_raster_if_not_started();
                }
                CMD_START_DISPLAY => {
                    self.cmd = Vg75Cmd::None;
                    self.status |= STATUS_INT_ENABLE | STATUS_VIDEO_ENABLE;
                    self.burst_count = 1 << (val & BURST_COUNT_MASK);
                    let space = (val >> BURST_SPACE_SHIFT) & BURST_SPACE_MASK;
                    self.burst_space = if space > 0 {
                        space * BURST_SPACE_MULTIPLIER - 1
                    } else {
                        0
                    };
                    self.start_raster_if_not_started();
                }
                CMD_STOP_DISPLAY => {
                    self.cmd = Vg75Cmd::None;
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
                    self.start_raster_if_not_started();
                }
                CMD_ENABLE_INT => {
                    self.cmd = Vg75Cmd::None;
                    self.status |= STATUS_INT_ENABLE;
                    self.start_raster_if_not_started();
                }
                CMD_DISABLE_INT => {
                    self.cmd = Vg75Cmd::None;
                    self.status &= !STATUS_INT_ENABLE;
                    self.start_raster_if_not_started();
                }
                CMD_PRESET_COUNTERS => {
                    self.cmd = Vg75Cmd::None;
                    self.raster_running = false;
                    self.crt_scan_row = (self.n_rows as u32) + (self.n_vr_rows as u32) - 1;
                    self.crt_scan_line = 0;
                    self.crt_x = 0;
                    self.crt_cur_row = 0;
                    self.is_end_of_screen = false;
                    self.was_dma_underrun = false;
                    self.display_row_buffer.clear();
                    self.fill_row_buffer.clear();
                    self.fill_fifo_pos = 0;
                    self.display_fifo_pos = 0;
                    self.next_to_fifo = false;
                    self.dma_stopped_for_row = false;
                    self.dma_state = DmaState::Idle;
                    self.need_extra_byte = false;
                    self.chars_in_current_burst = 0;
                    self.cclk_wait_timer = 0;
                    self.reset_field_attributes();
                    self.is_blanked_to_end_of_screen = false;
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
                        let rp = self.reset_param;
                        self.spaced_rows = (rp[0] & RESET_SPACED_ROWS_MASK) != 0;
                        self.n_chars =
                            ((rp[0] & RESET_CHARS_PER_ROW_MASK) + 1).min(MAX_CHARS as u8);
                        self.n_vr_rows = ((rp[1] & RESET_VR_ROWS_MASK) >> RESET_VR_ROWS_SHIFT) + 1;
                        self.n_rows = (rp[1] & RESET_DISPLAY_ROWS_MASK) + 1;
                        self.und_line =
                            (rp[2] & RESET_UNDERLINE_LINE_MASK) >> RESET_UNDERLINE_LINE_SHIFT;
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
                        self.cursor_x = val & CURSOR_X_MASK;
                        self.param_num = PARAM_POS_ROW;
                    } else {
                        self.cursor_y = val & CURSOR_Y_MASK;
                        self.param_num = PARAM_POS_CHAR;
                        self.cmd = Vg75Cmd::None;
                    }
                }
                Vg75Cmd::ReadLpen | Vg75Cmd::None => {
                    self.status |= STATUS_IMPROPER_CMD;
                }
            }
        }
    }

    fn start_raster_if_not_started(&mut self) {
        if !self.raster_running {
            self.raster_running = true;
            self.crt_x = 0;
            self.crt_scan_line = 0;
            self.crt_scan_row = (self.n_rows as u32) + (self.n_vr_rows as u32) - 1;
            self.begin_row();
        }
    }

    pub fn dack(&mut self, c: u8) {
        if self.dma_state != DmaState::Requesting {
            return;
        }

        if self.need_extra_byte {
            self.need_extra_byte = false;
            self.dma_state = DmaState::Idle;
            return;
        }

        if self.next_to_fifo {
            self.fill_fifo[self.fill_fifo_pos % MAX_FIFO_LEN] = c & CHAR_CODE_MASK;
            self.fill_fifo_pos += 1;
            if self.fill_fifo_pos > MAX_FIFO_LEN {
                self.status |= STATUS_FIFO_OVERRUN;
            }
            self.next_to_fifo = false;
        } else {
            if self.fill_row_buffer.len() < self.n_chars as usize {
                self.fill_row_buffer.push(c);
            }

            if (c & SPECIAL_CODE_MASK) == SPECIAL_CODE_VAL {
                if (c & SPECIAL_CODE_STOP_DMA_BIT) != 0 {
                    if (c & SPECIAL_CODE_EOF_BIT) != 0 {
                        self.is_end_of_screen = true;
                    }
                    self.dma_stopped_for_row = true;
                }
            } else if self.transparent_attr && (c & FIELD_ATTRIBUTE_MASK) == FIELD_ATTRIBUTE_VAL {
                self.next_to_fifo = true;
            }
        }

        self.chars_in_current_burst += 1;

        if self.dma_stopped_for_row {
            if self.fill_row_buffer.len() == self.n_chars as usize
                || self.chars_in_current_burst >= self.burst_count
            {
                self.dma_state = DmaState::Idle;
            } else {
                self.need_extra_byte = true;
            }
        } else if self.fill_row_buffer.len() >= self.n_chars as usize {
            self.dma_state = DmaState::Idle;
        } else if self.chars_in_current_burst >= self.burst_count {
            self.chars_in_current_burst = 0;
            self.cclk_wait_timer = self.burst_space;
            if self.cclk_wait_timer > 0 {
                self.dma_state = DmaState::WaitingSpace;
            } else {
                self.dma_state = DmaState::Requesting;
            }
        }
    }

    pub fn tick_char(&mut self) -> bool {
        if !self.raster_running {
            return false;
        }

        if self.dma_state == DmaState::WaitingSpace {
            if self.cclk_wait_timer > 0 {
                self.cclk_wait_timer -= 1;
            }
            if self.cclk_wait_timer == 0 {
                self.dma_state = DmaState::Requesting;
            }
        }

        self.crt_x += 1;
        let total_chars = (self.n_chars as u32) + (self.n_hr_chars as u32);

        if self.crt_x >= total_chars {
            self.crt_x = 0;
            self.crt_scan_line += 1;

            if self.crt_scan_line >= self.n_lines as u32 {
                self.crt_scan_line = 0;
                self.crt_scan_row += 1;

                if self.crt_scan_row >= (self.n_rows as u32) + (self.n_vr_rows as u32) {
                    self.crt_scan_row = 0;
                    self.frame_count = self.frame_count.wrapping_add(1);
                    self.reset_field_attributes();
                }

                self.begin_row();

                if self.crt_scan_row == (self.n_rows as u32).saturating_sub(1)
                    && self.is_ints_enabled()
                {
                    self.status |= STATUS_INT_REQUEST;
                }

                if self.crt_scan_row == self.n_rows as u32 {
                    return true;
                }
            }
        }
        false
    }

    fn begin_row(&mut self) {
        let just_finished_filling_idx = if self.crt_scan_row == 0 {
            Some(0)
        } else if self.crt_scan_row <= self.n_rows as u32 {
            Some(self.crt_scan_row)
        } else {
            None
        };

        if let Some(idx) = just_finished_filling_idx {
            let is_spaced = self.spaced_rows && (idx % 2 != 0);
            if self.is_display_enabled()
                && !self.is_end_of_screen
                && !is_spaced
                && !self.dma_stopped_for_row
                && self.fill_row_buffer.len() < self.n_chars as usize
            {
                self.was_dma_underrun = true;
                self.status |= STATUS_DMA_UNDERRUN;
            }
        }

        if self.crt_scan_row < self.n_rows as u32 {
            self.crt_cur_row = self.crt_scan_row;
            std::mem::swap(&mut self.display_row_buffer, &mut self.fill_row_buffer);
            self.fill_row_buffer.clear();
            self.display_fifo = self.fill_fifo;
            self.display_fifo_pos = self.fill_fifo_pos;
            self.render_current_row();
        } else {
            self.fill_row_buffer.clear();
        }

        self.dma_stopped_for_row = false;
        self.need_extra_byte = false;
        self.chars_in_current_burst = 0;
        self.cclk_wait_timer = 0;

        let filling_row_idx =
            if self.crt_scan_row == (self.n_rows as u32) + (self.n_vr_rows as u32) - 1 {
                self.is_end_of_screen = false;
                self.is_blanked_to_end_of_screen = false;
                self.was_dma_underrun = false;
                Some(0)
            } else if self.crt_scan_row < self.n_rows as u32 - 1 {
                Some(self.crt_scan_row + 1)
            } else {
                None
            };

        if let Some(idx) = filling_row_idx {
            self.fill_fifo_pos = 0;
            let is_spaced = self.spaced_rows && (idx % 2 != 0);
            if self.is_end_of_screen
                || self.was_dma_underrun
                || is_spaced
                || !self.is_display_enabled()
            {
                self.dma_state = DmaState::Idle;
            } else {
                self.cclk_wait_timer = self.burst_space;
                if self.cclk_wait_timer > 0 {
                    self.dma_state = DmaState::WaitingSpace;
                } else {
                    self.dma_state = DmaState::Requesting;
                }
            }
        } else {
            self.dma_state = DmaState::Idle;
        }
    }

    fn reset_field_attributes(&mut self) {
        self.attr_underline = false;
        self.attr_reverse = false;
        self.attr_blink = false;
        self.attr_highlight = false;
        self.attr_gpa0 = false;
        self.attr_gpa1 = false;
    }

    fn render_current_row(&mut self) {
        let mut is_blanked_to_end_of_row = false;
        let mut fifo_read_pos = if self.display_fifo_pos > MAX_FIFO_LEN {
            self.display_fifo_pos % MAX_FIFO_LEN
        } else {
            0
        };
        let is_spaced_row_blank = self.spaced_rows && !self.crt_scan_row.is_multiple_of(2);

        for i in 0..(self.n_chars as usize) {
            let mut c = self.display_row_buffer.get(i).copied().unwrap_or(0);

            let is_forced_blank = self.was_dma_underrun
                || self.is_blanked_to_end_of_screen
                || !self.is_display_enabled()
                || is_spaced_row_blank;

            let mut is_effectively_blanked = is_forced_blank || is_blanked_to_end_of_row;

            if self.transparent_attr && (c & FIELD_ATTRIBUTE_MASK) == FIELD_ATTRIBUTE_VAL {
                if !is_effectively_blanked {
                    self.attr_underline = (c & CHAR_ATTR_UNDERLINE) != 0;
                    self.attr_reverse = (c & CHAR_ATTR_REVERSE) != 0;
                    self.attr_blink = (c & CHAR_ATTR_BLINK) != 0;
                    self.attr_highlight = (c & CHAR_ATTR_HIGHLIGHT) != 0;
                    self.attr_gpa0 = (c & CHAR_ATTR_GPA0) != 0;
                    self.attr_gpa1 = (c & CHAR_ATTR_GPA1) != 0;
                }

                c = self.display_fifo[fifo_read_pos];
                fifo_read_pos = (fifo_read_pos + 1) % MAX_FIFO_LEN;
            }

            let mut sym = ParsedSymbol {
                chr: c & CHAR_CODE_MASK,
                ..Default::default()
            };

            if (c & SPECIAL_CODE_MASK) == SPECIAL_CODE_VAL {
                if (c & SPECIAL_CODE_EOF_BIT) != 0 {
                    self.is_blanked_to_end_of_screen = true;
                } else {
                    is_blanked_to_end_of_row = true;
                }
                is_effectively_blanked = true;
            }

            if is_effectively_blanked {
                sym.chr = 0;
                sym.set_rvv(false);
                sym.set_hglt(false);
                sym.set_gpa0(false);
                sym.set_gpa1(false);
                for j in 0..(self.n_lines as usize) {
                    sym.set_vsp(j, true);
                    sym.set_lten(j, false);
                }
            } else if (c & CHAR_MSB_MASK) == 0 {
                sym.set_rvv(self.attr_reverse);
                sym.set_hglt(self.attr_highlight);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
                for j in 0..(self.n_lines as usize) {
                    sym.set_vsp(
                        j,
                        self.attr_blink && (self.frame_count & BLINK_DIV_32_MASK) != 0,
                    );
                    sym.set_lten(j, false);
                }
                if self.attr_underline && (self.und_line as usize) < MAX_LINES_PER_ROW {
                    let lten_val = if self.attr_blink {
                        (self.frame_count & BLINK_DIV_32_MASK) == 0
                    } else {
                        true
                    };
                    sym.set_lten(self.und_line as usize, lten_val);
                }
            } else if (c & FIELD_ATTRIBUTE_MASK) == FIELD_ATTRIBUTE_VAL {
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
            } else if (c & CHAR_ATTRIBUTE_MASK) == CHAR_ATTRIBUTE_VAL
                && (c & CHAR_ATTRIBUTE_EXCLUSION) != CHAR_ATTRIBUTE_EXCLUSION
            {
                let cccc = ((c & CHAR_ATTR_INDEX_MASK) >> CHAR_ATTR_INDEX_SHIFT) as usize;
                let def = CHAR_ATTR_DEFS[cccc];
                let blink_active =
                    (c & CHAR_ATTR_BLINK) != 0 && (self.frame_count & BLINK_DIV_32_MASK) != 0;

                for j in 0..(self.n_lines as usize) {
                    if j < self.und_line as usize {
                        sym.set_vsp(j, def.vsp_above || blink_active);
                        sym.set_lten(j, false);
                    } else if j > self.und_line as usize {
                        sym.set_vsp(j, def.vsp_below || blink_active);
                        sym.set_lten(j, false);
                    } else {
                        sym.set_vsp(j, def.vsp_at_underline || blink_active);
                        sym.set_lten(j, def.lten_at_underline && !blink_active);
                    }
                }
                sym.set_hglt((c & CHAR_ATTR_HIGHLIGHT) != 0);
                sym.set_rvv(self.attr_reverse);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
            } else {
                sym.set_rvv(self.attr_reverse);
                sym.set_hglt(self.attr_highlight);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
                for j in 0..(self.n_lines as usize) {
                    sym.set_vsp(j, true);
                    sym.set_lten(j, false);
                }
                if self.attr_underline && (self.und_line as usize) < MAX_LINES_PER_ROW {
                    sym.set_lten(self.und_line as usize, true);
                }
            }

            if self.und_line > UNDERLINE_MSB_THRESHOLD && self.n_lines > 0 {
                sym.set_vsp(0, true);
                sym.set_vsp((self.n_lines as usize).saturating_sub(1), true);
            }

            if (self.crt_scan_row as usize) < MAX_ROWS && i < MAX_CHARS {
                self.parsed_frame[self.crt_scan_row as usize][i] = sym;
            }
        }

        if self.is_display_enabled()
            && self.crt_scan_row as u8 == self.cursor_y
            && (self.cursor_x as usize) < MAX_CHARS
        {
            let cx = self.cursor_x as usize;
            if cx < self.n_chars as usize && (self.crt_scan_row as usize) < MAX_ROWS {
                if self.cursor_under {
                    if (self.und_line as usize) < MAX_LINES_PER_ROW
                        && (!self.cursor_blink || (self.frame_count & BLINK_DIV_16_MASK) != 0)
                    {
                        self.parsed_frame[self.crt_scan_row as usize][cx]
                            .set_lten(self.und_line as usize, true);
                    }
                } else {
                    if !self.cursor_blink || (self.frame_count & BLINK_DIV_16_MASK) != 0 {
                        let rvv = self.parsed_frame[self.crt_scan_row as usize][cx].rvv();
                        self.parsed_frame[self.crt_scan_row as usize][cx].set_rvv(!rvv);
                    }
                }
            }
        }
    }
}
