# Memory management

This directory records the intended memory-management architecture. Most of
the design is still work in progress; the documents distinguish contracts we
intend to preserve from implementation details that may change.

## Documents

- [`layout.md`](layout.md) defines the kernel's x86-64 virtual-address layout.
- [`paging.md`](paging.md) describes page and page-table types, direct mapping,
  and the page mapper.
- [`allocation.md`](allocation.md) describes physical-page allocation,
  virtual-region allocation, slabs, and the global allocator policy.
- [`bootstrap.md`](bootstrap.md) describes the transition from Limine's initial
  mappings to page tables owned by the kernel.
- [`../module-dependencies.md`](../module-dependencies.md) defines the project-wide
  dependency rules used by the memory subsystem.

## Architectural boundary

Memory-allocation policy belongs to `subsystems::memory`. Portable subsystem
code depends only on contracts in `arch::common`; it must not name
`arch::x86_64` types directly. The concrete x86-64 implementation lives under
`arch::x86_64`, and the final assembly layer constructs it and injects it into
the subsystem.

```text
                         main / assembly
                        /              \
                       v                v
                subsystems        arch::x86_64
                       \                /
                        v              v
                          arch::common
```

Architecture contracts use associated types for concrete address, page,
flags, and mapper types. The x86-64 implementations therefore retain their
architecture-specific invariants without leaking them into portable memory
policy. Operations that need architectural context, such as checking a
physical address against `MAXPHYADDR`, are performed through explicit context
or trait operations rather than process-wide mutable state.

General-purpose data structures such as `Bitmap` and red-black-tree mechanics
belong in `util`. Limine response types are temporarily permitted in bootstrap
code during the single-crate phase, but they must not be stored in steady-state
allocator structures.

## Core policy

- The kernel builds and owns its final page tables instead of relying on
  Limine's virtual layout.
- All managed RAM must be reachable through the kernel DirectMap.
- The default `GlobalAlloc` interface does not promise physical contiguity, a
  DirectMap address, or any particular allocation strategy.
- The planned initial global allocator uses a free-list heap in a separate
  virtual arena and backs it with individually allocated physical pages.
- Callers that need a specific strategy use an explicit allocator, such as a
  physical-page allocator, a DirectMap-page allocator, or `Slab<T>`.
- Allocator metadata must not depend recursively on the global allocator.

## Status

The first `arch::common` paging contracts, x86-64 DirectMap/layout types, and
architecture-independent memory-subsystem type boundaries are now defined.
The mapper and allocator operations remain incomplete. Further refinement of
the contracts, page-table reclamation, SMP TLB shootdown, and a recoverable
kernel out-of-memory policy remain open design work.
