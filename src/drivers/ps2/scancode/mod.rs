pub mod set2;

use crate::interfaces::input::KeyEvent;

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
