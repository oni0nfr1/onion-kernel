use super::{KeyCode, KeyEvent, KeyState, ScanCodeDecoder, ScanCodeSet};

const RELEASE_PREFIX: u8 = 0xf0;
const EXTENDED_PREFIX: u8 = 0xe0;
const EXTENDED_SEQUENCE_PREFIX: u8 = 0xe1;

const PRINT_SCREEN_PRESSED: [u8; 4] = [EXTENDED_PREFIX, 0x12, EXTENDED_PREFIX, 0x7c];
const PRINT_SCREEN_RELEASED: [u8; 6] = [
    EXTENDED_PREFIX,
    RELEASE_PREFIX,
    0x7c,
    EXTENDED_PREFIX,
    RELEASE_PREFIX,
    0x12,
];
const PAUSE_PRESSED: [u8; 8] = [
    EXTENDED_SEQUENCE_PREFIX,
    0x14,
    0x77,
    EXTENDED_SEQUENCE_PREFIX,
    RELEASE_PREFIX,
    0x14,
    RELEASE_PREFIX,
    0x77,
];

const MAX_SEQUENCE_LENGTH: usize = PAUSE_PRESSED.len();

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Set2DecodeError {
    UnexpectedByte(u8),
    UnknownScanCode { scan_code: u8, extended: bool },
}

/// A stateful decoder for untranslated PS/2 Scan Code Set 2 bytes.
pub struct Set2Decoder {
    sequence: [u8; MAX_SEQUENCE_LENGTH],
    sequence_length: usize,
}

impl Set2Decoder {
    pub const fn new() -> Self {
        Self {
            sequence: [0; MAX_SEQUENCE_LENGTH],
            sequence_length: 0,
        }
    }

    fn push_sequence_byte(&mut self, byte: u8) {
        debug_assert!(
            self.sequence_length < MAX_SEQUENCE_LENGTH,
            "Set 2 scan-code sequence overflow: length={}, capacity={}, next byte={byte:#04x}",
            self.sequence_length,
            MAX_SEQUENCE_LENGTH,
        );
        self.sequence[self.sequence_length] = byte;
        self.sequence_length += 1;
    }

    fn finish_key(
        &mut self,
        scan_code: u8,
        extended: bool,
        state: KeyState,
    ) -> Result<Option<KeyEvent>, Set2DecodeError> {
        self.reset();

        let code = decode_key(scan_code, extended).ok_or(Set2DecodeError::UnknownScanCode {
            scan_code,
            extended,
        })?;

        Ok(Some(KeyEvent { code, state }))
    }

    fn finish_special(&mut self, code: KeyCode, state: KeyState) -> Option<KeyEvent> {
        self.reset();
        Some(KeyEvent { code, state })
    }

    fn fail(&mut self, byte: u8) -> Result<Option<KeyEvent>, Set2DecodeError> {
        self.reset();
        Err(Set2DecodeError::UnexpectedByte(byte))
    }
}

impl Default for Set2Decoder {
    fn default() -> Self {
        Self::new()
    }
}

impl ScanCodeDecoder for Set2Decoder {
    type Error = Set2DecodeError;

    const SCAN_CODE_SET: ScanCodeSet = ScanCodeSet::Set2;

    fn push_byte(&mut self, byte: u8) -> Result<Option<KeyEvent>, Self::Error> {
        if self.sequence_length == MAX_SEQUENCE_LENGTH {
            return self.fail(byte);
        }

        self.push_sequence_byte(byte);

        let sequence_length = self.sequence_length;
        let sequence = &self.sequence[..sequence_length];

        if PRINT_SCREEN_PRESSED.starts_with(sequence) {
            if sequence_length == PRINT_SCREEN_PRESSED.len() {
                return Ok(self.finish_special(KeyCode::PrintScreen, KeyState::Pressed));
            }
            return Ok(None);
        }

        if PRINT_SCREEN_RELEASED.starts_with(sequence) {
            if sequence_length == PRINT_SCREEN_RELEASED.len() {
                return Ok(self.finish_special(KeyCode::PrintScreen, KeyState::Released));
            }
            return Ok(None);
        }

        if PAUSE_PRESSED.starts_with(sequence) {
            if sequence_length == PAUSE_PRESSED.len() {
                return Ok(self.finish_special(KeyCode::Pause, KeyState::Pressed));
            }
            return Ok(None);
        }

        match sequence {
            [RELEASE_PREFIX] => Ok(None),
            [RELEASE_PREFIX, scan_code] => self.finish_key(*scan_code, false, KeyState::Released),
            [EXTENDED_PREFIX, scan_code] => self.finish_key(*scan_code, true, KeyState::Pressed),
            [EXTENDED_PREFIX, RELEASE_PREFIX, scan_code] => {
                self.finish_key(*scan_code, true, KeyState::Released)
            }
            [scan_code] => self.finish_key(*scan_code, false, KeyState::Pressed),
            _ => self.fail(byte),
        }
    }

