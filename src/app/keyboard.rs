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

use winit::keyboard::KeyCode;

pub fn map_keycode(keycode: KeyCode) -> Option<(usize, usize)> {
    match keycode {
        KeyCode::Home => Some((0, 0)),
        KeyCode::F12 | KeyCode::Delete | KeyCode::End => Some((0, 1)),
        KeyCode::Escape => Some((0, 2)),
        KeyCode::F1 => Some((0, 3)),
        KeyCode::F2 => Some((0, 4)),
        KeyCode::F3 => Some((0, 5)),
        KeyCode::F4 => Some((0, 6)),
        KeyCode::F5 => Some((0, 7)),
        KeyCode::Tab => Some((1, 0)),
        KeyCode::PageDown => Some((1, 1)),
        KeyCode::Enter | KeyCode::NumpadEnter => Some((1, 2)),
        KeyCode::Backspace => Some((1, 3)),
        KeyCode::ArrowLeft => Some((1, 4)),
        KeyCode::ArrowUp => Some((1, 5)),
        KeyCode::ArrowRight => Some((1, 6)),
        KeyCode::ArrowDown => Some((1, 7)),
        KeyCode::Digit0 | KeyCode::Numpad0 => Some((2, 0)),
        KeyCode::Digit1 | KeyCode::Numpad1 => Some((2, 1)),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some((2, 2)),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some((2, 3)),
        KeyCode::Digit4 | KeyCode::Numpad4 => Some((2, 4)),
        KeyCode::Digit5 | KeyCode::Numpad5 => Some((2, 5)),
        KeyCode::Digit6 | KeyCode::Numpad6 => Some((2, 6)),
        KeyCode::Digit7 | KeyCode::Numpad7 => Some((2, 7)),
        KeyCode::Digit8 | KeyCode::Numpad8 => Some((3, 0)),
        KeyCode::Digit9 | KeyCode::Numpad9 => Some((3, 1)),
        KeyCode::Equal => Some((3, 2)),
        KeyCode::Semicolon => Some((3, 3)),
        KeyCode::Comma | KeyCode::NumpadComma => Some((3, 4)),
        KeyCode::Minus | KeyCode::NumpadSubtract => Some((3, 5)),
        KeyCode::Period | KeyCode::NumpadDecimal => Some((3, 6)),
        KeyCode::Slash | KeyCode::NumpadDivide => Some((3, 7)),
        KeyCode::Backquote => Some((4, 0)),
        KeyCode::KeyA => Some((4, 1)),
        KeyCode::KeyB => Some((4, 2)),
        KeyCode::KeyC => Some((4, 3)),
        KeyCode::KeyD => Some((4, 4)),
        KeyCode::KeyE => Some((4, 5)),
        KeyCode::KeyF => Some((4, 6)),
        KeyCode::KeyG => Some((4, 7)),
        KeyCode::KeyH => Some((5, 0)),
        KeyCode::KeyI => Some((5, 1)),
        KeyCode::KeyJ => Some((5, 2)),
        KeyCode::KeyK => Some((5, 3)),
        KeyCode::KeyL => Some((5, 4)),
        KeyCode::KeyM => Some((5, 5)),
        KeyCode::KeyN => Some((5, 6)),
        KeyCode::KeyO => Some((5, 7)),
        KeyCode::KeyP => Some((6, 0)),
        KeyCode::KeyQ => Some((6, 1)),
        KeyCode::KeyR => Some((6, 2)),
        KeyCode::KeyS => Some((6, 3)),
        KeyCode::KeyT => Some((6, 4)),
        KeyCode::KeyU => Some((6, 5)),
        KeyCode::KeyV => Some((6, 6)),
        KeyCode::KeyW => Some((6, 7)),
        KeyCode::KeyX => Some((7, 0)),
        KeyCode::KeyY => Some((7, 1)),
        KeyCode::KeyZ => Some((7, 2)),
        KeyCode::BracketLeft => Some((7, 3)),
        KeyCode::Backslash => Some((7, 4)),
        KeyCode::BracketRight => Some((7, 5)),
        KeyCode::Quote => Some((7, 6)),
        KeyCode::Space => Some((7, 7)),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Some((8, 5)),
        KeyCode::ControlLeft | KeyCode::ControlRight => Some((8, 6)),
        KeyCode::Insert | KeyCode::CapsLock | KeyCode::AltLeft | KeyCode::AltRight => Some((8, 7)),
        _ => None,
    }
}
