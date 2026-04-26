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

pub const VG75_IE: u8 = 0x40;
pub const VG75_IR: u8 = 0x20;
pub const VG75_IC: u8 = 0x08;
pub const VG75_VE: u8 = 0x04;
pub const VG75_DU: u8 = 0x02;
pub const VG75_FO: u8 = 0x01;

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

#[derive(Clone, Copy, PartialEq)]
enum Vg75Cmd {
    Reset,
    LoadCursor,
    ReadLpen,
    None,
}

#[derive(Clone, Copy, Default)]
pub struct ParsedSymbol {
    pub chr: u8,
    pub attrs: u8,
    pub vsp: u16,
    pub lten: u16,
}

impl ParsedSymbol {
    #[inline]
    pub fn rvv(&self) -> bool {
        (self.attrs & 0x01) != 0
    }
    #[inline]
    pub fn set_rvv(&mut self, val: bool) {
        if val {
            self.attrs |= 0x01;
        } else {
            self.attrs &= !0x01;
        }
    }
    #[inline]
    pub fn hglt(&self) -> bool {
        (self.attrs & 0x02) != 0
    }
    #[inline]
    pub fn set_hglt(&mut self, val: bool) {
        if val {
            self.attrs |= 0x02;
        } else {
            self.attrs &= !0x02;
        }
    }
    #[inline]
    pub fn gpa0(&self) -> bool {
        (self.attrs & 0x04) != 0
    }
    #[inline]
    pub fn set_gpa0(&mut self, val: bool) {
        if val {
            self.attrs |= 0x04;
        } else {
            self.attrs &= !0x04;
        }
    }
    #[inline]
    pub fn gpa1(&self) -> bool {
        (self.attrs & 0x08) != 0
    }
    #[inline]
    pub fn set_gpa1(&mut self, val: bool) {
        if val {
            self.attrs |= 0x08;
        } else {
            self.attrs &= !0x08;
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

pub struct Kr580Vg75 {
    cmd: Vg75Cmd,
    status: u8,
    param_num: usize,
    reset_param: [u8; 4],

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

    cursor_x: u8,
    cursor_y: u8,

    row_buffer: Vec<u8>,
    fifo: Vec<u8>,
    next_to_fifo: bool,
    dma_stopped_for_row: bool,
    is_end_of_screen: bool,
    was_dma_underrun: bool,

    attr_underline: bool,
    attr_reverse: bool,
    attr_blink: bool,
    attr_highlight: bool,
    attr_gpa0: bool,
    attr_gpa1: bool,
    is_blanked_to_end_of_screen: bool,

    parsed_frame: Box<[[ParsedSymbol; 80]; 64]>,

    cpu_inte: bool,
    cur_font_bank: bool,
    prev_row: usize,
    row_font_banks: [bool; 64],

    crt_x: u32,
    crt_scan_line: u32,
    crt_scan_row: u32,
    crt_cur_row: u32,
    frame_count: usize,
}

impl Kr580Vg75 {
    pub fn new() -> Self {
        Self {
            cmd: Vg75Cmd::None,
            status: 0,
            param_num: 0,
            reset_param: [0; 4],

            raster_running: false,

            n_chars: 78,
            n_rows: 30,
            n_lines: 10,
            n_hr_chars: 18,
            n_vr_rows: 4,
            und_line: 9,
            transparent_attr: false,
            font_down: false,
            cursor_blink: true,
            cursor_under: false,
            spaced_rows: false,

            cursor_x: 0,
            cursor_y: 0,

            row_buffer: Vec::with_capacity(80),
            fifo: Vec::with_capacity(16),
            next_to_fifo: false,
            dma_stopped_for_row: false,
            is_end_of_screen: false,
            was_dma_underrun: false,

            attr_underline: false,
            attr_reverse: false,
            attr_blink: false,
            attr_highlight: false,
            attr_gpa0: false,
            attr_gpa1: false,
            is_blanked_to_end_of_screen: false,

            parsed_frame: Box::new([[ParsedSymbol::default(); 80]; 64]),

            cpu_inte: false,
            cur_font_bank: false,
            prev_row: 0,
            row_font_banks: [false; 64],

            crt_x: 0,
            crt_scan_line: 0,
            crt_scan_row: 0,
            crt_cur_row: 0,
            frame_count: 0,
        }
    }

    #[inline]
    pub fn is_display_enabled(&self) -> bool {
        (self.status & VG75_VE) != 0
    }
    #[inline]
    pub fn is_ints_enabled(&self) -> bool {
        (self.status & VG75_IE) != 0
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
    pub fn parsed_frame(&self) -> &[[ParsedSymbol; 80]; 64] {
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
        if (port & 1) == 1 {
            let s = self.status;
            self.status = s & 0xC4;
            s
        } else {
            match self.cmd {
                Vg75Cmd::Reset => {
                    let val = self.reset_param[self.param_num];
                    self.param_num += 1;
                    if self.param_num == 4 {
                        self.cmd = Vg75Cmd::None;
                        self.status |= VG75_IC;
                        self.param_num = 0;
                    }
                    val
                }
                Vg75Cmd::LoadCursor => {
                    if self.param_num == 0 {
                        self.param_num = 1;
                        self.cursor_x
                    } else {
                        self.param_num = 0;
                        self.cmd = Vg75Cmd::None;
                        self.status |= VG75_IC;
                        self.cursor_y
                    }
                }
                Vg75Cmd::ReadLpen => {
                    if self.param_num == 0 {
                        self.param_num = 1;
                        0
                    } else {
                        self.param_num = 0;
                        self.cmd = Vg75Cmd::None;
                        0
                    }
                }
                _ => self.status & 0x7F,
            }
        }
    }

    pub fn write(&mut self, port: u16, val: u8) {
        if port & 1 == 1 {
            let cmd = val >> 5;
            self.status &= !VG75_IC;
            match cmd {
                0 => {
                    self.cmd = Vg75Cmd::Reset;
                    self.param_num = 0;
                    self.status = (self.status & !(VG75_IE | VG75_VE)) | VG75_IC;
                    self.start_raster_if_not_started();
                }
                1 => {
                    self.status |= VG75_IE | VG75_VE;
                    self.start_raster_if_not_started();
                }
                2 => {
                    self.status &= !VG75_VE;
                    self.start_raster_if_not_started();
                }
                3 => {
                    self.cmd = Vg75Cmd::ReadLpen;
                    self.param_num = 0;
                    self.start_raster_if_not_started();
                }
                4 => {
                    self.cmd = Vg75Cmd::LoadCursor;
                    self.param_num = 0;
                    self.status |= VG75_IC;
                    self.start_raster_if_not_started();
                }
                5 => {
                    self.status |= VG75_IE;
                    self.start_raster_if_not_started();
                }
                6 => {
                    self.status &= !VG75_IE;
                    self.start_raster_if_not_started();
                }
                7 => {
                    self.raster_running = false;
                    self.crt_scan_row = 0;
                    self.crt_scan_line = 0;
                    self.crt_x = 0;
                    self.crt_cur_row = 0;
                    self.is_end_of_screen = false;
                    self.was_dma_underrun = false;
                    self.row_buffer.clear();
                    self.fifo.clear();
                    self.next_to_fifo = false;
                    self.dma_stopped_for_row = false;
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
                    if self.param_num == 4 {
                        self.cmd = Vg75Cmd::None;
                        self.status &= !VG75_IC;
                        let rp = self.reset_param;
                        self.spaced_rows = (rp[0] & 0x80) != 0;
                        self.n_chars = (rp[0] & 0x7F) + 1;
                        self.n_vr_rows = ((rp[1] & 0xC0) >> 6) + 1;
                        self.n_rows = (rp[1] & 0x3F) + 1;
                        self.und_line = (rp[2] & 0xF0) >> 4;
                        self.n_lines = (rp[2] & 0x0F) + 1;
                        self.font_down = (rp[3] & 0x80) != 0;
                        self.transparent_attr = (rp[3] & 0x40) == 0;
                        self.cursor_blink = (rp[3] & 0x20) == 0;
                        self.cursor_under = (rp[3] & 0x10) != 0;
                        self.n_hr_chars = ((rp[3] & 0x0F) + 1) * 2;
                    }
                }
                Vg75Cmd::LoadCursor => {
                    if self.param_num == 0 {
                        self.cursor_x = val & 0x7F;
                        self.param_num = 1;
                    } else {
                        self.cursor_y = val & 0x3F;
                        self.param_num = 0;
                        self.cmd = Vg75Cmd::None;
                        self.status &= !VG75_IC;
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

    pub fn tick(&mut self, vt57: &mut Kr580Vt57, ram: &[u8; 0x10000]) -> bool {
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
                    self.next_row(vt57, ram);
                }

                if self.crt_scan_row == self.n_rows as u32 {
                    if self.is_ints_enabled() {
                        self.status |= VG75_IR;
                    }
                    self.finalize_font_banks();
                    return true;
                } else if self.crt_scan_row >= (self.n_rows as u32) + (self.n_vr_rows as u32) {
                    self.next_frame(vt57, ram);
                }
            }
        }
        false
    }

    fn next_row(&mut self, vt57: &mut Kr580Vt57, ram: &[u8; 0x10000]) {
        self.display_buffer();

        self.crt_cur_row = self.crt_scan_row;
        self.row_buffer.clear();
        self.fifo.clear();
        self.next_to_fifo = false;
        self.dma_stopped_for_row = false;

        if self.is_display_enabled() && self.crt_cur_row < self.n_rows as u32 {
            self.fetch_dma_row(vt57, ram);
        }
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
        self.fifo.clear();
        self.next_to_fifo = false;
        self.dma_stopped_for_row = false;
    }

    fn next_frame(&mut self, vt57: &mut Kr580Vt57, ram: &[u8; 0x10000]) {
        self.prepare_next_frame();

        if self.is_display_enabled() {
            self.fetch_dma_row(vt57, ram);
        }
    }

    fn fetch_dma_row(&mut self, vt57: &mut Kr580Vt57, ram: &[u8; 0x10000]) {
        if vt57.is_enabled() && !self.is_end_of_screen && !self.was_dma_underrun {
            let mut bytes_fetched = 0;

            while self.row_buffer.len() < self.n_chars as usize || self.next_to_fifo {
                let c = ram[vt57.ch2_addr() as usize];
                vt57.step_ch2();
                bytes_fetched += 1;

                if self.next_to_fifo {
                    if self.fifo.len() < 16 {
                        self.fifo.push(c);
                    } else {
                        self.status |= VG75_FO;
                    }
                    self.next_to_fifo = false;
                } else {
                    self.row_buffer.push(c);
                    if (c & 0xF1) == 0xF1 {
                        if (c & 0x02) != 0 {
                            self.is_end_of_screen = true;
                        }
                        self.dma_stopped_for_row = true;
                        break;
                    } else if self.transparent_attr && (c & 0xC0) == 0x80 {
                        self.next_to_fifo = true;
                    }
                }
            }
            vt57.add_halt_cycles(bytes_fetched * 4);
        }
    }

    fn display_buffer(&mut self) {
        let mut is_blanked_to_end_of_row = false;
        let mut fifo_pos = 0;

        for i in 0..(self.n_chars as usize) {
            let mut c = if i < self.row_buffer.len() {
                self.row_buffer[i]
            } else {
                if !self.dma_stopped_for_row && !self.is_end_of_screen && self.is_display_enabled()
                {
                    self.was_dma_underrun = true;
                    self.status |= VG75_DU;
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

            if self.transparent_attr && (c & 0xC0) == 0x80 {
                self.attr_underline = (c & 0x20) != 0;
                self.attr_reverse = (c & 0x10) != 0;
                self.attr_blink = (c & 0x02) != 0;
                self.attr_highlight = (c & 0x01) != 0;
                self.attr_gpa0 = (c & 0x04) != 0;
                self.attr_gpa1 = (c & 0x08) != 0;
                c = if fifo_pos < self.fifo.len() {
                    self.fifo[fifo_pos]
                } else {
                    0
                };
                fifo_pos += 1;
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
                    sym.set_vsp(j, self.attr_blink && (self.frame_count & 0x10) != 0);
                    sym.set_lten(j, false);
                    if self.und_line > 7
                        && (j == 0 || j == (self.n_lines as usize).saturating_sub(1))
                    {
                        sym.set_vsp(j, true);
                    }
                }
                if self.attr_underline && (self.und_line as usize) < 16 {
                    let lten_val = if self.attr_blink {
                        (self.frame_count & 0x10) == 0
                    } else {
                        true
                    };
                    sym.set_lten(self.und_line as usize, lten_val);
                }
            } else if (c & 0xC0) == 0x80 {
                self.attr_underline = (c & 0x20) != 0;
                self.attr_reverse = (c & 0x10) != 0;
                self.attr_blink = (c & 0x02) != 0;
                self.attr_highlight = (c & 0x01) != 0;
                self.attr_gpa0 = (c & 0x04) != 0;
                self.attr_gpa1 = (c & 0x08) != 0;
                for j in 0..(self.n_lines as usize) {
                    sym.set_vsp(j, true);
                    sym.set_lten(j, false);
                }
                sym.set_rvv(false);
                sym.set_hglt(self.attr_highlight);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
            } else if (c & 0xC0) == 0xC0 && (c & 0x30) != 0x30 {
                let cccc = ((c & 0x3C) >> 2) as usize;
                for j in 0..(self.n_lines as usize) {
                    if j < self.und_line as usize {
                        sym.set_vsp(
                            j,
                            CHAR_ATTR_VSP[cccc][0]
                                || ((c & 0x02) != 0 && (self.frame_count & 0x10) != 0),
                        );
                        sym.set_lten(j, false);
                    } else if j > self.und_line as usize {
                        sym.set_vsp(
                            j,
                            CHAR_ATTR_VSP[cccc][1]
                                || ((c & 0x02) != 0 && (self.frame_count & 0x10) != 0),
                        );
                        sym.set_lten(j, false);
                    } else {
                        let vsp_val = (c & 0x02) != 0 && (self.frame_count & 0x10) != 0;
                        sym.set_vsp(j, vsp_val);
                        sym.set_lten(j, CHAR_ATTR_LTEN[cccc] && !vsp_val);
                    }
                }
                sym.set_hglt((c & 0x01) != 0);
                sym.set_rvv(self.attr_reverse);
                sym.set_gpa0(self.attr_gpa0);
                sym.set_gpa1(self.attr_gpa1);
            } else {
                if (c & 0x02) != 0 {
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

            if (self.crt_cur_row as usize) < 64 && i < 80 {
                self.parsed_frame[self.crt_cur_row as usize][i] = sym;
            }
        }

        if self.is_display_enabled()
            && self.crt_cur_row as u8 == self.cursor_y
            && (self.cursor_x as usize) < 80
        {
            let cx = self.cursor_x as usize;
            if cx < self.n_chars as usize && (self.crt_cur_row as usize) < 64 {
                if self.cursor_under {
                    if (self.und_line as usize) < 16 {
                        let blink_state = !self.cursor_blink || (self.frame_count & 0x08) != 0;
                        self.parsed_frame[self.crt_cur_row as usize][cx]
                            .set_lten(self.und_line as usize, blink_state);
                    }
                } else {
                    if !self.cursor_blink || (self.frame_count & 0x08) != 0 {
                        let current_rvv = self.parsed_frame[self.crt_cur_row as usize][cx].rvv();
                        self.parsed_frame[self.crt_cur_row as usize][cx].set_rvv(!current_rvv);
                    }
                }
            }
        }
    }
}
