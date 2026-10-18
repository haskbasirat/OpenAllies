PLAY_SFX definition compiler and Audio source lookup

Production scope
----------------
New child crates/aa-runtime/src/fx_graph_sound.rs is used by the parent's
native FX compiler. 42ADD0 constructs only its common fields. 42C3C0 loads
its common42C1B0 body first, then looping(+40), audio_ids -> Audio(+44),
optional unsupported-loop warning and clear(+40), then AT(+3C). Null Audio
and invalid AT preserve the native preceding stores. No +68 or derived
zero initialization is invented. No first-VFX reference entry is inserted.

find_audio resolves an actual source-registered Audio definition, with
historical one-hop translation and the same source chronology as its Object
owner. It requires actual mode0 registry/source IDS/provenance. Native
GameData48AD48..78 creates its +584 Audio manager through482090(initial capacity64,mode0);
47D880 initializes its translation count0. The Audio vtable+8 is6A71D0 true,
which permits48D2D0 translation insertion. 416370 returns null immediately for
empty IDS, otherwise optionally translates once, then searches4915F0.
Non-ASCII lookup/translated IDs are explicitly unsupported because locale
wide-case folding is not proved by this ASCII source/domain evidence.

Evidence
--------
native_fx_sound.py:99 rows/198 fresh original+v14 executions, all3PC.
Original allocator dispatch,42ADD0,42C1B0,42C3C0, recursive child traversal
and list links execute unchanged. Typed source getters, source identities,
registry responses, strings/enums/hash/allocations and logger calls are
explicit leaf services. Fixtures cover all seven AT values, invalid AT,
empty/missing Audio, looping correction, failures at source getters/find/enum, mixed
VFX/sound recursion and partial child failure. Full committed fields,
references, list heads, warning records and getter/service order are saved.

native_audio_lookup.py:300 rows/600 fresh original+v14 executions, all3PC.
47D880's base constructor and416370/4154D0/4915F0 execute unchanged. Explicit
CString pool/copy/hash/CRT-case transports are seamed. Single-bucket maps
force native collision traversal. Covers empty/direct/missing/case-insensitive
IDs, translation precedence, translation-target failure without fallback,
and one-hop (not recursive) translation. native-manager-bindings.json stores
separate static native instruction/literal evidence for the actual manager
construction/vtable; it is not called full GameData initialization.

Final focused Rust validation: rust-tests-v2.log, both tests pass (2/2).
The first99-row portable replay and actual source test are both included.
The source test resolves tutorial02 saved2696 from the actual map to its
canonical definition and inherited WhenSelected list, then checks the actual
city_building_select Audio identity/provenance at all3PC. Optional
AA_CAPTURE_FX_SOUND writes a new capture using create_new; its output is
compared with unchanged original/v14 compilation by native_actual_fx_sound.py.
Its3 all-PC paired rows/6 native executions match the captured Rust compiled
fields, references, roots and warning records exactly. The earlier failed
rust-tests.log records unrelated shared cfg-test source errors, later repaired.
Final successful test process was reaped before this acceptance.

Limits
------
This compiles definitions only. It does not decode/play audio, create an
audio device, change a file, run an active battle or certify all FXC types.
The parent owns common loader/list-context integration. A successful earlier
list compilation is not a claim that a later unsupported type is complete.
No proprietary SDK implementation was read or copied for this leaf.
