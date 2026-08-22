/// Reads one byte from an x86 I/O port.
///
/// # Safety
///
/// The caller must be privileged to execute port I/O, ensure that `port` is
/// valid to read in the target hardware's current state, and synchronize this
/// access with every other user of the same device. The caller must also
/// account for any device side effects caused by the read.
#[inline]
pub unsafe fn read_u8(port: u16) -> u8 {
    let value: u8;

    unsafe {
        core::arch::asm!(
            "in al, dx",
            in("dx") port,
            out("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }

    value
}

/// Writes one byte to an x86 I/O port.
///
/// # Safety
///
/// The caller must be privileged to execute port I/O, ensure that `port` is
/// valid to write in the target hardware's current state, and synchronize this
/// access with every other user of the same device. `value` must be valid for
/// the selected device register and protocol state.
#[inline]
pub unsafe fn write_u8(port: u16, value: u8) {
    unsafe {
        core::arch::asm!(
            "out dx, al",
            in("dx") port,
            in("al") value,
            options(nomem, nostack, preserves_flags),
        );
    }
}
