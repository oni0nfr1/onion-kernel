#![no_std]
#![no_main]

use core::{fmt::Write as _, panic::PanicInfo};
use kernel::{
    drivers::{
        framebuffer::Framebuffer,
        ps2::{
            bus::Ps2Bus,
            controller::I8042,
            device::{NoDevice, Ps2Event},
            keyboard::Ps2Keyboard,
            scancode::set2::Set2Decoder,
        },
        serial::{COM1_BASE, SerialPort},
    },
    subsystems::{
        console::{AsciiConsole, Console, ConsoleColors},
        display::{
            font::{DEFAULT_FONT_DATA, onft::OnftFont},
            text::FramebufferTextScreen,
        },
        input::keyboard::{KeyAction, KeyCode, UsQwerty, UsQwertyKeyboard},
    },
    util::color::Color,
};
use limine::{BaseRevision, RequestsEndMarker, RequestsStartMarker, request::FramebufferRequest};

#[used]
#[unsafe(link_section = ".limine_requests_start")]
static REQUESTS_START: RequestsStartMarker = RequestsStartMarker::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static BASE_REVISION: BaseRevision = BaseRevision::new();

#[used]
#[unsafe(link_section = ".limine_requests")]
static FRAMEBUFFER_REQUEST: FramebufferRequest = FramebufferRequest::new();

#[used]
#[unsafe(link_section = ".limine_requests_end")]
static REQUESTS_END: RequestsEndMarker = RequestsEndMarker::new();

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    // 지원되지 않는 limine 프로토콜 버전이면 하드웨어를 얻을 수 없음
    assert!(BASE_REVISION.is_supported());

    let font = OnftFont::parse(DEFAULT_FONT_DATA).expect("embedded ONFT font must be valid");

    // 지원되는 프레임버퍼가 없으면 이 커널을 쓸 수 없음
    let response = FRAMEBUFFER_REQUEST
        .response()
        .expect("framebuffer response received from bootloader");

    let framebuffer = response
        .framebuffers()
        .iter()
        .copied()
        .find_map(|framebuffer| Framebuffer::new(framebuffer).ok())
        .expect("supported framebuffer found from bootloader response");

    let screen =
        FramebufferTextScreen::new(framebuffer, font).expect("font must fit in framebuffer");
    let mut console = AsciiConsole::new(screen, ConsoleColors::new(Color::WHITE, Color::BLACK))
        .expect("framebuffer text screen must contain at least one cell");
    console.clear();

    // SAFETY: This kernel assumes an i8042-compatible controller is present on
    // the current x86_64 platform, and this is the only value that accesses its
    // data and status ports. Interrupt-driven access has not been enabled.
    let controller = unsafe { I8042::new() };
    let keyboard_driver = Ps2Keyboard::new(Set2Decoder::new());
    let mut ps2_bus = Ps2Bus::new(controller, keyboard_driver, NoDevice);

    ps2_bus
        .initialize_controller()
        .expect("i8042 controller and its first PS/2 port must initialize");
    let initialization = ps2_bus
        .request_first(|keyboard| keyboard.initialize())
        .expect("first-port PS/2 keyboard initialization must be queued");

    loop {
        let Some(output) = ps2_bus
            .poll()
            .expect("PS/2 bus must remain operational during keyboard initialization")
        else {
            core::hint::spin_loop();
            continue;
        };

        if let Some(completion) = output.completion
            && completion.id == initialization
        {
            completion
                .result
                .expect("first-port PS/2 keyboard must initialize");
            break;
        }
    }

    let mut keyboard = UsQwertyKeyboard::new(UsQwerty);

    loop {
        let Some(output) = ps2_bus
            .poll()
            .expect("PS/2 bus must remain operational while polling keyboard input")
        else {
            core::hint::spin_loop();
            continue;
        };
        let Some(Ps2Event::Keyboard(event)) = output.event else {
            continue;
        };

        let input = keyboard.handle_key_event(event);
        let byte = if let Some(ch) = input.text {
            if ch.is_ascii() { ch as u8 } else { b'?' }
        } else if matches!(input.key.action, KeyAction::Pressed | KeyAction::Repeated) {
            match input.key.code {
                KeyCode::Enter | KeyCode::NumpadEnter => b'\n',
                KeyCode::Tab => b'\t',
                KeyCode::Backspace => 0x08,
                _ => continue,
            }
        } else {
            continue;
        };

        let output = if byte == 0x08 {
            console.write_bytes(b"\x08 \x08")
        } else {
            console.write_byte(byte)
        };
        output.expect("framebuffer console output must remain drawable");
    }
}

fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    // SAFETY: Panic output is the kernel's only COM1 user. Interrupt-driven
    // serial access is not enabled, and reinitialization is acceptable on this
    // terminal failure path.
    let mut serial = unsafe { SerialPort::new(COM1_BASE) };
    serial.initialize();
    let _ = writeln!(serial, "\nKERNEL PANIC: {info}");

    halt()
}
