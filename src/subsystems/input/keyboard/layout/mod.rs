pub mod us_qwerty;

use super::{KeyCode, LockState, Modifiers};

pub trait KeyboardLayout {
    fn translate(&self, code: KeyCode, modifiers: Modifiers, locks: LockState) -> Option<char>;
}
