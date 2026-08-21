use super::{
    device::{
        FixedQueue, Ps2CommandQueued, Ps2CommandResponse, Ps2Device, Ps2DeviceOutput, Ps2Error,
        Ps2Event, Ps2KeyboardCommandResponse,
    },
    scancode::{ScanCodeDecoder, set2::Set2DecodeError},
};

const COMMAND_SET_SCAN_CODE_SET: u8 = 0xf0;
const COMMAND_DISABLE_SCANNING: u8 = 0xf5;
const COMMAND_ENABLE_SCANNING: u8 = 0xf4;

const RESPONSE_ACKNOWLEDGE: u8 = 0xfa;
const RESPONSE_RESEND: u8 = 0xfe;

const MAX_RESENDS: usize = 3;
const COMMAND_QUEUE_CAPACITY: usize = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps2KeyboardCommandQueueError {
    Full,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Ps2KeyboardCommand {
    Initialize,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum KeyboardAvailability {
    Uninitialized,
    Ready,
    Failed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InitializationStage {
    DisableScanning,
    SetScanCodeSetCommand,
    SetScanCodeSetValue,
    EnableScanning,
}

impl InitializationStage {
    const fn byte(self, scan_code_set: u8) -> u8 {
        match self {
            Self::DisableScanning => COMMAND_DISABLE_SCANNING,
            Self::SetScanCodeSetCommand => COMMAND_SET_SCAN_CODE_SET,
            Self::SetScanCodeSetValue => scan_code_set,
            Self::EnableScanning => COMMAND_ENABLE_SCANNING,
        }
    }

    const fn next(self) -> Option<Self> {
        match self {
            Self::DisableScanning => Some(Self::SetScanCodeSetCommand),
            Self::SetScanCodeSetCommand => Some(Self::SetScanCodeSetValue),
            Self::SetScanCodeSetValue => Some(Self::EnableScanning),
            Self::EnableScanning => None,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ActiveKeyboardCommand {
    Initialize {
        stage: InitializationStage,
        resends: usize,
    },
}

/// A PS/2 keyboard whose semantic commands are queued inside the driver.
pub struct Ps2Keyboard<D>
where
    D: ScanCodeDecoder,
{
    decoder: D,
    availability: KeyboardAvailability,
    active_command: Option<ActiveKeyboardCommand>,
    commands: FixedQueue<Ps2KeyboardCommand, COMMAND_QUEUE_CAPACITY>,
}

impl<D> Ps2Keyboard<D>
where
    D: ScanCodeDecoder,
{
    pub const fn new(decoder: D) -> Self {
        Self {
            decoder,
            availability: KeyboardAvailability::Uninitialized,
            active_command: None,
            commands: FixedQueue::new(),
        }
    }

    /// Queues the keyboard's device-specific initialization command.
    ///
    /// The command does not change the live protocol state until the bus grants
    /// this device the controller command path and calls `begin_next_command`.
    pub fn initialize(&mut self) -> Result<Ps2CommandQueued, Ps2KeyboardCommandQueueError> {
        self.commands
            .push(Ps2KeyboardCommand::Initialize)
            .map_err(|_| Ps2KeyboardCommandQueueError::Full)?;
        Ok(Ps2CommandQueued::new())
    }

    pub const fn is_initialized(&self) -> bool {
        matches!(self.availability, KeyboardAvailability::Ready)
    }

    pub const fn decoder(&self) -> &D {
        &self.decoder
    }

    pub const fn decoder_mut(&mut self) -> &mut D {
        &mut self.decoder
    }

    pub fn into_decoder(self) -> D {
        self.decoder
    }

    fn begin_initialization(&mut self) -> Ps2DeviceOutput {
        match self.availability {
            KeyboardAvailability::Ready => {
                return Ps2DeviceOutput::failed(Ps2Error::AlreadyInitialized);
            }
            KeyboardAvailability::Uninitialized | KeyboardAvailability::Failed => {}
        }

        let stage = InitializationStage::DisableScanning;
        self.active_command = Some(ActiveKeyboardCommand::Initialize { stage, resends: 0 });
        Ps2DeviceOutput::send(stage.byte(D::SCAN_CODE_SET.as_u8()))
    }

    fn handle_active_command(&mut self, byte: u8) -> Ps2DeviceOutput
    where
        D::Error: Into<Ps2Error>,
    {
        let Some(active_command) = self.active_command else {
            return self.decode_scan_code(byte, false);
        };

        match byte {
            RESPONSE_ACKNOWLEDGE => self.handle_acknowledge(active_command),
            RESPONSE_RESEND => self.handle_resend(active_command),
            _ => self.decode_scan_code(byte, true),
        }
    }

    fn handle_acknowledge(&mut self, active_command: ActiveKeyboardCommand) -> Ps2DeviceOutput {
        match active_command {
            ActiveKeyboardCommand::Initialize { stage, .. } => {
                let Some(next_stage) = stage.next() else {
                    self.active_command = None;
                    self.availability = KeyboardAvailability::Ready;
                    self.decoder.reset();
                    return Ps2DeviceOutput::completed(Ps2CommandResponse::Keyboard(
                        Ps2KeyboardCommandResponse::Initialized,
                    ));
                };

                self.active_command = Some(ActiveKeyboardCommand::Initialize {
                    stage: next_stage,
                    resends: 0,
                });
                Ps2DeviceOutput::send(next_stage.byte(D::SCAN_CODE_SET.as_u8()))
            }
        }
    }

    fn handle_resend(&mut self, active_command: ActiveKeyboardCommand) -> Ps2DeviceOutput {
        match active_command {
            ActiveKeyboardCommand::Initialize { stage, resends } => {
                let byte = stage.byte(D::SCAN_CODE_SET.as_u8());
                if resends >= MAX_RESENDS {
                    self.active_command = None;
                    self.availability = KeyboardAvailability::Failed;
                    return Ps2DeviceOutput::failed(Ps2Error::TooManyResends(byte));
                }

                self.active_command = Some(ActiveKeyboardCommand::Initialize {
                    stage,
                    resends: resends + 1,
                });
                Ps2DeviceOutput::send(byte)
            }
        }
    }

    fn decode_scan_code(&mut self, byte: u8, command_pending: bool) -> Ps2DeviceOutput
    where
        D::Error: Into<Ps2Error>,
    {
        match self.decoder.push_byte(byte) {
            Ok(Some(event)) if command_pending => {
                Ps2DeviceOutput::pending_event(Ps2Event::Keyboard(event))
            }
            Ok(Some(event)) => Ps2DeviceOutput::event(Ps2Event::Keyboard(event)),
            Ok(None) if command_pending => Ps2DeviceOutput {
                command: super::device::Ps2CommandStatus::Pending,
                ..Ps2DeviceOutput::none()
            },
            Ok(None) => Ps2DeviceOutput::none(),
            Err(error) if command_pending => Ps2DeviceOutput::pending_error(error.into()),
            Err(error) => Ps2DeviceOutput::error(error.into()),
        }
    }

    fn fail_active_command(&mut self, error: Ps2Error) -> Ps2DeviceOutput {
        if matches!(
            self.active_command,
            Some(ActiveKeyboardCommand::Initialize { .. })
        ) {
            self.availability = KeyboardAvailability::Failed;
        }
        self.active_command = None;
        Ps2DeviceOutput::failed(error)
    }
}

impl<D> Ps2Device for Ps2Keyboard<D>
where
    D: ScanCodeDecoder,
    D::Error: Into<Ps2Error>,
{
    fn has_pending_command(&self) -> bool {
        !self.commands.is_empty()
    }

    fn begin_next_command(&mut self) -> Ps2DeviceOutput {
        if self.active_command.is_some() {
            return Ps2DeviceOutput::failed(Ps2Error::NoPendingCommand);
        }

        match self.commands.pop() {
            Some(Ps2KeyboardCommand::Initialize) => self.begin_initialization(),
            None => Ps2DeviceOutput::failed(Ps2Error::NoPendingCommand),
        }
    }

    fn handle_byte(&mut self, byte: u8) -> Ps2DeviceOutput {
        self.handle_active_command(byte)
    }

    fn handle_timeout(&mut self) -> Ps2DeviceOutput {
        if self.active_command.is_some() {
            self.fail_active_command(Ps2Error::ResponseTimeout)
        } else {
            Ps2DeviceOutput::none()
        }
    }

    fn abort_active_command(&mut self) {
        if matches!(
            self.active_command,
            Some(ActiveKeyboardCommand::Initialize { .. })
        ) {
            self.availability = KeyboardAvailability::Failed;
        }
        self.active_command = None;
    }

    fn cancel_pending_commands(&mut self) {
        self.commands.clear();
    }

    fn reset_receive_state(&mut self) {
        self.decoder.reset();
    }
}

impl<D> Default for Ps2Keyboard<D>
where
    D: ScanCodeDecoder + Default,
{
    fn default() -> Self {
        Self::new(D::default())
    }
}

impl From<Set2DecodeError> for Ps2Error {
    fn from(error: Set2DecodeError) -> Self {
        match error {
            Set2DecodeError::UnexpectedByte(byte) => Self::UnexpectedScanCodeByte(byte),
            Set2DecodeError::UnknownScanCode {
                scan_code,
                extended,
            } => Self::UnknownScanCode {
                scan_code,
                extended,
            },
        }
    }
}
