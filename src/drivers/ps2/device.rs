use super::{
    controller::I8042Error,
    scancode::{KeyEvent, ScanCodeSet},
};

/// An input event produced spontaneously by a supported PS/2 device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps2Event {
    Keyboard(KeyEvent),
}

/// The successful response of a keyboard command.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps2KeyboardCommandResponse {
    Initialized,
    DeviceId(u16),
    ScanCodeSet(ScanCodeSet),
}

/// The successful response of a command issued to a supported PS/2 device.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps2CommandResponse {
    Completed,
    Keyboard(Ps2KeyboardCommandResponse),
}

/// A PS/2 device or command-processing error.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps2Error {
    Controller(I8042Error),
    ResponseTimeout,
    TooManyResends(u8),
    NotInitialized,
    AlreadyInitialized,
    NoPendingCommand,
    UnexpectedScanCodeByte(u8),
    UnknownScanCode { scan_code: u8, extended: bool },
    DeviceRemoved,
}

/// An immediate hardware operation requested by a device driver.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[must_use]
pub enum Ps2DeviceAction {
    Send(u8),
}

/// Proof returned by a concrete driver after it enqueues exactly one command.
///
/// Only PS/2 driver implementations can construct this value. The bus uses it
/// to keep its scheduling-token queue aligned with device-owned command queues.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ps2CommandQueued {
    _private: (),
}

impl Ps2CommandQueued {
    pub(crate) const fn new() -> Self {
        Self { _private: () }
    }
}

/// The effect that processing had on the currently active command.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Ps2CommandStatus {
    #[default]
    None,
    Pending,
    Completed(Ps2CommandResponse),
    Failed(Ps2Error),
}

/// Everything a device driver produced while processing one state transition.
///
/// These fields are independent because a normal input event can arrive while
/// a command remains pending and causes a follow-up action.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[must_use]
pub struct Ps2DeviceOutput {
    pub action: Option<Ps2DeviceAction>,
    pub event: Option<Ps2Event>,
    pub error: Option<Ps2Error>,
    pub command: Ps2CommandStatus,
}

impl Ps2DeviceOutput {
    pub const fn none() -> Self {
        Self {
            action: None,
            event: None,
            error: None,
            command: Ps2CommandStatus::None,
        }
    }

    pub const fn event(event: Ps2Event) -> Self {
        Self {
            action: None,
            event: Some(event),
            error: None,
            command: Ps2CommandStatus::None,
        }
    }

    pub const fn error(error: Ps2Error) -> Self {
        Self {
            action: None,
            event: None,
            error: Some(error),
            command: Ps2CommandStatus::None,
        }
    }

    pub const fn send(byte: u8) -> Self {
        Self {
            action: Some(Ps2DeviceAction::Send(byte)),
            event: None,
            error: None,
            command: Ps2CommandStatus::Pending,
        }
    }

    pub const fn pending_event(event: Ps2Event) -> Self {
        Self {
            action: None,
            event: Some(event),
            error: None,
            command: Ps2CommandStatus::Pending,
        }
    }

    pub const fn pending_error(error: Ps2Error) -> Self {
        Self {
            action: None,
            event: None,
            error: Some(error),
            command: Ps2CommandStatus::Pending,
        }
    }

    pub const fn completed(response: Ps2CommandResponse) -> Self {
        Self {
            action: None,
            event: None,
            error: None,
            command: Ps2CommandStatus::Completed(response),
        }
    }

    pub const fn failed(error: Ps2Error) -> Self {
        Self {
            action: None,
            event: None,
            error: None,
            command: Ps2CommandStatus::Failed(error),
        }
    }
}

/// A driver for one device connected to a PS/2 port.
///
/// Device-specific command types and queues belong to the concrete driver.
/// The bus sees only whether work is pending and the actions produced while it
/// drives that work, which keeps this trait object-safe for future hot-plugged
/// device slots.
pub trait Ps2Device {
    fn has_pending_command(&self) -> bool;

    fn begin_next_command(&mut self) -> Ps2DeviceOutput;

    fn handle_byte(&mut self, byte: u8) -> Ps2DeviceOutput;

    fn handle_timeout(&mut self) -> Ps2DeviceOutput;

    fn abort_active_command(&mut self);

    fn cancel_pending_commands(&mut self);

    fn reset_receive_state(&mut self);
}

/// A device slot that intentionally ignores every received byte.
#[derive(Clone, Copy, Debug, Default)]
pub struct NoDevice;

impl Ps2Device for NoDevice {
    fn has_pending_command(&self) -> bool {
        false
    }

    fn begin_next_command(&mut self) -> Ps2DeviceOutput {
        Ps2DeviceOutput::failed(Ps2Error::NoPendingCommand)
    }

    fn handle_byte(&mut self, _byte: u8) -> Ps2DeviceOutput {
        Ps2DeviceOutput::none()
    }

    fn handle_timeout(&mut self) -> Ps2DeviceOutput {
        Ps2DeviceOutput::none()
    }

    fn abort_active_command(&mut self) {}

    fn cancel_pending_commands(&mut self) {}

    fn reset_receive_state(&mut self) {}
}

/// A small allocation-free FIFO used for device-owned commands and bus-owned
/// scheduling tokens. It intentionally supports `Copy` values only so it can
/// be initialized in a `const fn` without allocation or unsafe code.
pub(crate) struct FixedQueue<T, const CAPACITY: usize>
where
    T: Copy,
{
    entries: [Option<T>; CAPACITY],
    head: usize,
    length: usize,
}

impl<T, const CAPACITY: usize> FixedQueue<T, CAPACITY>
where
    T: Copy,
{
    pub const fn new() -> Self {
        Self {
            entries: [None; CAPACITY],
            head: 0,
            length: 0,
        }
    }

    pub const fn is_empty(&self) -> bool {
        self.length == 0
    }

    pub const fn is_full(&self) -> bool {
        self.length == CAPACITY
    }

    pub fn push(&mut self, value: T) -> Result<(), T> {
        if self.is_full() {
            return Err(value);
        }

        let distance_to_end = CAPACITY - self.head;
        let tail = if self.length >= distance_to_end {
            self.length - distance_to_end
        } else {
            self.head + self.length
        };
        self.entries[tail] = Some(value);
        self.length += 1;
        Ok(())
    }

    pub fn pop(&mut self) -> Option<T> {
        if self.is_empty() {
            return None;
        }

        let value = self.entries[self.head].take();
        self.head = if self.head + 1 == CAPACITY {
            0
        } else {
            self.head + 1
        };
        self.length -= 1;
        value
    }

    pub fn clear(&mut self) {
        *self = Self::new();
    }
}
