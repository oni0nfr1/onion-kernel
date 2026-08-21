pub mod set2;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyState {
    Pressed,
    Released,
}

/// A physical key independent of a keyboard layout and modifier state.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KeyEvent {
    pub code: KeyCode,
    pub state: KeyState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(u8)]
pub enum ScanCodeSet {
    Set1 = 1,
    Set2 = 2,
    Set3 = 3,
}

impl ScanCodeSet {
    pub const fn as_u8(self) -> u8 {
        self as u8
    }
}

/// Incrementally decodes a stream of keyboard scan-code bytes.
///
/// Prefixes and other incomplete sequences produce `Ok(None)`. A completed
/// physical key transition produces `Ok(Some(_))`.
pub trait ScanCodeDecoder {
    type Error;

    /// The scan-code set that the keyboard must be configured to emit.
    const SCAN_CODE_SET: ScanCodeSet;

    fn push_byte(&mut self, byte: u8) -> Result<Option<KeyEvent>, Self::Error>;

    /// Discards any incomplete scan-code sequence held by the decoder.
    ///
    /// This should be called after a controller read error or any other event
    /// that may have dropped a byte from the scan-code stream.
    fn reset(&mut self);
}
