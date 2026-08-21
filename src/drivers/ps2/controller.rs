use crate::arch::x86_64::port;

const DATA_PORT: u16 = 0x60;
const STATUS_COMMAND_PORT: u16 = 0x64;

const COMMAND_READ_CONFIGURATION: u8 = 0x20;
const COMMAND_WRITE_CONFIGURATION: u8 = 0x60;
const COMMAND_DISABLE_SECOND_PORT: u8 = 0xa7;
const COMMAND_ENABLE_SECOND_PORT: u8 = 0xa8;
const COMMAND_TEST_SECOND_PORT: u8 = 0xa9;
const COMMAND_TEST_CONTROLLER: u8 = 0xaa;
const COMMAND_TEST_FIRST_PORT: u8 = 0xab;
const COMMAND_DISABLE_FIRST_PORT: u8 = 0xad;
const COMMAND_ENABLE_FIRST_PORT: u8 = 0xae;
const COMMAND_WRITE_SECOND_PORT: u8 = 0xd4;

const CONTROLLER_TEST_PASSED: u8 = 0x55;
const PORT_TEST_PASSED: u8 = 0x00;

const CONFIGURATION_FIRST_PORT_INTERRUPT: u8 = 1 << 0;
const CONFIGURATION_SECOND_PORT_INTERRUPT: u8 = 1 << 1;
const CONFIGURATION_SECOND_PORT_CLOCK_DISABLED: u8 = 1 << 5;
const CONFIGURATION_FIRST_PORT_TRANSLATION: u8 = 1 << 6;

