#![no_std]
#![no_main]

mod arch;
mod drivers;
mod interfaces;
mod subsystems;
mod util;

use core::panic::PanicInfo;
use limine::{BaseRevision, RequestsEndMarker, RequestsStartMarker, request::FramebufferRequest};

use crate::{
    drivers::{
        framebuffer::Framebuffer,
        ps2::{
            bus::Ps2Bus,
            controller::I8042,
            device::{NoDevice, Ps2Event},
            keyboard::Ps2Keyboard,
            scancode::set2::Set2Decoder,
        },
    },
    subsystems::{
        display::{
            font::{DEFAULT_FONT_DATA, onft::OnftFont},
            text::{FramebufferTextScreen, TextScreen},
        },
        input::keyboard::{UsQwerty, UsQwertyKeyboard},
    },
    util::{color::Color, grid::GridPosition},
};

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

    let mut screen =
        FramebufferTextScreen::new(framebuffer, font).expect("font must fit in framebuffer");

    screen.clear(Color::BLACK);

    const MESSAGE: &str = "Hello World!";

    let screen_size = screen.size();
    let message_columns = MESSAGE.chars().count();
    let message_column = screen_size
        .columns
        .checked_sub(message_columns)
        .expect("message must fit in text screen")
        / 2;
    let message_row = screen_size.rows / 2;

    for (index, ch) in MESSAGE.chars().enumerate() {
        screen
            .put_char(
                GridPosition::new(message_column + index, message_row),
                ch,
                Color::WHITE,
                Color::BLACK,
            )
            .expect("centered message must fit in text screen");
    }

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
        let Some(ch) = input.text else {
            continue;
        };

        screen
            .put_char(GridPosition::new(0, 0), ch, Color::WHITE, Color::BLACK)
            .expect("top-left text-screen cell must remain drawable");
    }
}

fn halt() -> ! {
    loop {
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(_: &PanicInfo<'_>) -> ! {
    halt()
}
