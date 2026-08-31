# Memory allocation

This document describes the planned allocators and their ownership rules. The
general allocation interface deliberately does not expose an implementation
strategy.

## Allocation layers

```text
GlobalAlloc / KernelHeap
          |
          v
VirtualRegionAllocator ----> region descriptors and node slabs
          |
          v
architecture-neutral mapping contracts
          |
          v
x86-64 PageMapper ----> PhysicalPageAllocator ----> Bitmap
```

The final assembly layer connects the portable allocator policy to the x86-64
mapper implementation through `arch::common` traits.

## Physical-page allocator

The initial allocator manages page-aligned portions of Limine `USABLE` memory.
Each usable physical region has a descriptor and its own `Bitmap`; a set bit
records an unavailable page and a clear bit records a page available for
allocation. Searches wrap around a region so the saved start position acts as
a hint rather than a hard lower bound.

Only single-page allocation is initially required. Contiguous physical-page
allocation is not part of the first implementation.

During bootstrap, the allocator calculates the descriptor and bitmap storage
required by all usable regions. It reserves that metadata as a page-aligned
prefix of one sufficiently large usable region and automatically excludes the
reserved pages from allocation. Consequently, a normal allocation after
initialization fails only when no managed free page remains. Initialization
may additionally fail because the memory map is invalid, arithmetic overflows,
or no usable region can hold the contiguous metadata area. Invalid or
duplicate deallocation is a caller contract violation.

The allocator records the physical starting pages of its descriptor and bitmap
storage instead of retaining bootstrap virtual references. It accesses those
ranges temporarily through its current `DirectMap`. After activating the
kernel address space, `reconstruct_with_direct_map` consumes the allocator and
returns one using `KernelDirectMap` without clearing or copying allocation
bits.

## Virtual-region allocator

`VirtualRegionAllocator` records which page-aligned portions of a bounded
virtual arena may be assigned to a new request. It reserves virtual addresses;
it does not remember which physical page backs each reservation. Page tables
are the source of truth for physical mappings.

The allocator uses two red-black-tree indexes over shared region descriptors:

- an address-ordered tree containing both `Free` and `Reserved` regions, used
  for overlap checks and adjacent-region merging; and
- a size-ordered tree containing only free regions, keyed by
  `(page_count, start)`, used to find an allocation candidate efficiently.

Each descriptor therefore contains independent links for both indexes:

```rust
struct VirtualRegionDescriptor {
    start: VirtualPage,
    page_count: usize,
    state: RegionState,
    address_links: RbTreeLinks,
    size_links: RbTreeLinks,
}
```

Reserved descriptors remain in the address tree. Releasing a region can change
that descriptor to `Free`, merge adjacent free descriptors, and return
redundant nodes without allocating metadata.

## Region-node slabs

Region-tree nodes must not be allocated through `KernelHeap`, because growing
the heap can itself request a virtual region. That would recurse into the
global allocator or deadlock on allocator locks.

The node pool uses dedicated `Slab<VirtualRegionDescriptor>` pages obtained
directly from the physical-page allocator and accessed through the DirectMap.
When no free slot remains it:

1. allocates one physical page;
2. obtains its explicit DirectMap address;
3. initializes slab metadata within that page; and
4. divides the remaining storage into fixed-size node slots.

This path neither reserves a general virtual region nor invokes `KernelHeap`.
Slab pages remain owned by the pool and are not reclaimed initially.

An operation prepares all required nodes before mutating either tree. Splitting
one free region into a free prefix, a reserved region, and a free suffix needs
at most two additional descriptors. If preparation fails, both indexes remain
unchanged.

`Slab<T>` is intended as a reusable fixed-size kernel allocator. Its low-level
API accepts explicitly supplied backing pages; it does not decide where those
pages come from or recursively use the global allocator.

## Kernel heap and `GlobalAlloc`

The current planned global allocator is a free-list `KernelHeap` in
`KERNEL_HEAP_ARENA`. The free list chooses byte ranges; the virtual-region and
mapping layers make the necessary pages accessible. Contiguous virtual heap
storage may be backed by noncontiguous physical pages.

This strategy is an implementation detail. Rust's `GlobalAlloc` contract in
this kernel promises neither:

- a DirectMap address;
- physically contiguous storage;
- a fixed virtual arena; nor
- continued use of a free-list implementation.

Code that requires a strategy uses a separate explicit allocator. Examples
include physical-page allocation, DirectMap-page allocation, slab allocation,
and page-granular virtual-region allocation. Naming APIs after the promised
property is preferred over exposing `kmalloc`/`vmalloc` terminology.

A physical page backing the heap also has a DirectMap alias. While the page is
owned by the heap, other code must not use that alias to access heap objects;
doing so would bypass ownership and aliasing rules.

## Exhaustion policy

If a region-node slab is full and the physical-page allocator cannot provide a
new backing page, the initial kernel treats the condition as an unrecoverable
kernel OOM and panics. Continuing without metadata would corrupt both tree
indexes, and the current kernel has no process termination, reclaim, or
memory-pressure mechanism.

This is a temporary policy. Future options include fallible kernel allocation,
emergency metadata reserves, reclaimable caches, and a kernel-wide distinction
between critical and noncritical allocations.
