use serde_derive::{Deserialize, Serialize};
use winit::keyboard::NamedKey;

/// Symbolic name for a keyboard key.
#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum KeyCode {
    /// The '1' key over the letters.
    Key1,
    /// The '2' key over the letters.
    Key2,
    /// The '3' key over the letters.
    Key3,
    /// The '4' key over the letters.
    Key4,
    /// The '5' key over the letters.
    Key5,
    /// The '6' key over the letters.
    Key6,
    /// The '7' key over the letters.
    Key7,
    /// The '8' key over the letters.
    Key8,
    /// The '9' key over the letters.
    Key9,
    /// The '0' key over the 'O' and 'P' keys.
    Key0,

    A,
    B,
    C,
    D,
    E,
    F,
    G,
    H,
    I,
    J,
    K,
    L,
    M,
    N,
    O,
    P,
    Q,
    R,
    S,
    T,
    U,
    V,
    W,
    X,
    Y,
    Z,

    /// The Escape key, next to F1.
    Escape,

    F1,
    F2,
    F3,
    F4,
    F5,
    F6,
    F7,
    F8,
    F9,
    F10,
    F11,
    F12,
    F13,
    F14,
    F15,
    F16,
    F17,
    F18,
    F19,
    F20,
    F21,
    F22,
    F23,
    F24,

    /// Print Screen/SysRq.
    Snapshot,
    /// Scroll Lock.
    Scroll,
    /// Pause/Break key, next to Scroll lock.
    Pause,

    /// `Insert`, next to Backspace.
    Insert,
    Home,
    Delete,
    End,
    PageDown,
    PageUp,

    Left,
    Up,
    Right,
    Down,

    /// The Backspace key, right over Enter.
    // TODO: rename
    Back,
    /// The Enter key.
    Return,
    /// The space bar.
    Space,

    /// The "Compose" key on Linux.
    Compose,

    /// Caret is a legacy name for the `^` character.
    Caret,

    Numlock,
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
    NumpadDivide,
    NumpadDecimal,
    NumpadComma,
    NumpadEnter,
    NumpadEquals,
    NumpadMultiply,
    NumpadSubtract,

    AbntC1,
    AbntC2,
    Apostrophe,
    Apps,
    Asterisk,
    At,
    Ax,
    Backslash,
    Calculator,
    Capital,
    Colon,
    Comma,
    Convert,
    Equals,
    Grave,
    Kana,
    Kanji,
    LAlt,
    LBracket,
    LControl,
    LShift,
    LWin,
    Mail,
    MediaSelect,
    MediaStop,
    Minus,
    Mute,
    MyComputer,
    // also called "Next"
    NavigateForward,
    // also called "Prior"
    NavigateBackward,
    NextTrack,
    NoConvert,
    OEM102,
    Period,
    PlayPause,
    Plus,
    Power,
    PrevTrack,
    RAlt,
    RBracket,
    RControl,
    RShift,
    RWin,
    Semicolon,
    Slash,
    Sleep,
    Stop,
    Sysrq,
    Tab,
    Underline,
    Unlabeled,
    VolumeDown,
    VolumeUp,
    Wake,
    WebBack,
    WebFavorites,
    WebForward,
    WebHome,
    WebRefresh,
    WebSearch,
    WebStop,
    Yen,
    Copy,
    Paste,
    Cut,

    Unknown,
}

