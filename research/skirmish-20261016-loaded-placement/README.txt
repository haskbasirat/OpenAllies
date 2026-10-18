Existing-object placement descriptor wrapper, 2026-10-16
======================================================

API
---
aa_spatial::loaded_object_placement::check(request, precision, &mut world)
aa_spatial::loaded_object_placement::check_saved(object, precision, &mut world)

This ports original 4F4590, the mandatory placement service reached by saved
Object Init4EF6A0(NULL). check_saved supplies the actual null position/facing
arguments and two zero words. The general wrapper also retains optional live
vector references. Object, definition, vector and temporary-name handles are
typed host identities rather than native 32-bit pointers. No dependency or
original game file was changed. The only shared source edit appends one module
export; lib.rs.before backs up the prior aa-spatial export file.

Exact bounded behavior
----------------------
* Construct the temporary empty-name owner before reading the object.
* Capture current definition, then facing (2 words), then position (3 words).
  Optional vector references are read at their original point; no f32 numeric
  conversion changes signed zero, NaN, infinity or arbitrary saved bit patterns.
* Dispatch actual virtual+B4 using AL truth. Only a nonzero AL calls the CURRENT
  virtual+108. Changes to the object's definition/position during these virtuals
  do not replace the earlier captured descriptor fields.
* Reread current Thing.flags after both virtuals. Descriptor+52 is the complement
  of bit10. All defined constructor dwords and bytes match native A25B28. Byte
  +53 is uninitialized in the native stack descriptor and is deliberately absent
  from the Rust representation; it is never invented as zero.
* Invoke required4F3640 with descriptor, object, object, 0, 0, argument4,
  argument5, preserving native argument order and caller-selected precision.
* On normal return, release the CURRENT descriptor+1C name, including a
  coordinator replacement, then return the original full dword result unchanged.
* A missing/failed callback preserves previous effects and returns its error.
  It does not synthesize a clear placement or substitute4F3C40.

Still required
--------------
The complete4F3640 placement coordinator, real geometry/footprint/required-object
and source dispatch, temporary-name allocation/refcount owner and current-record
storage access are mandatory host services. This module does NOT implement
4F3640 or its return-code5/6/7 subgraphs. Existing SpatialPlacement::verdict is
4F3C40 and cannot replace it. Actual Object/Actor/Team/Kingdom Init binding,
PostCreate, world registration, map publication, battle simulation and networking
remain separate work. This wrapper is not integrated into the runtime yet.

Native failure/SEH unwind, allocation failure and temporary-name destruction
after a callback error are outside this explicit stopped-prefix contract. The
caller owns cleanup of a discarded failed attempt. No native exceptional unwind
equivalence is claimed. The normal-return release service is not a new Rust
CString allocator implementation.

Evidence
--------
native_loaded_placement.py maps the pinned original AA.exe and v14 AA.exe
separately, with page-zero fault guard. Both run at PC24, PC53 and PC64. All111
visited native instructions are byte-matched across the images. Native x87
arithmetic does not occur in this wrapper; no transcendental approximation is
used. The coordinator and virtual/allocator leaves are explicit test seams.

159 rows /318 original+v14 native executions cover:
saved NULL, all four optional-vector combinations, noncanonical AL truth,
flags/signed-zero/nonfinite bits, lazy+108, empty-name/virtual mutation order,
native full-dword return codes, coordinator descriptor/name replacement,
CString atomic refcounts at signed boundaries, and5 explicit service-error
prefixes. The native CString refcount instructions execute untouched; allocator
release is a seam. The Rust fixture host models that required release service;
these cases do not turn it into a production native allocator port.

crates/aa-spatial/tests/loaded_object_placement_native.rs replays every row,
comparing current reads, callback order, coordinator arguments, all defined
descriptor fields, return/error prefix, mutated object state and name ownership.
The second test separately invokes check_saved and refuses missing live flags
after the correct earlier callbacks. Both tests pass. Native replay and final
Rust logs are saved alongside the fixture.

Reproduce from rust-rebuild in PowerShell:
  $env:PYTHONPATH='../reconstruction/tools/python-packages'
  python research/skirmish-20261016-loaded-placement/native_loaded_placement.py
  $env:CARGO_TARGET_DIR='target/s-loaded-placement'
  cargo test -p aa-spatial --test loaded_object_placement_native --release --locked --offline -j4

The fixture is immutable in ordinary replay; --write is only for deliberate
regeneration by the owner. No SDK code, gameplay rule, memory cap, timing,
network budget or release binary is changed by this module.
