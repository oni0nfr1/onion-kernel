# Memory bootstrap

Limine supplies a working address space, but the kernel does not assume that
its page-table layout matches the policy in [`layout.md`](layout.md). Bootstrap
therefore constructs a new hierarchy before steady-state memory management
begins.

## Required boot information

The initial implementation obtains at least:

- the Limine memory map;
- the Limine HHDM offset;
- the kernel's physical and virtual load addresses;
- framebuffer and other early device mappings; and
- x86-64 `MAXPHYADDR` from CPUID.

`boot::limine` owns Limine requests and exposes separate functions that copy
responses into physical-address-based resource descriptors. Neither the memory
subsystem nor the final runtime objects retain Limine response references.

The current kernel image is linked at its final virtual address, so the boot
layer's image-preparation step does not copy or relocate it. That no-op must
eventually validate the reported load address, complete image range, alignment,
and linker section boundaries. A new page-table hierarchy must still map those
physical image pages at the linker-selected virtual addresses.

## Transition sequence

1. Continue executing on Limine's page tables and stack.
2. Validate canonical-address, four-level-paging, physical-address-width, and
   DirectMap-capacity requirements.
3. Normalize the memory map and reserve the kernel image, boot metadata,
   allocator metadata, and every other early allocation.
4. Bootstrap the physical-page allocator using page-aligned `USABLE` regions.
5. Allocate and clear a new level-4 table and its intermediate tables through
   the Limine HHDM.
6. As one core address-space reconstruction operation, construct the kernel
   DirectMap, map kernel ELF sections with section-appropriate permissions,
   and allocate and map a new kernel stack with guard pages.
7. Ensure that the transition code, current execution path, and destination
    stack remain valid across the CR3 write.
8. Load the new CR3, switch to the new stack, and finish initializing the
    steady-state memory subsystem.
9. Individually reserve and map discovered framebuffer or other device regions
   in the dedicated MMIO arena, then convert their physical descriptors into
   runtime descriptors.
10. Remove temporary bootstrap mappings once no code still depends on them.

The linker will need symbols identifying at least the kernel image and the
text, read-only-data, data, and BSS boundaries so mappings can receive correct
permissions.

## Boot rejection

The initial kernel rejects boot with an explicit panic rather than continuing
with a partial or ambiguous mapping when:

- the CPU or current mode cannot support the chosen four-level layout;
- a required address is invalid under the detected address widths;
- managed RAM or page-table pages cannot fit in the 64-TiB DirectMap span;
- required bootstrap metadata cannot be placed safely;
- the new hierarchy cannot preserve the executing kernel and stack; or
- a required page-table allocation fails.

The coverage test uses the highest required physical end address, not total RAM
size. Sparse holes below that address remain holes but still count toward the
DirectMap's address span.
