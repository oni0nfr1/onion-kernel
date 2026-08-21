#![no_std]
#![no_main]

mod arch;
mod drivers;
mod subsystems;
mod util;

use core::panic::PanicInfo;
use limine::{BaseRevision, RequestsEndMarker, RequestsStartMarker, request::FramebufferRequest};

use crate::{
    drivers::framebuffer::Framebuffer,
    subsystems::display::{
        font::{DEFAULT_FONT_DATA, onft::OnftFont},
        text::{FramebufferTextScreen, TextScreen},
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

    halt() // 정상 종료
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