impl KeyCode {
    pub fn from(from: &winit::keyboard::Key) -> Self {
        match from {
            winit::keyboard::Key::Named(named) => {
                match named {
                    // winit で named として定義されたキーは多いが複数ストロークの要素として利用したいキーは限定されるため、一部のみ実装する。
                    NamedKey::Alt => KeyCode::LAlt,
                    NamedKey::Shift => KeyCode::LShift,
                    NamedKey::Control => KeyCode::LControl,
                    NamedKey::Enter => KeyCode::Return,
                    NamedKey::Tab => KeyCode::Tab,
                    NamedKey::Space => KeyCode::Space,
                    NamedKey::ArrowDown => KeyCode::Down,
                    NamedKey::ArrowLeft => KeyCode::Left,
                    NamedKey::ArrowRight => KeyCode::Right,
                    NamedKey::ArrowUp => KeyCode::Up,
                    NamedKey::End => KeyCode::End,
                    NamedKey::Home => KeyCode::Home,
                    NamedKey::PageDown => KeyCode::PageDown,
                    NamedKey::PageUp => KeyCode::PageUp,
                    NamedKey::Backspace => KeyCode::Back,
                    NamedKey::Delete => KeyCode::Delete,
                    NamedKey::Insert => KeyCode::Insert,
                    NamedKey::Escape => KeyCode::Escape,
                    NamedKey::F1 => KeyCode::F1,
                    NamedKey::F2 => KeyCode::F2,
                    NamedKey::F3 => KeyCode::F3,
                    NamedKey::F4 => KeyCode::F4,
                    NamedKey::F5 => KeyCode::F5,
                    NamedKey::F6 => KeyCode::F6,
                    NamedKey::F7 => KeyCode::F7,
                    NamedKey::F8 => KeyCode::F8,
                    NamedKey::F9 => KeyCode::F9,
                    NamedKey::F10 => KeyCode::F10,
                    NamedKey::F11 => KeyCode::F11,
                    NamedKey::F12 => KeyCode::F12,
                    _ => KeyCode::Unknown,
                }
            }
            winit::keyboard::Key::Character(char) => match char.to_ascii_uppercase().as_str() {
                "1" => KeyCode::Key1,
                "2" => KeyCode::Key2,
                "3" => KeyCode::Key3,
                "4" => KeyCode::Key4,
                "5" => KeyCode::Key5,
                "6" => KeyCode::Key6,
                "7" => KeyCode::Key7,
                "8" => KeyCode::Key8,
                "9" => KeyCode::Key9,
                "0" => KeyCode::Key0,
                "A" => KeyCode::A,
                "B" => KeyCode::B,
                "C" => KeyCode::C,
                "D" => KeyCode::D,
                "E" => KeyCode::E,
                "F" => KeyCode::F,
                "G" => KeyCode::G,
                "H" => KeyCode::H,
                "I" => KeyCode::I,
                "J" => KeyCode::J,
                "K" => KeyCode::K,
                "L" => KeyCode::L,
                "M" => KeyCode::M,
                "N" => KeyCode::N,
                "O" => KeyCode::O,
                "P" => KeyCode::P,
                "Q" => KeyCode::Q,
                "R" => KeyCode::R,
                "S" => KeyCode::S,
                "T" => KeyCode::T,
                "U" => KeyCode::U,
                "V" => KeyCode::V,
                "W" => KeyCode::W,
                "X" => KeyCode::X,
                "Y" => KeyCode::Y,
                "Z" => KeyCode::Z,
                "-" => KeyCode::Minus,
                "^" => KeyCode::Caret,
                // Logical Key だけでは IntlYen と IntlRo が区別できないため円とバックスラッシュは区別できない。
                "\\" => KeyCode::Backslash,
                "@" => KeyCode::At,
                ";" => KeyCode::Semicolon,
                ":" => KeyCode::Colon,
                "[" => KeyCode::LBracket,
                "]" => KeyCode::RBracket,
                "," => KeyCode::Comma,
                "." => KeyCode::Period,
                // < と > は日本語キーボードの深遠な理由から Comma, Period として扱う。
                // 以下、深遠な理由。
                // 日本語キーボードでは < は Shift + , で入力されるため、Comma として扱う。
                // 日本語キーボードでは > は Shift + . で入力されるため、Period として扱う。
                "<" => KeyCode::Comma,
                ">" => KeyCode::Period,
                "/" => KeyCode::Slash,
                _ => KeyCode::Unknown,
            },
            winit::keyboard::Key::Unidentified(_) => KeyCode::Unknown,
            winit::keyboard::Key::Dead(_) => KeyCode::Unknown,
        }
    }
}

#[derive(Debug, Hash, Ord, PartialOrd, PartialEq, Eq, Clone, Copy, Serialize, Deserialize)]
pub enum ModifiersState {
    CtrlAltShift,
    CtrlAlt,
    CtrlShift,
    AltShift,
    Ctrl,
    Alt,
    Shift,
    NONE,
}

impl ModifiersState {
    pub fn from(from: winit::event::Modifiers) -> Self {
        let on_ctrl = from.state().control_key();
        let on_alt = from.state().alt_key();
        let on_shift = from.state().shift_key();

        if on_ctrl && on_alt && on_shift {
            ModifiersState::CtrlAltShift
        } else if on_ctrl && on_alt {
            ModifiersState::CtrlAlt
        } else if on_ctrl && on_shift {
            ModifiersState::CtrlShift
        } else if on_alt && on_shift {
            ModifiersState::AltShift
        } else if on_ctrl {
            ModifiersState::Ctrl
        } else if on_alt {
            ModifiersState::Alt
        } else if on_shift {
            ModifiersState::Shift
        } else {
            ModifiersState::NONE
        }
    }
}
