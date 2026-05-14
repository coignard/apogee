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

use apogee_rs::core::peripherals::keyboard::Key;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use winit::event::{ElementState, KeyEvent};
use winit::keyboard::{Key as WinitKey, KeyCode, PhysicalKey};

#[derive(clap::ValueEnum, Clone, Copy, Debug, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeyboardLayout {
    #[default]
    Smart,
    Qwerty,
    Jcuken,
}

#[derive(Clone, Copy, Debug)]
struct ActiveKeyInfo {
    base_key: Key,
    synth_shift: Option<bool>,
    synth_ctrl: Option<bool>,
    synth_lang: Option<bool>,
}

impl ActiveKeyInfo {
    const fn new(base_key: Key) -> Self {
        Self {
            base_key,
            synth_shift: None,
            synth_ctrl: None,
            synth_lang: None,
        }
    }
}

pub struct KeyboardTranslator {
    pub layout: KeyboardLayout,
    active_keys: HashMap<PhysicalKey, ActiveKeyInfo>,

    phys_shift: bool,
    phys_ctrl: bool,
    phys_lang: bool,

    emu_shift: bool,
    emu_ctrl: bool,
    emu_lang: bool,
}

impl Default for KeyboardTranslator {
    fn default() -> Self {
        Self::new(KeyboardLayout::default())
    }
}

impl KeyboardTranslator {
    pub fn new(layout: KeyboardLayout) -> Self {
        Self {
            layout,
            active_keys: HashMap::with_capacity(8),
            phys_shift: false,
            phys_ctrl: false,
            phys_lang: false,
            emu_shift: false,
            emu_ctrl: false,
            emu_lang: false,
        }
    }

    pub fn process_key(&mut self, event: &KeyEvent) -> Vec<(Key, bool)> {
        let is_pressed = event.state == ElementState::Pressed;
        let phys = event.physical_key;

        let mut actions = Vec::with_capacity(4);

        if let PhysicalKey::Code(code) = phys {
            match code {
                KeyCode::ShiftLeft | KeyCode::ShiftRight => self.phys_shift = is_pressed,
                KeyCode::ControlLeft => self.phys_ctrl = is_pressed,
                KeyCode::ControlRight | KeyCode::Insert | KeyCode::Numpad0 => {
                    self.phys_lang = is_pressed;
                }
                _ => {}
            }
        }

        if is_pressed {
            if let Some(info) = self.translate(event) {
                self.active_keys.insert(phys, info);
                self.sync_modifiers(&mut actions);
                actions.push((info.base_key, true));
            } else {
                self.sync_modifiers(&mut actions);
            }
        } else {
            if let Some(info) = self.active_keys.remove(&phys) {
                actions.push((info.base_key, false));
            }
            self.sync_modifiers(&mut actions);
        }

        actions
    }

    fn sync_modifiers(&mut self, actions: &mut Vec<(Key, bool)>) {
        let mut force_shift_true = false;
        let mut force_shift_false = false;
        let mut force_ctrl_true = false;
        let mut force_ctrl_false = false;
        let mut force_lang_true = false;
        let mut force_lang_false = false;

        for info in self.active_keys.values() {
            match info.synth_shift {
                Some(true) => force_shift_true = true,
                Some(false) => force_shift_false = true,
                None => {}
            }
            match info.synth_ctrl {
                Some(true) => force_ctrl_true = true,
                Some(false) => force_ctrl_false = true,
                None => {}
            }
            match info.synth_lang {
                Some(true) => force_lang_true = true,
                Some(false) => force_lang_false = true,
                None => {}
            }
        }

        let want_shift = force_shift_true || (!force_shift_false && self.phys_shift);
        let want_ctrl = force_ctrl_true || (!force_ctrl_false && self.phys_ctrl);
        let want_lang = force_lang_true || (!force_lang_false && self.phys_lang);

        if want_shift != self.emu_shift {
            self.emu_shift = want_shift;
            actions.push((Key::Shift, want_shift));
        }
        if want_ctrl != self.emu_ctrl {
            self.emu_ctrl = want_ctrl;
            actions.push((Key::Ctrl, want_ctrl));
        }
        if want_lang != self.emu_lang {
            self.emu_lang = want_lang;
            actions.push((Key::Lang, want_lang));
        }
    }

