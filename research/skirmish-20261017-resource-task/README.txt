Bounded ResourceTask owner and queue proof (2026-10-17)

TaskLoad5B1850 now owns one authoritative shared allocation. The Resource entry
and queue retain that same allocation, with native fields/refcounts changed at
their real call boundaries. Queue head order is signed priority ascending and
unsigned sequence descending; duplicate nodes and first-node removal/reordering
are preserved. Growable host storage replaces the allocator pool capacity.

The request/run/block coordinators preserve immediate status, resource flag and
queue mutations. Required services are explicit. The wrapper oracle executes
real Texture3C OR4/OR6, but Resource4C return/required-stop is an expressly
declared backend seam. It does not prove texture/model decode, GPU readiness,
native C++ exception propagation, or an already-running worker's waiter.

The queue notification is a real Condvar-backed auto-reset event state. Native
CreateEvent arguments prove initial-clear auto-reset behavior. The native
Signal+4 byte was not initialized by its constructor; poisoned-byte cases are
retained. The host boolean represents the event kernel state, not that byte.

Immutable native replays: 36+66+6 paired cases, 216 separate original/v14 runs,
all three x87 precision modes. Five exact-source Rust tests pass. The tiny test
host excludes only ResourceObjects' unrelated integration-test declaration and
mechanically extracts the exact pure resource name normalizer. See source pins.
Production scoped cargo check passed; no whole runtime test link was performed.

B282F0's fresh native priority seed is 1 in both PEs. Thirty-five direct-address
references are reads; the audit is not a claim about arbitrary alias writes.
B3DA08 is one actual application enqueue counter. The validated Application
bridge returns a retained ExternalSequence for a real specialized task. Globals
owns integration into the actual LocalBootstrap task; no later actual counter
parity or heterogeneous queue/dequeue semantics are claimed until that separate
integration is accepted.

Run the three native_*.py scripts without flags to verify fixtures without
writing them. build_probe.py prepares the narrow standalone Rust host. The
frozen executable and exact owned production/test sources are retained here.
findings.json records hashes, commands, qualifications and remaining services.
