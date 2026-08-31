# Paging

This document describes the x86-64 paging mechanisms used by the memory
subsystem. Concrete formats live in `arch::x86_64::paging`; portable policy
accesses them through contracts in `arch::common`.

The common paging API currently consists of:

- `Address`, `Page`, `PhysicalPage`, and `VirtualPage` capability traits;
- semantic `MappingPermissions` that do not expose hardware entry bits;
- `PageTablePageProvider` for allocating intermediate-table storage;
- `PageMapper` and `DirectMap` operation traits;
- `VirtualPageRange`; and
- `KernelVirtualMemoryLayout`, which names the virtual arenas required by
  portable kernel policy.

The traits use associated types for architecture-specific addresses and pages.
The common module therefore defines operations and relationships without
duplicating x86-64 representations.

## Addresses and pages

`VirtualAddress` accepts only canonical x86-64 virtual addresses.
`PhysicalAddress` is bounded by the CPU's detected `MAXPHYADDR`. The maximum
physical-address width is passed explicitly to constructors and checked
arithmetic instead of being stored in global mutable state.

Both physical and virtual 4-KiB units use the term `Page`:

- `PhysicalPage` identifies one aligned physical page.
- `VirtualPage` identifies one aligned virtual page.

Checked page arithmetic preserves address validity and reports overflow or a
limit violation. Whether a valid physical address belongs to usable RAM is a
physical-allocator policy decision, not an address-type invariant.

## Page-table hierarchy

The four table levels use uniform names while retaining distinct types:

| Type | Conventional name | Leaf mapping size |
| --- | --- | ---: |
| `PageTable<Level4>` | PML4 | Not a leaf |
| `PageTable<Level3>` | PDPT | 1 GiB when huge pages are enabled |
| `PageTable<Level2>` | PD | 2 MiB when huge pages are enabled |
| `PageTable<Level1>` | PT | 4 KiB |

Each page table is 4-KiB aligned and contains exactly 512 entries. Indexed
entry access returns `None` for an index outside `0..512`.

Level traits are sealed so external code cannot claim an unsupported page-table
level. Non-leaf behavior is constrained separately from behavior valid on leaf
entries.

## Entries and flags

`PageTableEntry` stores the physical page address and hardware-defined bits.
`PageFlags` represents combinations of those bits. The initial implementation
actively uses `PRESENT`, `WRITABLE`, `USER_ACCESSIBLE`, and `NO_EXECUTE`, while
preserving other representable hardware bits rather than rejecting them.

`PageFlags::NONE` means that no flags are selected; it does not mean that an
entry is empty. `PageTableEntry::empty()` is the representation whose address
and bits are all zero.

- `contains_flags(flags)` tests whether every selected flag is present.
- `matches_flags(flags)` tests exact equality of the represented flags.
- `insert_flags(flags)` and `remove_flags(flags)` modify one or several flags.

Huge-page flags are meaningful only at levels 2 and 3. APIs may accept the
same flag representation across levels, but level-specific operations must
validate the semantics at the point where the entry is used.

`PageFlags` remains private to the architecture-facing paging API. A subsystem
requests writable, user-accessible, and executable semantics through
`MappingPermissions`; the x86-64 mapper adds `PRESENT` and translates those
semantics to its leaf flags.

## DirectMap

During bootstrap, Limine's higher-half direct map is called the **Limine HHDM**.
After switching page tables, the fixed mapping owned by this kernel is called
the **kernel DirectMap**. HHDM is a kind of direct map; the separate terms make
ownership and lifetime explicit.

Only a DirectMap supports arithmetic conversion between a physical address and
its corresponding virtual address. A general virtual address is translated by
walking page tables. There is no unique inverse from a physical address to an
arbitrary virtual address because the same physical page may have multiple
mappings.

Successful DirectMap address conversion means that the address lies inside the
reserved offset span; it does not by itself prove that a page-table entry is
present. Code may dereference the result only when the relevant physical page
is known to be mapped. This distinction permits physical holes to remain
unmapped inside the reserved DirectMap span.

The current page-table implementation requires every page-table page to be
reachable through the active DirectMap. An entry contains only the physical
address of the next table, so the walker adds the DirectMap base to access it.

## Page mapper

A mapper owns or borrows the following context:

- the root `PhysicalPage` read from or intended for CR3;
- access to the active DirectMap;
- the detected maximum physical-address width; and
- a provider that can allocate and release physical pages for intermediate
  page tables.

The x86-64 implementation represents the address conversion with
`KernelDirectMap` and implements the documented four-level arena policy with
`FourLevelKernelLayout`.

Mapping a 4-KiB page walks levels 4 through 1. For each missing intermediate
table, it allocates a physical page, clears it through the DirectMap, and links
it with the required flags. Unsupported huge mappings are rejected. A present
leaf produces an `AlreadyMapped` result instead of silently replacing it.

The operation must be transactional: if a later allocation or validation
fails, tables and entries created by the attempted mapping are rolled back.
The exact page-provider and rollback interfaces are not finalized yet.

Unmapping clears the leaf and invalidates the affected address in the local
TLB with `invlpg`. Reclaiming now-empty intermediate tables is planned but its
policy is not finalized. Multiprocessor support will also require remote TLB
shootdown; local invalidation alone is sufficient only during the current
single-CPU stage.

Reading and writing CR3 are architecture mechanisms. Writing CR3 requires the
caller to uphold the safety contract that the new root is valid and preserves
the executing code, stack, and required mappings.