    fn is_control_key(code: KeyCode) -> bool {
        matches!(
            code,
            KeyCode::Enter
                | KeyCode::NumpadEnter
                | KeyCode::Backspace
                | KeyCode::Escape
                | KeyCode::Tab
                | KeyCode::ArrowUp
                | KeyCode::ArrowDown
                | KeyCode::ArrowLeft
                | KeyCode::ArrowRight
                | KeyCode::Home
                | KeyCode::End
                | KeyCode::PageUp
                | KeyCode::PageDown
                | KeyCode::F1
                | KeyCode::F2
                | KeyCode::F3
                | KeyCode::F4
                | KeyCode::F5
                | KeyCode::F6
                | KeyCode::F7
                | KeyCode::F8
                | KeyCode::F9
                | KeyCode::F10
                | KeyCode::F11
                | KeyCode::F12
        )
    }

    fn translate(&self, event: &KeyEvent) -> Option<ActiveKeyInfo> {
        let code_opt = match event.physical_key {
            PhysicalKey::Code(c) => Some(c),
            _ => None,
        };

        if let Some(c) = code_opt
            && Self::is_control_key(c)
        {
            return map_qwerty(c).map(ActiveKeyInfo::new);
        }

        match self.layout {
            KeyboardLayout::Smart => {
                if let WinitKey::Character(ref ch) = event.logical_key
                    && let Some((k, shift, lang)) = map_smart_char(ch.as_str())
                {
                    return Some(ActiveKeyInfo {
                        base_key: k,
                        synth_shift: Some(shift),
                        synth_ctrl: None,
                        synth_lang: Some(lang),
                    });
                }
                code_opt.and_then(map_qwerty).map(ActiveKeyInfo::new)
            }
            KeyboardLayout::Qwerty => code_opt.and_then(map_qwerty).map(ActiveKeyInfo::new),
            KeyboardLayout::Jcuken => code_opt.and_then(map_jcuken).map(ActiveKeyInfo::new),
        }
    }
}

pub fn map_common(keycode: KeyCode) -> Option<Key> {
    match keycode {
        KeyCode::Digit0 => Some(Key::Num0),
        KeyCode::Digit1 => Some(Key::Num1),
        KeyCode::Digit2 => Some(Key::Num2),
        KeyCode::Digit3 => Some(Key::Num3),
        KeyCode::Digit4 => Some(Key::Num4),
        KeyCode::Digit5 => Some(Key::Num5),
        KeyCode::Digit6 => Some(Key::Num6),
        KeyCode::Digit7 => Some(Key::Num7),
        KeyCode::Digit8 => Some(Key::Num8),
        KeyCode::Digit9 => Some(Key::Num9),
        KeyCode::F1 => Some(Key::F1),
        KeyCode::F2 => Some(Key::F2),
        KeyCode::F3 => Some(Key::F3),
        KeyCode::F4 => Some(Key::F4),
        KeyCode::F5 => Some(Key::F5),
        KeyCode::Slash | KeyCode::NumpadDivide => Some(Key::Slash),
        KeyCode::Semicolon => Some(Key::Semicolon),
        KeyCode::Equal => Some(Key::Colon),
        KeyCode::Minus => Some(Key::Minus),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::Period => Some(Key::Period),
        KeyCode::Space => Some(Key::Space),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::Enter | KeyCode::NumpadEnter => Some(Key::Enter),
        KeyCode::Escape => Some(Key::Escape),
        KeyCode::ArrowUp | KeyCode::Numpad8 => Some(Key::Up),
        KeyCode::ArrowDown | KeyCode::Numpad2 => Some(Key::Down),
        KeyCode::ArrowLeft | KeyCode::Numpad4 => Some(Key::Left),
        KeyCode::ArrowRight | KeyCode::Numpad6 => Some(Key::Right),
        KeyCode::Comma => Some(Key::Comma),
        KeyCode::Home => Some(Key::Home),
        KeyCode::PageUp => Some(Key::Clear),
        KeyCode::PageDown => Some(Key::LineFeed),
        _ => None,
    }
}