/// Maximum number of status-register polls performed by one bounded wait.
///
/// This is an iteration limit rather than a duration. It can be replaced by a
/// timer-based deadline when the kernel gains a timer subsystem.
const WAIT_ITERATIONS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I8042Port {
    First,
    Second,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I8042Ports {
    pub first: bool,
    pub second: bool,
}

impl I8042Ports {
    pub const fn contains(self, port: I8042Port) -> bool {
        match port {
            I8042Port::First => self.first,
            I8042Port::Second => self.second,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct I8042Output {
    pub port: I8042Port,
    pub byte: u8,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I8042Wait {
    Read,
    Write,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I8042Error {
    Timeout(I8042Wait),
    OutputBufferDidNotDrain,
    DeviceTimeout,
    Parity,
    UnexpectedControllerResponse(I8042Output),
    ControllerSelfTestFailed(u8),
    FirstPortTestFailed(u8),
}

#[derive(Clone, Copy)]
struct Status(u8);

impl Status {
    const fn output_buffer_full(self) -> bool {
        self.0 & (1 << 0) != 0
    }

    const fn input_buffer_full(self) -> bool {
        self.0 & (1 << 1) != 0
    }

    const fn output_from_second_port(self) -> bool {
        self.0 & (1 << 5) != 0
    }

    const fn timeout_error(self) -> bool {
        self.0 & (1 << 6) != 0
    }

    const fn parity_error(self) -> bool {
        self.0 & (1 << 7) != 0
    }
}

/// An exclusively owned interface to an i8042-compatible PS/2 controller.
///
/// This type serializes ordinary accesses through `&mut self`, but callers
/// must still arrange synchronization with an interrupt handler if one also
/// accesses the controller.
pub struct I8042 {
    _private: (),
}

impl I8042 {
    /// Creates an interface to the platform's i8042-compatible controller.
    ///
    /// # Safety
    ///
    /// The caller must ensure that an i8042-compatible controller exists and
    /// that this value is its only owner. No other code, including interrupt
    /// handlers, may access ports `0x60` or `0x64` without synchronization.
    pub const unsafe fn new() -> Self {
        Self { _private: () }
    }

    /// Initializes the controller and enables every usable PS/2 port.
    ///
    /// Both controller interrupts and scan-code translation are left disabled.
    /// This method initializes only the controller. Device reset and
    /// configuration are queued separately through their device drivers.
    pub fn initialize(&mut self) -> Result<I8042Ports, I8042Error> {
        self.write_command(COMMAND_DISABLE_FIRST_PORT)?;
        self.write_command(COMMAND_DISABLE_SECOND_PORT)?;
        self.flush_output()?;

        let configuration = self.read_configuration()?
            & !(CONFIGURATION_FIRST_PORT_INTERRUPT
                | CONFIGURATION_SECOND_PORT_INTERRUPT
                | CONFIGURATION_FIRST_PORT_TRANSLATION);
        self.write_configuration(configuration)?;

        let response = self.command_response(COMMAND_TEST_CONTROLLER)?;
        if response.byte != CONTROLLER_TEST_PASSED {
            return Err(I8042Error::ControllerSelfTestFailed(response.byte));
        }

        // Some controllers reset their configuration byte during self-test.
        self.write_configuration(configuration)?;

        // Enabling the second port should clear its clock-disabled bit on a
        // dual-channel controller. Disable it again before interface tests so
        // no device output can interfere with controller responses.
        self.write_command(COMMAND_ENABLE_SECOND_PORT)?;
        let second_port_present =
            self.read_configuration()? & CONFIGURATION_SECOND_PORT_CLOCK_DISABLED == 0;
        self.write_command(COMMAND_DISABLE_SECOND_PORT)?;

        let response = self.command_response(COMMAND_TEST_FIRST_PORT)?;
        if response.byte != PORT_TEST_PASSED {
            return Err(I8042Error::FirstPortTestFailed(response.byte));
        }

        let second_port_usable = if second_port_present {
            self.command_response(COMMAND_TEST_SECOND_PORT)?.byte == PORT_TEST_PASSED
        } else {
            false
        };

        self.write_command(COMMAND_ENABLE_FIRST_PORT)?;
        if second_port_usable {
            self.write_command(COMMAND_ENABLE_SECOND_PORT)?;
        }

        Ok(I8042Ports {
            first: true,
            second: second_port_usable,
        })
    }

    /// Reads and removes one pending controller output byte without waiting.
    pub fn try_read(&mut self) -> Result<Option<I8042Output>, I8042Error> {
        let status = self.read_status();
        if !status.output_buffer_full() {
            return Ok(None);
        }

        // SAFETY: `status` was read immediately above and reports that the
        // output buffer is full. No data-port access occurred afterward, and
        // exclusive controller ownership prevents another ordinary caller
        // from consuming the pending byte.
        Ok(Some(unsafe { self.read_output_with_status(status)? }))
    }

    /// Writes one byte to the device connected to the first PS/2 port.
    pub fn write_first_port(&mut self, value: u8) -> Result<(), I8042Error> {
        self.write_port(I8042Port::First, value)
    }

    /// Writes one byte to a device connected to the selected PS/2 port.
    pub fn write_port(&mut self, port: I8042Port, value: u8) -> Result<(), I8042Error> {
        match port {
            I8042Port::First => self.write_data(value),
            I8042Port::Second => {
                self.write_command(COMMAND_WRITE_SECOND_PORT)?;
                self.write_data(value)
            }
        }
    }

    fn read_status(&self) -> Status {
        // SAFETY: Exclusive ownership of this I8042 value grants access to the
        // controller's status port. Reading it is valid in every controller state.
        Status(unsafe { port::read_u8(STATUS_COMMAND_PORT) })
    }

    fn wait_until_readable(&self) -> Result<Status, I8042Error> {
        for _ in 0..WAIT_ITERATIONS {
            let status = self.read_status();
            if status.output_buffer_full() {
                return Ok(status);
            }

            core::hint::spin_loop();
        }

        Err(I8042Error::Timeout(I8042Wait::Read))
    }

    fn wait_until_writable(&self) -> Result<(), I8042Error> {
        for _ in 0..WAIT_ITERATIONS {
            if !self.read_status().input_buffer_full() {
                return Ok(());
            }

            core::hint::spin_loop();
        }

        Err(I8042Error::Timeout(I8042Wait::Write))
    }

    fn read_output(&mut self) -> Result<I8042Output, I8042Error> {
        let status = self.wait_until_readable()?;

        // SAFETY: `wait_until_readable` returns the status observation that
        // found the output buffer full. No data-port access occurred between
        // that observation and this call, and the controller is exclusively
        // owned.
        unsafe { self.read_output_with_status(status) }
    }

    /// Reads and consumes the pending output byte described by `status`.
    ///
    /// # Safety
    ///
    /// `status` must be a current observation of this controller's status
    /// register with the output-buffer-full bit set. Since that observation,
    /// no code may have read the data port or otherwise consumed or replaced
    /// the pending output byte. Access by interrupt handlers must obey the same
    /// synchronization requirement.
    unsafe fn read_output_with_status(
        &mut self,
        status: Status,
    ) -> Result<I8042Output, I8042Error> {
        debug_assert!(
            status.output_buffer_full(),
            "read_output_with_status requires the output-buffer-full bit; status={:#04x}",
            status.0,
        );

        // The data must be consumed even when the status reports an error, or
        // the controller's output buffer could remain blocked.
        // SAFETY: The caller guarantees that `status` describes the byte still
        // pending in the output buffer, and this value owns the controller.
        let byte = unsafe { port::read_u8(DATA_PORT) };

        if status.timeout_error() {
            return Err(I8042Error::DeviceTimeout);
        }
        if status.parity_error() {
            return Err(I8042Error::Parity);
        }

        let port = if status.output_from_second_port() {
            I8042Port::Second
        } else {
            I8042Port::First
        };

        Ok(I8042Output { port, byte })
    }

    fn write_command(&mut self, command: u8) -> Result<(), I8042Error> {
        self.wait_until_writable()?;

        // SAFETY: This value exclusively owns the controller, and the caller
        // supplies an i8042 controller command at a valid point in its protocol.
        unsafe { port::write_u8(STATUS_COMMAND_PORT, command) };
        Ok(())
    }

    fn write_data(&mut self, value: u8) -> Result<(), I8042Error> {
        self.wait_until_writable()?;

        // SAFETY: This value exclusively owns the controller, and its input
        // buffer was observed to be available immediately before this write.
        unsafe { port::write_u8(DATA_PORT, value) };
        Ok(())
    }

    fn flush_output(&mut self) -> Result<(), I8042Error> {
        for _ in 0..WAIT_ITERATIONS {
            let status = self.read_status();
            if !status.output_buffer_full() {
                return Ok(());
            }

            // SAFETY: The output-buffer-full bit was observed immediately before
            // this access while this value exclusively owns the controller.
            let _ = unsafe { port::read_u8(DATA_PORT) };
        }

        Err(I8042Error::OutputBufferDidNotDrain)
    }

    fn read_configuration(&mut self) -> Result<u8, I8042Error> {
        let response = self.command_response(COMMAND_READ_CONFIGURATION)?;
        Ok(response.byte)
    }

    fn write_configuration(&mut self, configuration: u8) -> Result<(), I8042Error> {
        self.write_command(COMMAND_WRITE_CONFIGURATION)?;
        self.write_data(configuration)
    }

    fn command_response(&mut self, command: u8) -> Result<I8042Output, I8042Error> {
        self.write_command(command)?;
        let response = self.read_output()?;

        if response.port != I8042Port::First {
            return Err(I8042Error::UnexpectedControllerResponse(response));
        }

        Ok(response)
    }
}
