use core::fmt;

use crate::arch::x86_64::port;

pub const COM1_BASE: u16 = 0x3f8;

const DATA: u16 = 0;
const INTERRUPT_ENABLE: u16 = 1;
const FIFO_CONTROL: u16 = 2;
const LINE_CONTROL: u16 = 3;
const MODEM_CONTROL: u16 = 4;
const LINE_STATUS: u16 = 5;

const LINE_CONTROL_DLAB: u8 = 1 << 7;
const LINE_CONTROL_8N1: u8 = 0x03;
const LINE_STATUS_TRANSMITTER_EMPTY: u8 = 1 << 5;

const DIVISOR_38_400_BAUD: u16 = 3;
const TRANSMIT_WAIT_ITERATIONS: usize = 100_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SerialError {
    TransmitTimeout,
}

/// An exclusively owned polling interface to a 16550-compatible UART.
pub struct SerialPort {
    base: u16,
}

impl SerialPort {
    /// Creates an interface for the UART at `base` without probing it.
    ///
    /// # Safety
    ///
    /// `base..=base + 7` must identify a 16550-compatible UART, the addition
    /// must fit in `u16`, and this value must have exclusive access to those
    /// ports for its lifetime. Interrupt handlers must obey the same access
    /// synchronization requirement.
    pub const unsafe fn new(base: u16) -> Self {
        Self { base }
    }

    /// Configures the UART for polling output at 38,400 baud using 8-N-1.
    pub fn initialize(&mut self) {
        // SAFETY: `new` requires exclusive ownership of this UART's complete
        // register range. These writes follow the 16550 initialization order.
        unsafe {
            port::write_u8(self.register(INTERRUPT_ENABLE), 0x00);
            port::write_u8(self.register(LINE_CONTROL), LINE_CONTROL_DLAB);
            port::write_u8(self.register(DATA), DIVISOR_38_400_BAUD as u8);
            port::write_u8(
                self.register(INTERRUPT_ENABLE),
                (DIVISOR_38_400_BAUD >> 8) as u8,
            );
            port::write_u8(self.register(LINE_CONTROL), LINE_CONTROL_8N1);
            port::write_u8(self.register(FIFO_CONTROL), 0xc7);
            port::write_u8(self.register(MODEM_CONTROL), 0x0b);
        }
    }

    pub fn write_byte(&mut self, byte: u8) -> Result<(), SerialError> {
        for _ in 0..TRANSMIT_WAIT_ITERATIONS {
            // SAFETY: This value exclusively owns the UART register range,
            // and the line-status register is always safe to read.
            let status = unsafe { port::read_u8(self.register(LINE_STATUS)) };
            if status & LINE_STATUS_TRANSMITTER_EMPTY != 0 {
                // SAFETY: The transmitter holding register was observed to be
                // empty immediately above while this value owns the UART.
                unsafe { port::write_u8(self.register(DATA), byte) };
                return Ok(());
            }

            core::hint::spin_loop();
        }

        Err(SerialError::TransmitTimeout)
    }

    const fn register(&self, offset: u16) -> u16 {
        // SAFETY invariant of `new`: the complete register range fits in u16.
        self.base + offset
    }
}

impl fmt::Write for SerialPort {
    fn write_str(&mut self, text: &str) -> fmt::Result {
        for byte in text.bytes() {
            if byte == b'\n' {
                self.write_byte(b'\r').map_err(|_| fmt::Error)?;
            }
            self.write_byte(byte).map_err(|_| fmt::Error)?;
        }

        Ok(())
    }
}
