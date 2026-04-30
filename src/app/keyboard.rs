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
use winit::keyboard::KeyCode;

pub fn map_keycode(keycode: KeyCode) -> Option<Key> {
    match keycode {
        KeyCode::Home => Some(Key::Home),
        KeyCode::F12 | KeyCode::Delete | KeyCode::End => Some(Key::End),
        KeyCode::Escape => Some(Key::Escape),
        KeyCode::F1 => Some(Key::F1),
        KeyCode::F2 => Some(Key::F2),
        KeyCode::F3 => Some(Key::F3),
        KeyCode::F4 => Some(Key::F4),
        KeyCode::F5 => Some(Key::F5),
        KeyCode::Tab => Some(Key::Tab),
        KeyCode::PageDown => Some(Key::PageDown),
        KeyCode::Enter | KeyCode::NumpadEnter => Some(Key::Enter),
        KeyCode::Backspace => Some(Key::Backspace),
        KeyCode::ArrowLeft => Some(Key::Left),
        KeyCode::ArrowUp => Some(Key::Up),
        KeyCode::ArrowRight => Some(Key::Right),
        KeyCode::ArrowDown => Some(Key::Down),
        KeyCode::Digit0 | KeyCode::Numpad0 => Some(Key::Num0),
        KeyCode::Digit1 | KeyCode::Numpad1 => Some(Key::Num1),
        KeyCode::Digit2 | KeyCode::Numpad2 => Some(Key::Num2),
        KeyCode::Digit3 | KeyCode::Numpad3 => Some(Key::Num3),
        KeyCode::Digit4 | KeyCode::Numpad4 => Some(Key::Num4),
        KeyCode::Digit5 | KeyCode::Numpad5 => Some(Key::Num5),
        KeyCode::Digit6 | KeyCode::Numpad6 => Some(Key::Num6),
        KeyCode::Digit7 | KeyCode::Numpad7 => Some(Key::Num7),
        KeyCode::Digit8 | KeyCode::Numpad8 => Some(Key::Num8),
        KeyCode::Digit9 | KeyCode::Numpad9 => Some(Key::Num9),
        KeyCode::Equal => Some(Key::Equal),
        KeyCode::Semicolon => Some(Key::Semicolon),
        KeyCode::Comma | KeyCode::NumpadComma => Some(Key::Comma),
        KeyCode::Minus | KeyCode::NumpadSubtract => Some(Key::Minus),
        KeyCode::Period | KeyCode::NumpadDecimal => Some(Key::Period),
        KeyCode::Slash | KeyCode::NumpadDivide => Some(Key::Slash),
        KeyCode::Backquote => Some(Key::Backquote),
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
        KeyCode::Backslash => Some(Key::Backslash),
        KeyCode::BracketRight => Some(Key::BracketRight),
        KeyCode::Quote => Some(Key::Quote),
        KeyCode::Space => Some(Key::Space),
        KeyCode::ShiftLeft | KeyCode::ShiftRight => Some(Key::Shift),
        KeyCode::ControlLeft | KeyCode::ControlRight => Some(Key::Ctrl),
        KeyCode::Insert | KeyCode::CapsLock | KeyCode::AltLeft | KeyCode::AltRight => {
            Some(Key::Alt)
        }
        _ => None,
    }
}
