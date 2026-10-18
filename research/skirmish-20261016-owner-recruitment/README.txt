SAVED KINGDOM ORGANIZATION RECRUITMENT SCANS -- 2026-10-16

Original4E1430 scans the actual Kingdom+2DC HQ vector (+2E0 length) and
calls HQ+D8 component573E50(definition), stopping at the first true AL.
Original4E1E40 calls HQ+D8 component574BF0(Actor+4,false) for every entry;
only successful entries resolve Organization+D0 through the global generation
table, and entries different from that resolved actor increment the result.
Both routines snapshot the count once but reread the current vector pointer
per iteration. Definition pointers in the second loop are reloaded each time.
Duplicates are preserved. The port uses owned growable vectors/typed IDs.

aa-actors::organization_recruitment ports those loops with explicit leaf
services. saved_owner_recruitment binds the actual SpecialShell.vectors[2DC]
of the registered saved Kingdom. Empty constructed vectors return false/zero
without inventing any HQ payload. A present HQ still fails explicitly at
573E50 or574BF0/574810; full HQ recruitment is NOT implemented by this change.
4E1E40's empty-vector path never reads its actor argument, as in the native.

native_owner.py executes both original and v14 machine code at PC24/53/64:
600 rows /1200 executions. It verifies captured count and lazy entry reads
with mutation of the vector pointer/count after the first callback, duplicate
entries, success/failure and low-byte return values, changing definitions,
valid/stale generation resolution and exclusion of the currently referenced
HQ. Every executed original instruction is checked against both executables.
The v14 image is mapped within the original address range used by these
routines; no extended v14 patch section is entered. Page zero faults except
FS-prefixed accesses via the shared native_machine.py harness.

The fixture replay test and separate partial-callback failure test pass.
The runtime real-source test loads tutorial02 in explicit native mode1 using
InspectPartial ONLY as an owner source, then checks the actual empty HQ vector
and explicit service failures for a deliberately inserted test HQ entry. It
does not claim complete tutorial restoration, valid HQ construction, battle
publication or recruitment execution. Runtime test log is runtime-test.log.

Native instruction ranges:4E1430..4E147D;4E1E40..4E1EC9.
HQ leaf seams573E50/574BF0 are observed callbacks, not implementations.
Malformed/null HQ vectors and fault-side-effect ordering are unclaimed.
No original game files/profiles/saves or existing releases are modified.
