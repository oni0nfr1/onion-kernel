#![no_std]
#![no_main]

use core::{fmt::Write as _, panic::PanicInfo};

use kernel::{
    arch::{
        common::paging::{Address as _, Page as _},
        x86_64::paging::address::{
            PhysicalAddress, PhysicalPage, VirtualAddress, max_physical_address_bits,
        },
    },
    boot::{
        AddressDecoder,
        limine::{
            hhdm::LimineHhdm,
            memory::memory_map,
            requests::{BASE_REVISION, FRAMEBUFFER_REQUEST},
        },
        memory::UsablePhysicalPageRanges,
    },
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
        memory::BitmapPageAllocator,
    },
    util::color::Color,
};

type KernelConsole = AsciiConsole<FramebufferTextScreen<'static, OnftFont<'static>>>;
type KeyboardBus = Ps2Bus<Ps2Keyboard<Set2Decoder>, NoDevice>;

#[derive(Clone, Copy)]
struct PhysicalAddressDecoder {
    max_physical_address_bits: u8,
}

impl AddressDecoder for PhysicalAddressDecoder {
    type Address = PhysicalAddress;

    fn decode(&self, value: u64) -> Option<Self::Address> {
        PhysicalAddress::new(value, self.max_physical_address_bits)
    }
}

#[derive(Clone, Copy)]
struct VirtualAddressDecoder;

impl AddressDecoder for VirtualAddressDecoder {
    type Address = VirtualAddress;

    fn decode(&self, value: u64) -> Option<Self::Address> {
        VirtualAddress::new(value)
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn kmain() -> ! {
    assert!(
        BASE_REVISION.is_supported(),
        "bootloader must support the requested Limine base revision",
    );

    let mut console = initialize_console();
    writeln!(console, "onion kernel").expect("startup banner must be drawable");

    let mut ps2_bus = initialize_keyboard();
    writeln!(console, "[  OK  ] PS/2 keyboard initialized")
        .expect("keyboard initialization result must be drawable");

    test_hhdm_page_allocator(&mut console);
    writeln!(console, "\nKeyboard input:").expect("keyboard prompt must be drawable");

    run_keyboard_echo(&mut console, &mut ps2_bus)
}

fn initialize_console() -> KernelConsole {
    let font = OnftFont::parse(DEFAULT_FONT_DATA).expect("embedded ONFT font must be valid");
    let response = FRAMEBUFFER_REQUEST
        .response()
        .expect("framebuffer response must be provided by the bootloader");
    let framebuffer = response
        .framebuffers()
        .iter()
        .copied()
        .find_map(|framebuffer| Framebuffer::new(framebuffer).ok())
        .expect("bootloader must provide a supported framebuffer");
    let screen =
        FramebufferTextScreen::new(framebuffer, font).expect("embedded font must fit framebuffer");
    let mut console = AsciiConsole::new(screen, ConsoleColors::new(Color::WHITE, Color::BLACK))
        .expect("framebuffer text screen must contain at least one cell");
    console.clear();
    console
}

fn initialize_keyboard() -> KeyboardBus {
    // SAFETY: This is the kernel's only i8042 controller owner. Interrupt-driven
    // access has not been enabled, so no concurrent port access can occur.
    let controller = unsafe { I8042::new() };
    let keyboard_driver = Ps2Keyboard::new(Set2Decoder::new());
    let mut bus = Ps2Bus::new(controller, keyboard_driver, NoDevice);

    bus.initialize_controller()
        .expect("i8042 controller and first PS/2 port must initialize");
    let initialization = bus
        .request_first(|keyboard| keyboard.initialize())
        .expect("first-port keyboard initialization must be queued");

    loop {
        let Some(output) = bus
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
            return bus;
        }
    }
}

fn test_hhdm_page_allocator(console: &mut KernelConsole) {
    let address_bits = max_physical_address_bits()
        .expect("processor must report a supported MAXPHYADDR between 12 and 52 bits");
    let physical_decoder = PhysicalAddressDecoder {
        max_physical_address_bits: address_bits,
    };
    let hhdm = LimineHhdm::new(physical_decoder, VirtualAddressDecoder)
        .expect("bootloader must provide a valid HHDM response");
    let memory_map =
        memory_map(&physical_decoder).expect("bootloader must provide a physical memory map");
    let usable_pages = UsablePhysicalPageRanges::<_, PhysicalPage>::new(&memory_map, address_bits)
        .expect("boot memory map must contain valid physical page ranges");

    // SAFETY: Every range comes from Limine USABLE entries, the Limine HHDM
    // provides writable access to those pages, and no other allocator exists.
    let mut allocator = unsafe {
        BitmapPageAllocator::bootstrap(usable_pages, address_bits, hhdm)
            .expect("physical page allocator must bootstrap from usable memory")
    };

    let metadata = allocator.metadata_region();
    writeln!(console, "[  OK  ] HHDM bitmap page allocator initialized")
        .expect("allocator initialization result must be drawable");
    writeln!(console, "         MAXPHYADDR: {address_bits} bits")
        .expect("MAXPHYADDR result must be drawable");
    writeln!(
        console,
        "         metadata: {:#018x}, {} pages",
        metadata.start().start_address().value(),
        metadata.page_count(),
    )
    .expect("allocator metadata result must be drawable");

    let page = allocator
        .allocate()
        .expect("physical page allocation test must find a free page");
    writeln!(
        console,
        "[  OK  ] allocated page: {:#018x}",
        page.start_address().value(),
    )
    .expect("page allocation result must be drawable");

    // SAFETY: `page` was returned immediately above and has not been exposed or
    // accessed since allocation.
    unsafe { allocator.deallocate(page) };
    let reused_page = allocator
        .allocate()
        .expect("released physical page must be allocatable again");
    assert_eq!(
        reused_page, page,
        "allocator must reuse the page released by the allocation test",
    );
    // SAFETY: `reused_page` was allocated immediately above and has no users.
    unsafe { allocator.deallocate(reused_page) };

    writeln!(console, "[  OK  ] released page and allocated it again")
        .expect("page release result must be drawable");
}

fn run_keyboard_echo(console: &mut KernelConsole, ps2_bus: &mut KeyboardBus) -> ! {
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

        let result = if byte == 0x08 {
            console.write_bytes(b"\x08 \x08")
        } else {
            console.write_byte(byte)
        };
        result.expect("keyboard echo must remain drawable");
    }
}

fn halt() -> ! {
    loop {
        // SAFETY: Halting is valid on the terminal panic path. Interrupts may
        // wake the processor, after which the loop halts it again.
        unsafe { core::arch::asm!("hlt") }
    }
}

#[panic_handler]
fn panic(info: &PanicInfo<'_>) -> ! {
    // SAFETY: Panic output is the kernel's only COM1 user. Interrupt-driven
    // serial access is not enabled, and reinitialization is acceptable here.
    let mut serial = unsafe { SerialPort::new(COM1_BASE) };
    serial.initialize();
    let _ = writeln!(serial, "\nKERNEL PANIC: {info}");

    halt()
}
