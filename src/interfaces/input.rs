//! Transport-neutral physical input and device-feedback types.

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

/// A physical key independent of a keyboard layout and transport.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum KeyCode {
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
    PrintScreen,
    ScrollLock,
    Pause,

    Backquote,
    Digit1,
    Digit2,
    Digit3,
    Digit4,
    Digit5,
    Digit6,
    Digit7,
    Digit8,
    Digit9,
    Digit0,
    Minus,
    Equal,
    Backspace,

    Tab,
    Q,
    W,
    E,
    R,
    T,
    Y,
    U,
    I,
    O,
    P,
    LeftBracket,
    RightBracket,
    Backslash,

    CapsLock,
    A,
    S,
    D,
    F,
    G,
    H,
    J,
    K,
    L,
    Semicolon,
    Quote,
    Enter,

    LeftShift,
    Z,
    X,
    C,
    V,
    B,
    N,
    M,
    Comma,
    Period,
    Slash,
    RightShift,

    LeftControl,
    LeftMeta,
    LeftAlt,
    Space,
    RightAlt,
    RightMeta,
    Menu,
    RightControl,

    Insert,
    Home,
    PageUp,
    Delete,
    End,
    PageDown,
    ArrowUp,
    ArrowLeft,
    ArrowDown,
    ArrowRight,

    NumLock,
    NumpadDivide,
    NumpadMultiply,
    NumpadSubtract,
    Numpad7,
    Numpad8,
    Numpad9,
    NumpadAdd,
    Numpad4,
    Numpad5,
    Numpad6,
    Numpad1,
    Numpad2,
    Numpad3,
    NumpadEnter,
    Numpad0,
    NumpadDecimal,
}

impl KeyCode {
    pub(crate) const COUNT: usize = Self::NumpadDecimal as usize + 1;

    pub(crate) const fn index(self) -> usize {
        self as usize
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub state: KeyState,
}

/// Indicator state requested from a physical keyboard device.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct KeyboardLeds {
    pub caps_lock: bool,
    pub num_lock: bool,
    pub scroll_lock: bool,
}
