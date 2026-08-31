# Module dependency rules

This document records the intended dependency direction between the kernel's
top-level modules. It distinguishes long-term rules from temporary dependencies
accepted during the initial single-crate implementation.

## Top-level layers

The intended dependency graph is acyclic:

```text
main / assembly layer
    |-- constructs subsystems
    |-- constructs drivers
    |-- invokes boot resource interpreters
    `-- selects arch::x86_64 implementations

subsystems    ---> arch::common, interfaces, util
drivers       ---> concrete arch mechanisms, interfaces, util
boot          ---> arch::common, interfaces, util, boot-protocol crates
arch::x86_64  ---> arch::common, interfaces, util
arch::common  ---> interfaces, util
interfaces    ---> util
```

The arrows indicate allowed dependency directions, not required dependencies.
The assembly layer is responsible for constructing concrete drivers and
subsystems and connecting them together.

## Allowed dependencies

| Module | May depend on |
| --- | --- |
| `util` | `core` and architecture-independent external crates |
| `interfaces` | `util` |
| `arch::common` | `util` and, when necessary, neutral `interfaces` contracts |
| concrete `arch` modules | `arch::common`, `util`, and neutral `interfaces` contracts |
| `boot` | `arch::common`, `interfaces`, `util`, and boot-protocol crates |
| `drivers` | architecture mechanisms, `interfaces`, and `util` |
| `subsystems` | `arch::common`, `interfaces`, `util`, and explicitly lower-level subsystems |
| final assembly layer | every layer it needs to construct and connect |

The following directions are forbidden:

```text
arch        -> drivers
arch        -> subsystems
drivers     -> subsystems
subsystems  -> drivers
subsystems  -> concrete architecture modules such as arch::x86_64
boot        -> drivers, subsystems, or concrete architecture modules
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

### `boot`

`boot` provides boot-protocol interpreters and early-resource preparation
functions. `boot::protocol` contains only the bootloader-independent results
that drivers or subsystems consume. Here, `protocol` means the kernel's final
boot-to-runtime handoff format, not every intermediate boot value and not a
bootloader wire protocol. Types used only while preparing mappings remain in
the other `boot` modules.

It does not modify page tables, allocate virtual regions, construct drivers,
or initialize subsystems. The assembly layer explicitly sequences those
operations. For example, framebuffer preparation remains separate:

```text
boot::limine framebuffer response
             |
             v
boot::framebuffer::PhysicalFramebuffer
             |
             +----> assembly requests an MMIO mapping
             |
             v
boot::protocol::framebuffer::FramebufferData
             |
             v
assembly constructs the framebuffer driver
```

`boot::limine` owns Limine request declarations and response interpretation.
Other boot protocols can implement equivalent resource functions without
changing memory, driver, or subsystem code.

### `arch::common` and concrete architecture modules

`arch::common` contains architecture-neutral contracts needed by portable
kernel policy. Concrete modules such as `arch::x86_64` contain mechanisms and
representations such as port I/O, page-table formats, control registers, and
TLB invalidation.

Subsystems depend only on `arch::common`. Architecture-specific address, page,
flags, and mapper types remain concrete and are exposed through associated
types on the common contracts. The final assembly layer selects the x86-64
implementation and injects it into the memory subsystem:

```text
subsystems::memory ----> arch::common contracts
                               ^
                               |
arch::x86_64::paging ----------+
              ^
              |
       main constructs and connects
```

No architecture module may depend on a physical allocator implementation in
`subsystems::memory`. When paging needs physical pages for intermediate page
tables, the shared contract represents that requirement without choosing its
provider. Concrete composition remains outside both layers.

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

Memory policy belongs to `subsystems::memory`, neutral paging requirements
belong to `arch::common`, and x86-64 page-table manipulation belongs to
`arch::x86_64::paging`:

```text
subsystems::memory::heap
             |
             v
subsystems::memory::manager
       |
       +---> physical allocator --------> util::Bitmap
       |
       +---> virtual-region allocator --> region-tree/node-pool
       |
       +--------------------------------> arch::common contracts

arch::x86_64::paging -------------------> arch::common contracts

main / assembly: connects subsystem implementations to x86-64 implementations
```

The common contracts use associated types so the concrete x86-64 address and
page invariants do not have to be duplicated in portable modules. The exact
contracts are still under design. In particular, operations that require CPU
context such as the detected physical-address width must receive it explicitly
or obtain it through an injected architecture object, rather than global
mutable state.

`util::Bitmap` does not depend on any memory type, and neither
`arch::common` nor `arch::x86_64::paging` depends on the memory subsystem.

The global heap may call the memory manager to grow or release mappings. The
memory manager and paging implementation must never call `Box`, `Vec`, or any
other operation that can recursively invoke the global heap.

Detailed memory-management design is documented in
[`memory/README.md`](memory/README.md).

## Temporary exceptions

### Incomplete Limine migration

Limine requests and new response interpreters belong to `boot::limine`.
`subsystems::memory` no longer depends on Limine. The existing framebuffer
driver and temporary memory-map display in `main` still consume Limine values
directly until MMIO mapping and the new runtime framebuffer descriptor are
implemented.

Limine-specific values must never be stored in steady-state physical or
virtual allocator structures. Once the transition is complete, the intended
flow is:

```text
Limine responses
       |
       v
boot::limine resource function
       | produces physical resource descriptors
       v
main / assembly
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
onion-arch-common
onion-arch-x86_64
onion-boot
onion-drivers
onion-subsystems
onion-kernel
```

Cargo dependencies will then enforce most of the direction described in this
document at compile time.
