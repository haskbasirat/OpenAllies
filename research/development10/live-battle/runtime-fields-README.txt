Native World runtime fields and BoolSource

World ctor/init auxiliary fields and actual-byte BoolSource extension only; live_battle remains an unexported integration draft.

Native4F7D41/4F8042 writes+4=null,+275=0. Native Init tail resets+275, reads/stores actual SVar GameSpeed+278, allocates52 bytes, and actual539F30 zeroes thirteen words before replacing+4. Every executed nonseam instruction is pinned unchanged in original/v14. Allocator/destructor/free and initial tail registers are explicit inputs/seams. Production supports only fresh-null positive allocation, retaining failure prefix.

The same World owner lends actual first auxiliary word to the clocked counter. New BoolSource reads actual u8 from source/World without constructing a detached bool mirror. Existing bool fixtures remain compatible.

Scoped tests:2 new runtime-fields and7 unchanged live_clocked pass in runtime-fields-tests-v1.log. Exact source/evidence hashes are runtime-fields-findings.json. Whole application/battle integration is separate.