    fn reset(&mut self) {
        self.sequence_length = 0;
    }
}

fn decode_key(scan_code: u8, extended: bool) -> Option<KeyCode> {
    if extended {
        decode_extended_key(scan_code)
    } else {
        decode_standard_key(scan_code)
    }
}

fn decode_standard_key(scan_code: u8) -> Option<KeyCode> {
    Some(match scan_code {
        0x76 => KeyCode::Escape,
        0x05 => KeyCode::F1,
        0x06 => KeyCode::F2,
        0x04 => KeyCode::F3,
        0x0c => KeyCode::F4,
        0x03 => KeyCode::F5,
        0x0b => KeyCode::F6,
        0x83 => KeyCode::F7,
        0x0a => KeyCode::F8,
        0x01 => KeyCode::F9,
        0x09 => KeyCode::F10,
        0x78 => KeyCode::F11,
        0x07 => KeyCode::F12,
        0x7e => KeyCode::ScrollLock,

        0x0e => KeyCode::Backquote,
        0x16 => KeyCode::Digit1,
        0x1e => KeyCode::Digit2,
        0x26 => KeyCode::Digit3,
        0x25 => KeyCode::Digit4,
        0x2e => KeyCode::Digit5,
        0x36 => KeyCode::Digit6,
        0x3d => KeyCode::Digit7,
        0x3e => KeyCode::Digit8,
        0x46 => KeyCode::Digit9,
        0x45 => KeyCode::Digit0,
        0x4e => KeyCode::Minus,
        0x55 => KeyCode::Equal,
        0x66 => KeyCode::Backspace,

        0x0d => KeyCode::Tab,
        0x15 => KeyCode::Q,
        0x1d => KeyCode::W,
        0x24 => KeyCode::E,
        0x2d => KeyCode::R,
        0x2c => KeyCode::T,
        0x35 => KeyCode::Y,
        0x3c => KeyCode::U,
        0x43 => KeyCode::I,
        0x44 => KeyCode::O,
        0x4d => KeyCode::P,
        0x54 => KeyCode::LeftBracket,
        0x5b => KeyCode::RightBracket,
        0x5d => KeyCode::Backslash,

        0x58 => KeyCode::CapsLock,
        0x1c => KeyCode::A,
        0x1b => KeyCode::S,
        0x23 => KeyCode::D,
        0x2b => KeyCode::F,
        0x34 => KeyCode::G,
        0x33 => KeyCode::H,
        0x3b => KeyCode::J,
        0x42 => KeyCode::K,
        0x4b => KeyCode::L,
        0x4c => KeyCode::Semicolon,
        0x52 => KeyCode::Quote,
        0x5a => KeyCode::Enter,

        0x12 => KeyCode::LeftShift,
        0x1a => KeyCode::Z,
        0x22 => KeyCode::X,
        0x21 => KeyCode::C,
        0x2a => KeyCode::V,
        0x32 => KeyCode::B,
        0x31 => KeyCode::N,
        0x3a => KeyCode::M,
        0x41 => KeyCode::Comma,
        0x49 => KeyCode::Period,
        0x4a => KeyCode::Slash,
        0x59 => KeyCode::RightShift,

        0x14 => KeyCode::LeftControl,
        0x11 => KeyCode::LeftAlt,
        0x29 => KeyCode::Space,

        0x77 => KeyCode::NumLock,
        0x7c => KeyCode::NumpadMultiply,
        0x7b => KeyCode::NumpadSubtract,
        0x6c => KeyCode::Numpad7,
        0x75 => KeyCode::Numpad8,
        0x7d => KeyCode::Numpad9,
        0x79 => KeyCode::NumpadAdd,
        0x6b => KeyCode::Numpad4,
        0x73 => KeyCode::Numpad5,
        0x74 => KeyCode::Numpad6,
        0x69 => KeyCode::Numpad1,
        0x72 => KeyCode::Numpad2,
        0x7a => KeyCode::Numpad3,
        0x70 => KeyCode::Numpad0,
        0x71 => KeyCode::NumpadDecimal,

        _ => return None,
    })
}

fn decode_extended_key(scan_code: u8) -> Option<KeyCode> {
    Some(match scan_code {
        0x11 => KeyCode::RightAlt,
        0x14 => KeyCode::RightControl,
        0x1f => KeyCode::LeftMeta,
        0x27 => KeyCode::RightMeta,
        0x2f => KeyCode::Menu,

        0x70 => KeyCode::Insert,
        0x6c => KeyCode::Home,
        0x7d => KeyCode::PageUp,
        0x71 => KeyCode::Delete,
        0x69 => KeyCode::End,
        0x7a => KeyCode::PageDown,
        0x75 => KeyCode::ArrowUp,
        0x6b => KeyCode::ArrowLeft,
        0x72 => KeyCode::ArrowDown,
        0x74 => KeyCode::ArrowRight,

        0x4a => KeyCode::NumpadDivide,
        0x5a => KeyCode::NumpadEnter,

        _ => return None,
    })
}