pub fn map_qwerty(keycode: KeyCode) -> Option<Key> {
    match keycode {
        KeyCode::KeyA => Some(Key::A),
        KeyCode::KeyB => Some(Key::B),
        KeyCode::KeyC => Some(Key::C),
        KeyCode::KeyD => Some(Key::D),
        KeyCode::KeyE => Some(Key::E),
        KeyCode::KeyF => Some(Key::F),
        KeyCode::KeyG => Some(Key::G),
        KeyCode::KeyH => Some(Key::H),
        KeyCode::KeyI => Some(Key::I),
        KeyCode::KeyJ => Some(Key::J),
        KeyCode::KeyK => Some(Key::K),
        KeyCode::KeyL => Some(Key::L),
        KeyCode::KeyM => Some(Key::M),
        KeyCode::KeyN => Some(Key::N),
        KeyCode::KeyO => Some(Key::O),
        KeyCode::KeyP => Some(Key::P),
        KeyCode::KeyQ => Some(Key::Q),
        KeyCode::KeyR => Some(Key::R),
        KeyCode::KeyS => Some(Key::S),
        KeyCode::KeyT => Some(Key::T),
        KeyCode::KeyU => Some(Key::U),
        KeyCode::KeyV => Some(Key::V),
        KeyCode::KeyW => Some(Key::W),
        KeyCode::KeyX => Some(Key::X),
        KeyCode::KeyY => Some(Key::Y),
        KeyCode::KeyZ => Some(Key::Z),
        KeyCode::BracketLeft => Some(Key::BracketLeft),
        KeyCode::BracketRight => Some(Key::BracketRight),
        KeyCode::Backslash => Some(Key::Backslash),
        KeyCode::Quote => Some(Key::Caret),
        KeyCode::Backquote => Some(Key::At),
        _ => map_common(keycode),
    }
}

pub fn map_jcuken(code: KeyCode) -> Option<Key> {
    match code {
        KeyCode::KeyQ => Some(Key::J),
        KeyCode::KeyW => Some(Key::C),
        KeyCode::KeyE => Some(Key::U),
        KeyCode::KeyR => Some(Key::K),
        KeyCode::KeyT => Some(Key::E),
        KeyCode::KeyY => Some(Key::N),
        KeyCode::KeyU => Some(Key::G),
        KeyCode::KeyI => Some(Key::BracketLeft),
        KeyCode::KeyO => Some(Key::BracketRight),
        KeyCode::KeyP => Some(Key::Z),
        KeyCode::BracketLeft => Some(Key::H),

        KeyCode::KeyA => Some(Key::F),
        KeyCode::KeyS => Some(Key::Y),
        KeyCode::KeyD => Some(Key::W),
        KeyCode::KeyF => Some(Key::A),
        KeyCode::KeyG => Some(Key::P),
        KeyCode::KeyH => Some(Key::R),
        KeyCode::KeyJ => Some(Key::O),
        KeyCode::KeyK => Some(Key::L),
        KeyCode::KeyL => Some(Key::D),
        KeyCode::Semicolon => Some(Key::V),
        KeyCode::Quote => Some(Key::Backslash),

        KeyCode::KeyZ => Some(Key::Q),
        KeyCode::KeyX => Some(Key::Caret),
        KeyCode::KeyC => Some(Key::S),
        KeyCode::KeyV => Some(Key::M),
        KeyCode::KeyB => Some(Key::I),
        KeyCode::KeyN => Some(Key::T),
        KeyCode::KeyM => Some(Key::X),
        KeyCode::Comma => Some(Key::B),
        KeyCode::Period => Some(Key::At),

        _ => map_common(code),
    }
}

