# x86-64 virtual-address layout

This document defines the intended layout for four-level x86-64 paging. The
policy is architecture-specific and therefore belongs in
`arch::x86_64::paging::layout` rather than `subsystems::memory`.

The layout is based on PML4 entries. Each entry covers 512 GiB, and each group
of 128 entries covers 64 TiB.

## Address-space regions

| PML4 entries | Virtual range | Size | Purpose |
| --- | --- | ---: | --- |
| 0-255 | lower canonical half | 128 TiB | Future per-process user space |
| 256-383 | `ffff_8000_0000_0000..ffff_c000_0000_0000` | 64 TiB | Kernel DirectMap |
| 384-415 | `ffff_c000_0000_0000..ffff_d000_0000_0000` | 16 TiB | Default kernel heap arena |
| 416-447 | `ffff_d000_0000_0000..ffff_e000_0000_0000` | 16 TiB | General kernel mappings |
| 448-463 | `ffff_e000_0000_0000..ffff_e800_0000_0000` | 8 TiB | MMIO mappings |
| 464-479 | `ffff_e800_0000_0000..ffff_f000_0000_0000` | 8 TiB | Kernel stacks and guard pages |
| 480-495 | `ffff_f000_0000_0000..ffff_f800_0000_0000` | 8 TiB | Per-CPU data and modules |
| 496-510 | `ffff_f800_0000_0000..ffff_ff80_0000_0000` | 7.5 TiB | Reserved for future use |
| 511 | `ffff_ff80_0000_0000..=ffff_ffff_ffff_ffff` | 512 GiB | Fixed kernel image region |

The non-canonical hole between the lower and upper halves is never allocatable.
The current linker places the kernel beginning at `ffff_ffff_8000_0000`, which
lies inside PML4 entry 511.

The lower half is reserved for future user address spaces even if each process
eventually owns a separate page-table root. Keeping a common split makes kernel
mappings and privilege boundaries predictable across address spaces.

## DirectMap capacity

The DirectMap maps physical address `p` to a fixed virtual base plus `p`. Its
64-TiB virtual span can therefore cover physical addresses below `2^46`.
Capacity is checked against the highest physical address that must be mapped,
not the sum of installed RAM: holes in the physical map still consume offsets
in the DirectMap layout.

The kernel reserves the full DirectMap virtual span up to the highest covered
physical address, while leaving physical holes unmapped. All RAM managed by the
physical allocator, along with every physical page used for page tables, must
be covered. If the required physical span does not fit, boot fails explicitly.
MMIO ranges do not have to be part of the DirectMap and use the dedicated MMIO
arena when needed.

## Allocation meaning

`KERNEL_HEAP_ARENA` is the current planned backing arena for the default
free-list heap. It is an implementation choice, not part of the `GlobalAlloc`
contract.

`KERNEL_MAPPING_ARENA` holds page-granular mappings outside the default heap,
including mappings requested by kernel facilities that need a contiguous
virtual range but do not require contiguous physical pages.

The DirectMap is not the default object-allocation arena. A caller receives a
DirectMap address only through an explicit allocation strategy whose contract
states that behavior.
