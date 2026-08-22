pub mod layout;

pub use crate::interfaces::input::{KeyCode, KeyEvent, KeyState, KeyboardLeds};
pub use layout::{KeyboardLayout, us_qwerty::UsQwerty};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum KeyAction {
    Pressed,
    Repeated,
    Released,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Modifiers {
    pub left_shift: bool,
    pub right_shift: bool,
    pub left_control: bool,
    pub right_control: bool,
    pub left_alt: bool,
    pub right_alt: bool,
    pub left_meta: bool,
    pub right_meta: bool,
}

impl Modifiers {
    pub const fn shift(self) -> bool {
        self.left_shift || self.right_shift
    }

    pub const fn control(self) -> bool {
        self.left_control || self.right_control
    }

    pub const fn alt(self) -> bool {
        self.left_alt || self.right_alt
    }

    pub const fn alt_graph(self) -> bool {
        self.right_alt
    }

    pub const fn meta(self) -> bool {
        self.left_meta || self.right_meta
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LockState {
    pub caps_lock: bool,
    pub num_lock: bool,
    pub scroll_lock: bool,
}

impl From<LockState> for KeyboardLeds {
    fn from(locks: LockState) -> Self {
        Self {
            caps_lock: locks.caps_lock,
            num_lock: locks.num_lock,
            scroll_lock: locks.scroll_lock,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum KeyboardFeedback {
    SetLeds(KeyboardLeds),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LogicalKeyEvent {
    pub code: KeyCode,
    pub action: KeyAction,
    pub modifiers: Modifiers,
    pub locks: LockState,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub struct KeyboardOutput {
    pub key: LogicalKeyEvent,
    pub text: Option<char>,
    pub feedback: Option<KeyboardFeedback>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum LockKeyCycle {
    #[default]
    Idle,
    Locking,
    Unlocking,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct LockKeyCycles {
    caps_lock: LockKeyCycle,
    num_lock: LockKeyCycle,
    scroll_lock: LockKeyCycle,
}

/// Converts physical key transitions into layout-aware keyboard input.
///
/// Lock keys follow the traditional XKB-compatible behavior: locking occurs
/// on the first press, while unlocking occurs when the second press is
/// released.
pub struct Keyboard<L>
where
    L: KeyboardLayout,
{
    layout: L,
    pressed: [bool; KeyCode::COUNT],
    locks: LockState,
    lock_cycles: LockKeyCycles,
}

pub type UsQwertyKeyboard = Keyboard<UsQwerty>;

impl<L> Keyboard<L>
where
    L: KeyboardLayout,
{
    pub const fn new(layout: L) -> Self {
        Self {
            layout,
            pressed: [false; KeyCode::COUNT],
            locks: LockState {
                caps_lock: false,
                num_lock: false,
                scroll_lock: false,
            },
            lock_cycles: LockKeyCycles {
                caps_lock: LockKeyCycle::Idle,
                num_lock: LockKeyCycle::Idle,
                scroll_lock: LockKeyCycle::Idle,
            },
        }
    }

    pub const fn layout(&self) -> &L {
        &self.layout
    }

    pub fn layout_mut(&mut self) -> &mut L {
        &mut self.layout
    }

    pub fn into_layout(self) -> L {
        self.layout
    }

    pub const fn is_pressed(&self, code: KeyCode) -> bool {
        self.pressed[code.index()]
    }

    pub fn modifiers(&self) -> Modifiers {
        Modifiers {
            left_shift: self.is_pressed(KeyCode::LeftShift),
            right_shift: self.is_pressed(KeyCode::RightShift),
            left_control: self.is_pressed(KeyCode::LeftControl),
            right_control: self.is_pressed(KeyCode::RightControl),
            left_alt: self.is_pressed(KeyCode::LeftAlt),
            right_alt: self.is_pressed(KeyCode::RightAlt),
            left_meta: self.is_pressed(KeyCode::LeftMeta),
            right_meta: self.is_pressed(KeyCode::RightMeta),
        }
    }

    pub const fn locks(&self) -> LockState {
        self.locks
    }

    /// Clears physically held keys and incomplete lock-key cycles while
    /// preserving the logical lock state.
    pub fn reset_pressed_state(&mut self) {
        self.pressed.fill(false);
        self.lock_cycles = LockKeyCycles::default();
    }

    pub fn handle_key_event(&mut self, event: KeyEvent) -> KeyboardOutput {
        let was_pressed = self.is_pressed(event.code);
        let action = match event.state {
            KeyState::Pressed if was_pressed => KeyAction::Repeated,
            KeyState::Pressed => KeyAction::Pressed,
            KeyState::Released => KeyAction::Released,
        };

        let feedback = match action {
            KeyAction::Pressed => {
                // Pause has no break code in Scan Code Set 2 and is therefore
                // represented as an instantaneous press rather than a held key.
                if event.code != KeyCode::Pause {
                    self.pressed[event.code.index()] = true;
                }
                self.handle_lock_pressed(event.code)
            }
            KeyAction::Repeated => None,
            KeyAction::Released => {
                self.pressed[event.code.index()] = false;
                if was_pressed {
                    self.handle_lock_released(event.code)
                } else {
                    None
                }
            }
        };

        let modifiers = self.modifiers();
        let text = if matches!(action, KeyAction::Pressed | KeyAction::Repeated) {
            self.layout.translate(event.code, modifiers, self.locks)
        } else {
            None
        };

        KeyboardOutput {
            key: LogicalKeyEvent {
                code: event.code,
                action,
                modifiers,
                locks: self.locks,
            },
            text,
            feedback,
        }
    }

    fn handle_lock_pressed(&mut self, code: KeyCode) -> Option<KeyboardFeedback> {
        let changed = match code {
            KeyCode::CapsLock => {
                Self::begin_lock_cycle(&mut self.locks.caps_lock, &mut self.lock_cycles.caps_lock)
            }
            KeyCode::NumLock => {
                Self::begin_lock_cycle(&mut self.locks.num_lock, &mut self.lock_cycles.num_lock)
            }
            KeyCode::ScrollLock => Self::begin_lock_cycle(
                &mut self.locks.scroll_lock,
                &mut self.lock_cycles.scroll_lock,
            ),
            _ => false,
        };

        changed.then(|| KeyboardFeedback::SetLeds(self.locks.into()))
    }

    fn handle_lock_released(&mut self, code: KeyCode) -> Option<KeyboardFeedback> {
        let changed = match code {
            KeyCode::CapsLock => {
                Self::finish_lock_cycle(&mut self.locks.caps_lock, &mut self.lock_cycles.caps_lock)
            }
            KeyCode::NumLock => {
                Self::finish_lock_cycle(&mut self.locks.num_lock, &mut self.lock_cycles.num_lock)
            }
            KeyCode::ScrollLock => Self::finish_lock_cycle(
                &mut self.locks.scroll_lock,
                &mut self.lock_cycles.scroll_lock,
            ),
            _ => false,
        };

        changed.then(|| KeyboardFeedback::SetLeds(self.locks.into()))
    }

    fn begin_lock_cycle(lock: &mut bool, cycle: &mut LockKeyCycle) -> bool {
        if *lock {
            *cycle = LockKeyCycle::Unlocking;
            false
        } else {
            *lock = true;
            *cycle = LockKeyCycle::Locking;
            true
        }
    }

    fn finish_lock_cycle(lock: &mut bool, cycle: &mut LockKeyCycle) -> bool {
        let changed = if matches!(*cycle, LockKeyCycle::Unlocking) {
            *lock = false;
            true
        } else {
            false
        };
        *cycle = LockKeyCycle::Idle;
        changed
    }
}

impl<L> Default for Keyboard<L>
where
    L: KeyboardLayout + Default,
{
    fn default() -> Self {
        Self::new(L::default())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        KeyAction, KeyCode, KeyEvent, KeyState, Keyboard, KeyboardFeedback, KeyboardLeds, UsQwerty,
    };

    fn press(keyboard: &mut Keyboard<UsQwerty>, code: KeyCode) -> super::KeyboardOutput {
        keyboard.handle_key_event(KeyEvent {
            code,
            state: KeyState::Pressed,
        })
    }

    fn release(keyboard: &mut Keyboard<UsQwerty>, code: KeyCode) -> super::KeyboardOutput {
        keyboard.handle_key_event(KeyEvent {
            code,
            state: KeyState::Released,
        })
    }

    #[test]
    fn translates_plain_shifted_and_repeated_keys() {
        let mut keyboard = Keyboard::new(UsQwerty);

        assert_eq!(press(&mut keyboard, KeyCode::A).text, Some('a'));
        let repeated = press(&mut keyboard, KeyCode::A);
        assert_eq!(repeated.key.action, KeyAction::Repeated);
        assert_eq!(repeated.text, Some('a'));
        let _ = release(&mut keyboard, KeyCode::A);

        let _ = press(&mut keyboard, KeyCode::LeftShift);
        assert_eq!(press(&mut keyboard, KeyCode::Digit1).text, Some('!'));
        assert_eq!(press(&mut keyboard, KeyCode::B).text, Some('B'));
    }

    #[test]
    fn unlocks_caps_lock_on_the_second_release() {
        let mut keyboard = Keyboard::new(UsQwerty);

        let first_down = press(&mut keyboard, KeyCode::CapsLock);
        assert!(first_down.key.locks.caps_lock);
        assert_eq!(
            first_down.feedback,
            Some(KeyboardFeedback::SetLeds(KeyboardLeds {
                caps_lock: true,
                num_lock: false,
                scroll_lock: false,
            }))
        );

        let repeated = press(&mut keyboard, KeyCode::CapsLock);
        assert_eq!(repeated.key.action, KeyAction::Repeated);
        assert!(repeated.key.locks.caps_lock);
        assert_eq!(repeated.feedback, None);

        let first_up = release(&mut keyboard, KeyCode::CapsLock);
        assert!(first_up.key.locks.caps_lock);
        assert_eq!(first_up.feedback, None);

        let second_down = press(&mut keyboard, KeyCode::CapsLock);
        assert!(second_down.key.locks.caps_lock);
        assert_eq!(second_down.feedback, None);
        assert_eq!(press(&mut keyboard, KeyCode::A).text, Some('A'));
        let _ = release(&mut keyboard, KeyCode::A);

        let second_up = release(&mut keyboard, KeyCode::CapsLock);
        assert!(!second_up.key.locks.caps_lock);
        assert_eq!(
            second_up.feedback,
            Some(KeyboardFeedback::SetLeds(KeyboardLeds::default()))
        );
    }

    #[test]
    fn shift_cancels_caps_lock_for_letters() {
        let mut keyboard = Keyboard::new(UsQwerty);
        let _ = press(&mut keyboard, KeyCode::CapsLock);
        let _ = release(&mut keyboard, KeyCode::CapsLock);
        let _ = press(&mut keyboard, KeyCode::LeftShift);

        assert_eq!(press(&mut keyboard, KeyCode::A).text, Some('a'));
    }

    #[test]
    fn pause_is_an_instantaneous_press() {
        let mut keyboard = Keyboard::new(UsQwerty);

        assert_eq!(
            press(&mut keyboard, KeyCode::Pause).key.action,
            KeyAction::Pressed
        );
        assert!(!keyboard.is_pressed(KeyCode::Pause));
        assert_eq!(
            press(&mut keyboard, KeyCode::Pause).key.action,
            KeyAction::Pressed
        );
    }

    #[test]
    fn reset_clears_held_keys_but_preserves_locks() {
        let mut keyboard = Keyboard::new(UsQwerty);
        let _ = press(&mut keyboard, KeyCode::CapsLock);
        let _ = release(&mut keyboard, KeyCode::CapsLock);
        let _ = press(&mut keyboard, KeyCode::LeftShift);

        keyboard.reset_pressed_state();

        assert!(!keyboard.modifiers().shift());
        assert!(keyboard.locks().caps_lock);
        assert_eq!(press(&mut keyboard, KeyCode::A).text, Some('A'));
    }
}
