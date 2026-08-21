use super::{
    controller::{I8042, I8042Error, I8042Port, I8042Ports},
    device::{
        FixedQueue, Ps2CommandQueued, Ps2CommandResponse, Ps2CommandStatus, Ps2Device,
        Ps2DeviceAction, Ps2DeviceOutput, Ps2Error, Ps2Event,
    },
};

const SCHEDULE_CAPACITY: usize = 16;

/// Maximum number of bus polls allowed without progress from the active port.
///
/// This remains an iteration limit until the kernel has a timer subsystem.
const COMMAND_WAIT_POLLS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ps2CommandId {
    sequence: u64,
    port: I8042Port,
    generation: u32,
}

impl Ps2CommandId {
    pub const fn port(self) -> I8042Port {
        self.port
    }

    pub const fn generation(self) -> u32 {
        self.generation
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Ps2CommandCompletion {
    pub id: Ps2CommandId,
    pub result: Result<Ps2CommandResponse, Ps2Error>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Ps2BusOutput {
    pub event: Option<Ps2Event>,
    pub completion: Option<Ps2CommandCompletion>,
}

impl Ps2BusOutput {
    const fn is_empty(self) -> bool {
        self.event.is_none() && self.completion.is_none()
    }

    fn merge(&mut self, other: Self) -> Result<(), Ps2BusError> {
        if self.event.is_some() && other.event.is_some() {
            return Err(Ps2BusError::OutputOverflow);
        }
        if self.completion.is_some() && other.completion.is_some() {
            return Err(Ps2BusError::OutputOverflow);
        }

        if self.event.is_none() {
            self.event = other.event;
        }
        if self.completion.is_none() {
            self.completion = other.completion;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps2BusError {
    Controller(I8042Error),
    Device { port: I8042Port, error: Ps2Error },
    MissingDeviceCommand(I8042Port),
    UnexpectedDeviceAction(I8042Port),
    UnexpectedCommandStatus(I8042Port),
    OutputOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps2BusInitError {
    AlreadyInitialized,
    Controller(I8042Error),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ps2RequestError<E> {
    ControllerNotInitialized,
    PortUnavailable,
    ScheduleFull,
    Device(E),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ScheduledCommand {
    id: Ps2CommandId,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct ActiveCommand {
    scheduled: ScheduledCommand,
    wait_polls: usize,
}

/// Owns an i8042 controller, schedules device-owned commands, and routes every
/// received byte to the driver attached to its source port.
pub struct Ps2Bus<F, S>
where
    F: Ps2Device,
    S: Ps2Device,
{
    controller: I8042,
    first: F,
    second: S,
    ports: Option<I8042Ports>,
    scheduled: FixedQueue<ScheduledCommand, SCHEDULE_CAPACITY>,
    active: Option<ActiveCommand>,
    next_sequence: u64,
    first_generation: u32,
    second_generation: u32,
}

impl<F, S> Ps2Bus<F, S>
where
    F: Ps2Device,
    S: Ps2Device,
{
    pub const fn new(controller: I8042, first: F, second: S) -> Self {
        Self {
            controller,
            first,
            second,
            ports: None,
            scheduled: FixedQueue::new(),
            active: None,
            next_sequence: 0,
            first_generation: 0,
            second_generation: 0,
        }
    }

    /// Initializes only the i8042 controller and its usable ports.
    ///
    /// Device initialization is an ordinary device-specific command requested
    /// afterward through `request_first` or `request_second`.
    pub fn initialize_controller(&mut self) -> Result<I8042Ports, Ps2BusInitError> {
        if self.ports.is_some() {
            return Err(Ps2BusInitError::AlreadyInitialized);
        }

        let ports = self
            .controller
            .initialize()
            .map_err(Ps2BusInitError::Controller)?;
        self.ports = Some(ports);
        Ok(ports)
    }

    /// Enqueues exactly one device-specific command for the first port.
    pub fn request_first<E>(
        &mut self,
        request: impl FnOnce(&mut F) -> Result<Ps2CommandQueued, E>,
    ) -> Result<Ps2CommandId, Ps2RequestError<E>> {
        self.ensure_requestable(I8042Port::First)?;

        request(&mut self.first).map_err(Ps2RequestError::Device)?;
        Ok(self.schedule(I8042Port::First))
    }

    /// Enqueues exactly one device-specific command for the second port.
    pub fn request_second<E>(
        &mut self,
        request: impl FnOnce(&mut S) -> Result<Ps2CommandQueued, E>,
    ) -> Result<Ps2CommandId, Ps2RequestError<E>> {
        self.ensure_requestable(I8042Port::Second)?;

        request(&mut self.second).map_err(Ps2RequestError::Device)?;
        Ok(self.schedule(I8042Port::Second))
    }

    /// Advances controller I/O and command scheduling by one bounded step.
    ///
    /// At most one controller output byte and one newly scheduled command are
    /// consumed per call. Follow-up sends are performed immediately, while
    /// ordinary events and command completions are returned to the caller.
    pub fn poll(&mut self) -> Result<Option<Ps2BusOutput>, Ps2BusError> {
        let mut bus_output = Ps2BusOutput::default();

        if self.active.is_none()
            && let Some(output) = self.begin_next_scheduled_command()?
        {
            bus_output.merge(output)?;
        }

        match self.controller.try_read() {
            Ok(Some(input)) => {
                let output = match input.port {
                    I8042Port::First => self.first.handle_byte(input.byte),
                    I8042Port::Second => self.second.handle_byte(input.byte),
                };
                bus_output.merge(self.process_device_output(input.port, output)?)?;
            }
            Ok(None) => {}
            Err(error) => {
                self.first.reset_receive_state();
                self.second.reset_receive_state();

                if let Some(completion) = self.fail_active_command(Ps2Error::Controller(error)) {
                    bus_output.merge(Ps2BusOutput {
                        event: None,
                        completion: Some(completion),
                    })?;
                } else {
                    return Err(Ps2BusError::Controller(error));
                }
            }
        }

        if let Some(timeout_output) = self.advance_timeout()? {
            bus_output.merge(timeout_output)?;
        }

        if bus_output.is_empty() {
            Ok(None)
        } else {
            Ok(Some(bus_output))
        }
    }

    pub const fn first(&self) -> &F {
        &self.first
    }

    pub const fn second(&self) -> &S {
        &self.second
    }

    pub const fn ports(&self) -> Option<I8042Ports> {
        self.ports
    }

    pub fn into_parts(self) -> (I8042, F, S) {
        (self.controller, self.first, self.second)
    }

    fn ensure_requestable<E>(&self, port: I8042Port) -> Result<(), Ps2RequestError<E>> {
        let Some(ports) = self.ports else {
            return Err(Ps2RequestError::ControllerNotInitialized);
        };
        if !ports.contains(port) {
            return Err(Ps2RequestError::PortUnavailable);
        }
        if self.scheduled.is_full() {
            return Err(Ps2RequestError::ScheduleFull);
        }
        Ok(())
    }

    fn schedule(&mut self, port: I8042Port) -> Ps2CommandId {
        debug_assert!(
            !self.scheduled.is_full(),
            "schedule called with a full PS/2 command queue (capacity={SCHEDULE_CAPACITY}, port={port:?})",
        );

        let generation = match port {
            I8042Port::First => self.first_generation,
            I8042Port::Second => self.second_generation,
        };
        let id = Ps2CommandId {
            sequence: self.next_sequence,
            port,
            generation,
        };
        self.next_sequence = self.next_sequence.wrapping_add(1);

        let result = self.scheduled.push(ScheduledCommand { id });
        debug_assert!(
            result.is_ok(),
            "PS/2 command queue became full after its precondition was checked (capacity={SCHEDULE_CAPACITY}, id={id:?})",
        );
        id
    }

    fn begin_next_scheduled_command(&mut self) -> Result<Option<Ps2BusOutput>, Ps2BusError> {
        let Some(scheduled) = self.scheduled.pop() else {
            return Ok(None);
        };

        let port = scheduled.id.port;
        let has_pending_command = match port {
            I8042Port::First => self.first.has_pending_command(),
            I8042Port::Second => self.second.has_pending_command(),
        };
        if !has_pending_command {
            return Err(Ps2BusError::MissingDeviceCommand(port));
        }

        self.active = Some(ActiveCommand {
            scheduled,
            wait_polls: 0,
        });
        let output = match port {
            I8042Port::First => self.first.begin_next_command(),
            I8042Port::Second => self.second.begin_next_command(),
        };
        self.process_device_output(port, output).map(Some)
    }

    fn process_device_output(
        &mut self,
        port: I8042Port,
        output: Ps2DeviceOutput,
    ) -> Result<Ps2BusOutput, Ps2BusError> {
        if let Some(error) = output.error {
            return Err(Ps2BusError::Device { port, error });
        }

        let mut bus_output = Ps2BusOutput {
            event: output.event,
            completion: None,
        };

        if let Some(action) = output.action {
            if self.active_port() != Some(port) {
                return Err(Ps2BusError::UnexpectedDeviceAction(port));
            }

            match action {
                Ps2DeviceAction::Send(byte) => {
                    if let Err(error) = self.controller.write_port(port, byte) {
                        let Some(completion) =
                            self.fail_active_command(Ps2Error::Controller(error))
                        else {
                            return Err(Ps2BusError::UnexpectedDeviceAction(port));
                        };
                        bus_output.completion = Some(completion);
                        return Ok(bus_output);
                    }
                    if let Some(active) = &mut self.active {
                        active.wait_polls = 0;
                    }
                }
            }
        }

        match output.command {
            Ps2CommandStatus::None => {
                if output.action.is_some() {
                    return Err(Ps2BusError::UnexpectedCommandStatus(port));
                }
            }
            Ps2CommandStatus::Pending => {
                if self.active_port() != Some(port) {
                    return Err(Ps2BusError::UnexpectedCommandStatus(port));
                }
            }
            Ps2CommandStatus::Completed(response) => {
                let active = self.take_active_for(port)?;
                bus_output.completion = Some(Ps2CommandCompletion {
                    id: active.scheduled.id,
                    result: Ok(response),
                });
            }
            Ps2CommandStatus::Failed(error) => {
                let active = self.take_active_for(port)?;
                bus_output.completion = Some(Ps2CommandCompletion {
                    id: active.scheduled.id,
                    result: Err(error),
                });
            }
        }

        Ok(bus_output)
    }

    fn advance_timeout(&mut self) -> Result<Option<Ps2BusOutput>, Ps2BusError> {
        let Some(active) = &mut self.active else {
            return Ok(None);
        };

        active.wait_polls = active.wait_polls.saturating_add(1);
        if active.wait_polls < COMMAND_WAIT_POLLS {
            return Ok(None);
        }

        let port = active.scheduled.id.port;
        let output = match port {
            I8042Port::First => self.first.handle_timeout(),
            I8042Port::Second => self.second.handle_timeout(),
        };
        self.process_device_output(port, output).map(Some)
    }

    fn active_port(&self) -> Option<I8042Port> {
        self.active.map(|active| active.scheduled.id.port)
    }

    fn take_active_for(&mut self, port: I8042Port) -> Result<ActiveCommand, Ps2BusError> {
        if self.active_port() != Some(port) {
            return Err(Ps2BusError::UnexpectedCommandStatus(port));
        }
        self.active
            .take()
            .ok_or(Ps2BusError::UnexpectedCommandStatus(port))
    }

    fn fail_active_command(&mut self, error: Ps2Error) -> Option<Ps2CommandCompletion> {
        let active = self.active.take()?;
        match active.scheduled.id.port {
            I8042Port::First => self.first.abort_active_command(),
            I8042Port::Second => self.second.abort_active_command(),
        }
        Some(Ps2CommandCompletion {
            id: active.scheduled.id,
            result: Err(error),
        })
    }
}