fn map_smart_char(s: &str) -> Option<(Key, bool, bool)> {
    let mut chars = s.chars();
    let c = match (chars.next(), chars.next()) {
        (Some(c), None) => c,
        _ => return None,
    };

    let (key, mut shift, mut lang) = match c {
        'a' => (Key::A, false, false),
        'A' => (Key::A, true, false),
        'b' => (Key::B, false, false),
        'B' => (Key::B, true, false),
        'c' => (Key::C, false, false),
        'C' => (Key::C, true, false),
        'd' => (Key::D, false, false),
        'D' => (Key::D, true, false),
        'e' => (Key::E, false, false),
        'E' => (Key::E, true, false),
        'f' => (Key::F, false, false),
        'F' => (Key::F, true, false),
        'g' => (Key::G, false, false),
        'G' => (Key::G, true, false),
        'h' => (Key::H, false, false),
        'H' => (Key::H, true, false),
        'i' => (Key::I, false, false),
        'I' => (Key::I, true, false),
        'j' => (Key::J, false, false),
        'J' => (Key::J, true, false),
        'k' => (Key::K, false, false),
        'K' => (Key::K, true, false),
        'l' => (Key::L, false, false),
        'L' => (Key::L, true, false),
        'm' => (Key::M, false, false),
        'M' => (Key::M, true, false),
        'n' => (Key::N, false, false),
        'N' => (Key::N, true, false),
        'o' => (Key::O, false, false),
        'O' => (Key::O, true, false),
        'p' => (Key::P, false, false),
        'P' => (Key::P, true, false),
        'q' => (Key::Q, false, false),
        'Q' => (Key::Q, true, false),
        'r' => (Key::R, false, false),
        'R' => (Key::R, true, false),
        's' => (Key::S, false, false),
        'S' => (Key::S, true, false),
        't' => (Key::T, false, false),
        'T' => (Key::T, true, false),
        'u' => (Key::U, false, false),
        'U' => (Key::U, true, false),
        'v' => (Key::V, false, false),
        'V' => (Key::V, true, false),
        'w' => (Key::W, false, false),
        'W' => (Key::W, true, false),
        'x' => (Key::X, false, false),
        'X' => (Key::X, true, false),
        'y' => (Key::Y, false, false),
        'Y' => (Key::Y, true, false),
        'z' => (Key::Z, false, false),
        'Z' => (Key::Z, true, false),

        'а' => (Key::A, false, true),
        'А' => (Key::A, true, true),
        'б' => (Key::B, false, true),
        'Б' => (Key::B, true, true),
        'в' => (Key::W, false, true),
        'В' => (Key::W, true, true),
        'г' => (Key::G, false, true),
        'Г' => (Key::G, true, true),
        'д' => (Key::D, false, true),
        'Д' => (Key::D, true, true),
        'е' | 'ё' => (Key::E, false, true),
        'Е' | 'Ё' => (Key::E, true, true),
        'ж' => (Key::V, false, true),
        'Ж' => (Key::V, true, true),
        'з' => (Key::Z, false, true),
        'З' => (Key::Z, true, true),
        'и' => (Key::I, false, true),
        'И' => (Key::I, true, true),
        'й' => (Key::J, false, true),
        'Й' => (Key::J, true, true),
        'к' => (Key::K, false, true),
        'К' => (Key::K, true, true),
        'л' => (Key::L, false, true),
        'Л' => (Key::L, true, true),
        'м' => (Key::M, false, true),
        'М' => (Key::M, true, true),
        'н' => (Key::N, false, true),
        'Н' => (Key::N, true, true),
        'о' => (Key::O, false, true),
        'О' => (Key::O, true, true),
        'п' => (Key::P, false, true),
        'П' => (Key::P, true, true),
        'р' => (Key::R, false, true),
        'Р' => (Key::R, true, true),
        'с' => (Key::S, false, true),
        'С' => (Key::S, true, true),
        'т' => (Key::T, false, true),
        'Т' => (Key::T, true, true),
        'у' => (Key::U, false, true),
        'У' => (Key::U, true, true),
        'ф' => (Key::F, false, true),
        'Ф' => (Key::F, true, true),
        'х' => (Key::H, false, true),
        'Х' => (Key::H, true, true),
        'ц' => (Key::C, false, true),
        'Ц' => (Key::C, true, true),
        'ч' => (Key::Caret, false, true),
        'Ч' => (Key::Caret, true, true),
        'ш' => (Key::BracketLeft, false, true),
        'Ш' => (Key::BracketLeft, true, true),
        'щ' => (Key::BracketRight, false, true),
        'Щ' => (Key::BracketRight, true, true),
        'ъ' => (Key::Backspace, false, true),
        'Ъ' => (Key::Backspace, true, true),
        'ы' => (Key::Y, false, true),
        'Ы' => (Key::Y, true, true),
        'ь' => (Key::X, false, true),
        'Ь' => (Key::X, true, true),
        'э' => (Key::Backslash, false, true),
        'Э' => (Key::Backslash, true, true),
        'ю' => (Key::At, false, true),
        'Ю' => (Key::At, true, true),
        'я' => (Key::Q, false, true),
        'Я' => (Key::Q, true, true),

        '0' => (Key::Num0, false, false),
        '1' => (Key::Num1, false, false),
        '2' => (Key::Num2, false, false),
        '3' => (Key::Num3, false, false),
        '4' => (Key::Num4, false, false),
        '5' => (Key::Num5, false, false),
        '6' => (Key::Num6, false, false),
        '7' => (Key::Num7, false, false),
        '8' => (Key::Num8, false, false),
        '9' => (Key::Num9, false, false),

        ' ' => (Key::Space, false, false),
        ';' => (Key::Semicolon, false, false),
        '+' => (Key::Semicolon, true, false),
        '-' => (Key::Minus, false, false),
        '=' => (Key::Minus, true, false),
        '_' => (Key::Backspace, true, false),
        ':' => (Key::Colon, false, false),
        '*' => (Key::Colon, true, false),
        '[' => (Key::BracketLeft, false, false),
        ']' => (Key::BracketRight, false, false),
        '\\' => (Key::Backslash, false, false),
        '|' => (Key::Backslash, true, false),
        '.' => (Key::Period, false, false),
        '>' => (Key::Period, true, false),
        '^' => (Key::Caret, false, false),
        '@' => (Key::At, true, false),
        ',' => (Key::Comma, false, false),
        '<' => (Key::Comma, true, false),
        '/' => (Key::Slash, false, false),
        '?' => (Key::Slash, true, false),

        '!' => (Key::Num1, true, false),
        '"' => (Key::Num2, true, false),
        '#' => (Key::Num3, true, false),
        '$' => (Key::Num4, true, false),
        '%' => (Key::Num5, true, false),
        '&' => (Key::Num6, true, false),
        '\'' => (Key::Num7, true, false),
        '(' => (Key::Num8, true, false),
        ')' => (Key::Num9, true, false),

        _ => return None,
    };

    if key != Key::Backspace {
        let key_u8 = key as u8;
        if (Key::A as u8..=Key::Z as u8).contains(&key_u8) {
            shift = lang;
        }

        if lang
            && matches!(
                key,
                Key::BracketLeft | Key::BracketRight | Key::Backslash | Key::Caret | Key::At
            )
        {
            shift = lang;
        } else if key == Key::At {
            shift = !shift;
        }
    }

    lang = false;

    Some((key, shift, lang))
}
