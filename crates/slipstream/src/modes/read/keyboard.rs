//! Keyboard binding mappings.

use super::*;

pub use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

pub const QUIT: KeyEvent =
    KeyEvent::new(KeyCode::Char('q'), KeyModifiers::NONE);
pub const UPDATE: KeyEvent =
    KeyEvent::new(KeyCode::Char('u'), KeyModifiers::NONE);
pub const DOWN: KeyEvent =
    KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
pub const UP: KeyEvent = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
pub const LEFT: KeyEvent =
    KeyEvent::new(KeyCode::Char('h'), KeyModifiers::NONE);
pub const RIGHT: KeyEvent =
    KeyEvent::new(KeyCode::Char('l'), KeyModifiers::NONE);
pub const PAGE_DOWN: KeyEvent =
    KeyEvent::new(KeyCode::Char('j'), KeyModifiers::SHIFT);
pub const PAGE_UP: KeyEvent =
    KeyEvent::new(KeyCode::Char('k'), KeyModifiers::SHIFT);
pub const MENU: KeyEvent = KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE);
pub const TAB: KeyEvent = KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE);
pub const COMMAND_MODE: KeyEvent =
    KeyEvent::new(KeyCode::Char(':'), KeyModifiers::NONE);
pub const SEARCH_MODE: KeyEvent =
    KeyEvent::new(KeyCode::Char('/'), KeyModifiers::NONE);

/// Keyboard key.
#[derive(Clone, Debug, Serialize, PartialEq, Eq, PartialOrd, Hash)]
pub struct BindingKey {
    /// Underlying crossterm key event.
    key: KeyEvent,
}

impl From<BindingKey> for KeyEvent {
    fn from(value: BindingKey) -> Self {
        value.key
    }
}

impl From<&BindingKey> for KeyEvent {
    fn from(value: &BindingKey) -> Self {
        value.key
    }
}

impl<'de> Deserialize<'de> for BindingKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let mut text = String::deserialize(deserializer)?;

        // Parse potential modifier.
        let mut modifier = KeyModifiers::NONE;
        if text.starts_with("C-") {
            modifier = KeyModifiers::CONTROL;
            text = text[2..].into();
        }
        if text.starts_with("S-") {
            modifier = KeyModifiers::SHIFT;
            text = text[2..].into();
        }
        if text.starts_with("A-") {
            modifier = KeyModifiers::ALT;
            text = text[2..].into();
        }

        // Get keycode.
        let mut code: KeyCode = match text.as_str() {
            "a" => KeyCode::Char('a'),
            "b" => KeyCode::Char('b'),
            "c" => KeyCode::Char('c'),
            "d" => KeyCode::Char('d'),
            "e" => KeyCode::Char('e'),
            "f" => KeyCode::Char('f'),
            "g" => KeyCode::Char('g'),
            "h" => KeyCode::Char('h'),
            "i" => KeyCode::Char('i'),
            "l" => KeyCode::Char('l'),
            "m" => KeyCode::Char('m'),
            "n" => KeyCode::Char('n'),
            "o" => KeyCode::Char('o'),
            "p" => KeyCode::Char('p'),
            "q" => KeyCode::Char('q'),
            "r" => KeyCode::Char('r'),
            "s" => KeyCode::Char('s'),
            "t" => KeyCode::Char('t'),
            "u" => KeyCode::Char('u'),
            "v" => KeyCode::Char('v'),
            "w" => KeyCode::Char('w'),
            "x" => KeyCode::Char('x'),
            "y" => KeyCode::Char('y'),
            "z" => KeyCode::Char('z'),
            "1" => KeyCode::Char('1'),
            "2" => KeyCode::Char('2'),
            "3" => KeyCode::Char('3'),
            "4" => KeyCode::Char('4'),
            "5" => KeyCode::Char('5'),
            "6" => KeyCode::Char('6'),
            "7" => KeyCode::Char('7'),
            "8" => KeyCode::Char('8'),
            "9" => KeyCode::Char('9'),
            "0" => KeyCode::Char('0'),
            "tilde" => KeyCode::Char('`'),
            "tab" => KeyCode::Tab,
            "capslock" => KeyCode::CapsLock,
            "enter" => KeyCode::Enter,
            "space" => KeyCode::Char(' '),
            "pageup" => KeyCode::PageUp,
            "pagedown" => KeyCode::PageDown,
            _ => {
                use serde::de::Error;
                return Err(D::Error::unknown_variant(
                    text.as_str(),
                    &[
                        "lowercase alphanumeric",
                        "tilde",
                        "tab",
                        "capslock",
                        "enter",
                        "space",
                        "pageup",
                        "pagedown",
                    ],
                ));
            }
        };

        // Adjust code for shift modifier.
        if modifier == KeyModifiers::SHIFT {
            code = match text.as_str() {
                "a" => KeyCode::Char('A'),
                "b" => KeyCode::Char('B'),
                "c" => KeyCode::Char('C'),
                "d" => KeyCode::Char('D'),
                "e" => KeyCode::Char('E'),
                "f" => KeyCode::Char('F'),
                "g" => KeyCode::Char('G'),
                "h" => KeyCode::Char('H'),
                "i" => KeyCode::Char('I'),
                "l" => KeyCode::Char('L'),
                "m" => KeyCode::Char('M'),
                "n" => KeyCode::Char('N'),
                "o" => KeyCode::Char('O'),
                "p" => KeyCode::Char('P'),
                "q" => KeyCode::Char('Q'),
                "r" => KeyCode::Char('R'),
                "s" => KeyCode::Char('S'),
                "t" => KeyCode::Char('T'),
                "u" => KeyCode::Char('U'),
                "v" => KeyCode::Char('V'),
                "w" => KeyCode::Char('W'),
                "x" => KeyCode::Char('X'),
                "y" => KeyCode::Char('Y'),
                "z" => KeyCode::Char('Z'),
                "1" => KeyCode::Char('!'),
                "2" => KeyCode::Char('@'),
                "3" => KeyCode::Char('#'),
                "4" => KeyCode::Char('$'),
                "5" => KeyCode::Char('%'),
                "6" => KeyCode::Char('^'),
                "7" => KeyCode::Char('&'),
                "8" => KeyCode::Char('*'),
                "9" => KeyCode::Char('('),
                "0" => KeyCode::Char(')'),
                _ => code,
            }
        }

        Ok(Self {
            key: KeyEvent::new(code, modifier),
        })
    }
}
