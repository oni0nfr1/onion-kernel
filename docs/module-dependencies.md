# Module dependency rules

This document records the intended dependency direction between the kernel's
top-level modules. It distinguishes long-term rules from temporary dependencies
accepted during the initial single-crate implementation.

## Top-level layers

The intended dependency graph is acyclic:

```text
                         main / assembly layer
                           /              \
                          v                v
                    subsystems          drivers
                       |   \              /  |
                       |    v            v   |
                       |      interfaces     |
                       |          |           |
                       v          v           v
                     arch ----------------> util
```

The arrows indicate allowed dependency directions, not required dependencies.
The assembly layer is responsible for constructing concrete drivers and
subsystems and connecting them together.

## Allowed dependencies

| Module | May depend on |
| --- | --- |
| `util` | `core` and architecture-independent external crates |
| `interfaces` | `util` |
| `arch` | `util` and, when necessary, neutral `interfaces` contracts |
| `drivers` | `arch`, `interfaces`, and `util` |
| `subsystems` | `arch`, `interfaces`, `util`, and explicitly lower-level subsystems |
| final assembly layer | every layer it needs to construct and connect |

The following directions are forbidden:

```text
arch        -> drivers
arch        -> subsystems
drivers     -> subsystems
subsystems  -> drivers
util        -> any kernel layer above util
interfaces  -> drivers or subsystems
```

In particular, architecture-specific code may expose mechanisms that other
layers use, but it must not know which driver or subsystem supplies policy.

## Responsibilities

### `util`

`util` contains data structures and operations that do not encode driver,
subsystem, boot-protocol, or architecture policy. Any layer may depend on it.

Examples include:

- `Bitmap`;
- checked byte decoding;
- colors and geometry; and
- grid coordinates.

A type should not be moved to `util` merely because several modules use it. A
shared type that represents a protocol between layers belongs in `interfaces`.

### `interfaces`

`interfaces` defines transport- and implementation-neutral contracts shared by
drivers, subsystems, and the assembly layer. It must not select a concrete
driver or subsystem implementation.

The existing input event types are an example:

```text
drivers::ps2 -> interfaces::input <- subsystems::input
```

Neither side depends directly on the other.

### `arch`

`arch` contains CPU- and platform-architecture mechanisms such as port I/O,
page-table formats, control registers, and TLB invalidation. Other modules may
depend on these mechanisms.

`arch` must not depend on a physical allocator implementation in
`subsystems::memory`. When paging needs physical pages for intermediate page tables, it
defines the required contract on the architecture side:

```text
arch::x86_64::paging::PageTablePageProvider
                         ^
                         | implements
subsystems::memory::BitmapPageAllocator
```

This preserves the dependency direction:

```text
subsystems::memory -> arch::x86_64::paging
```

### `drivers`

Drivers directly control hardware or firmware-provided devices. They may use
architecture mechanisms and produce or consume neutral interface types. They
must not invoke subsystem policy.

Examples include the framebuffer, serial port, i8042 controller, PS/2 bus, and
PS/2 keyboard driver.

### `subsystems`

Subsystems implement policy and abstractions above raw hardware mechanisms.
Examples include text display, keyboard input interpretation, consoles, and
memory management.

Subsystems must not name concrete driver types. A subsystem should instead
depend on a neutral interface, or a final assembly adapter should connect the
two sides.

Dependencies between subsystem modules are permitted only in a documented
downward direction. The detailed subsystem-internal layering will be decided
after the memory allocator is complete. Until then, new cross-subsystem
dependencies require explicit review to avoid cycles.

The existing console-to-text-display dependency is currently treated as a
higher-level-to-lower-level dependency:

```text
subsystems::console -> subsystems::display::text
```

## Memory-management dependency direction

Memory policy belongs to `subsystems::memory`, while x86-64 page-table
manipulation belongs to `arch::x86_64::paging`:

```text
subsystems::memory::heap
             |
             v
subsystems::memory::manager
       |
       +---> physical allocator --------> util::Bitmap
       |              |
       |              +----------------> arch::x86_64::paging contracts
       |
       +---> virtual-region allocator --> region-tree/node-pool
       |              |
       |              +----------------> arch::x86_64::paging address types
       |
       +--------------------------------> arch::x86_64::paging mapper
```

The diagram expresses conceptual use; `util::Bitmap` does not depend on any
memory type, and `arch::x86_64::paging` does not depend on the memory subsystem.

The global heap may call the memory manager to grow or release mappings. The
memory manager and paging implementation must never call `Box`, `Vec`, or any
other operation that can recursively invoke the global heap.

## Temporary exceptions

### Limine boot protocol

During the initial implementation, the kernel and
`subsystems::memory::bootstrap` may depend directly on Limine response types.
Limine-specific values must not be stored in the steady-state physical or
virtual allocator structures.

When the project is split into crates, the final assembly crate will translate
Limine responses into internal boot-information types and expose them through
traits defined by `interfaces`:

```text
Limine responses
       |
       v
final assembly adapter
       | implements interfaces::boot contracts
       v
memory subsystem
```

### Framebuffer text screen

`subsystems::display::text::FramebufferTextScreen` currently names
`drivers::framebuffer::Framebuffer` directly. This violates the long-term
subsystem-to-driver rule and is an accepted temporary dependency.

After the allocator is complete, the display boundary will be designed in more
detail. Expected options include a neutral pixel-surface interface or a
dedicated assembly adapter. That work will also define the internal dependency
order between display, console, input, and future TTY modules.

## Enforcement

Rust modules inside one crate do not enforce architectural dependency
directions. During the single-crate stage the rules are maintained through:

- review of `use crate::...` paths;
- keeping concrete composition in `main` or an assembly module;
- moving neutral cross-layer contracts to `interfaces`;
- keeping implementation-specific helper modules private where possible; and
- documenting every temporary exception.

When the project grows, the intended crate split is:

```text
onion-util
onion-interfaces
onion-arch-x86_64
onion-drivers
onion-subsystems
onion-kernel
```

Cargo dependencies will then enforce most of the direction described in this
document at compile time.
