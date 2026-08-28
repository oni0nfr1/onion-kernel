# Memory management design

This document records the intended memory-management architecture. Most of
the components described here are not implemented yet.

The top-level layering and allowed dependency directions are documented in
[`module-dependencies.md`](module-dependencies.md).

## Module boundaries

Memory policy belongs to `subsystems::memory`. Architecture-specific page
table manipulation remains in `arch::x86_64::paging`, and general-purpose data
structures such as `Bitmap` remain in `util`.

The kernel may depend directly on the Limine boot protocol during the initial
implementation. When the project is split into crates, the final assembly
layer should translate Limine responses into protocols defined by
`interfaces`, and the memory subsystem should consume boot information through
those interfaces.

## Physical frame allocator

The initial physical frame allocator manages page-aligned portions of Limine
`USABLE` memory with one `Bitmap` per physical-memory region. It allocates one
frame at a time; contiguous-frame allocation is not currently planned.

After successful initialization, a normal frame-allocation request can fail
only when no managed free frame remains. Bitmap storage, region descriptors,
and other bootstrap metadata are reserved before the allocator becomes
available, so allocating a frame does not require allocating more allocator
metadata.

Initialization can still fail independently because of an invalid memory map,
arithmetic overflow, or insufficient usable space for bootstrap metadata.
Invalid or duplicate frame deallocation is a caller contract violation rather
than an out-of-memory condition.

## Virtual-region index

The virtual-region allocator divides a bounded virtual-address arena into
`Free` and `Reserved` regions. Its address-ordered red-black tree contains all
region descriptors, while its size-ordered red-black tree contains only free
regions.

Each region descriptor therefore has two independent sets of tree links:

```rust
struct VirtualRegionDescriptor {
    start: VirtualPage,
    page_count: usize,
    state: RegionState,
    address_links: RbTreeLinks,
    size_links: RbTreeLinks,
}
```

The address tree supports overlap checks and adjacent-region merging. The size
tree uses `(page_count, start)` as its key and supports allocation searches.
Keeping reserved descriptors in the address tree ensures that releasing a
region does not require allocating a new descriptor.

## Region-node storage

Region-tree nodes must not be allocated through `KernelHeap`, because growing
the heap can itself require a virtual-region allocation. Such an allocation
would recurse into the global allocator or deadlock on allocator locks.

The node pool is instead backed by dedicated slabs obtained directly from the
physical frame allocator. When no free node slot remains:

1. Allocate one physical frame.
2. Access it through the Limine higher-half direct map (HHDM).
3. Initialize it as a region-node slab.
4. Divide the remainder of the frame into fixed-size node slots.
5. Add those slots to the node pool and continue the interrupted operation.

This path neither requests a virtual region nor invokes `KernelHeap`. Slab
pages remain owned by the node pool and are not reclaimed in the initial
implementation.

An allocation that may split a free region must determine how many additional
nodes it needs and grow the pool before modifying either tree. Splitting one
free region into a free prefix, a reserved region, and a free suffix requires
at most two additional descriptors. If preparation fails, both trees must
remain unchanged.

## Initial node-pool exhaustion policy

If the node pool is empty and the physical frame allocator cannot provide a
frame for another slab, the initial kernel treats the condition as an
unrecoverable kernel out-of-memory event and panics.

This is an explicit temporary policy. At that point the node pool cannot
describe further changes to the virtual-address arena, and the initial kernel
has no process termination, memory-pressure notification, reclaim, or other
meaningful recovery mechanism. Continuing the allocation with incomplete
metadata would corrupt both red-black-tree indexes.

Releasing a reserved virtual region must not need a new node: it changes the
existing reserved descriptor to `Free`, merges it with adjacent free regions,
and returns redundant descriptors to the node pool. Therefore the temporary
panic policy applies to node-pool growth during allocation, not to a normal
release operation.

Future work must reconsider this policy and may include:

- fallible kernel allocation APIs for operations that can propagate OOM;
- an emergency reserve of region-node slabs for critical paths;
- reclaiming caches and other disposable kernel memory;
- a kernel-wide OOM policy that distinguishes critical and non-critical
  requests;
- terminating or notifying a process once process isolation exists; and
- handling global-allocation failure without assuming every request is
  kernel-critical.

Until one of those recovery mechanisms exists, failure to grow the region-node
pool is intentionally a kernel panic rather than an ordinary virtual-region
allocation error.
