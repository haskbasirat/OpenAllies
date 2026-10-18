Phase 6: serialized Actor and Properties payloads
================================================

Production: crates/aa-mapworld/src/actor_payload.rs
API: ActorPayload::decode_prefix(bytes, localization_mode) starts at the actor
version after the complete GObject payload. ActorTail::decode_prefix starts after
ActorPrefix. Both return the exact consumed byte count without scanning for a
following record. This is a wire decoder, not a complete runtime actor restore.

Native evidence
---------------
native_actor_payload.py executes original x86 instructions using the pinned
original AA.exe; every observed native instruction and named function range is
byte-identical in the accepted modified v14 executable. Neither image is changed.
The corpus contains 112 synthetic cases across actor versions 1..14 plus source
Crete and DeveloperMaps/axis00_dunkirk records. Each runs from both 4D919C (the
tail entry, with the original frame locals restored) and 4D8CF0 (the actor layer
entry with an explicit GObject::Load seam). Total: 228 native executions.
Both stop at 4D9714, immediately before post-load runtime fixups. The full function
disassembly identifies the later handle/name/number fixups separately.

Original 4D8290 component dispatch, 4CFB00 component validation, 581AE0 Properties
Load, 50BCD0 Updatable Load, 8589B0 version helper, and 49AD70 localized-string
branch decisions execute. Stream transport, string storage/allocation, component
allocation, Properties dependency callback, localization/owner lookups, log output
and Updatable scheduling cleanup are fixture seams. The restored definition
component mask and 42 non-transient state descriptors are fixture inputs.
executed-instructions.json records observed instruction addresses including seam
entry addresses; those seam entries are intercepted before their implementation
executes. This is not a complete live-world or unmocked original-engine test.

Recovered layout and version gates
----------------------------------
ActorPrefix: version <=14, owner handle, low state word, and high state word at
version >=9. The earlier mapworld native fixtures separately cover owner lookup
and old-state OR behavior. This corpus also executes prefix and tail together.

At actor version >=1, the first mask is DEFAULT components and the second is
SAVED components. Native 4D91D3/4D91D8 pass their addresses to 4D8290 in that order
as arguments four and three respectively. This matters: Dunkirk default=0x40,
saved=0 consumes no Properties wire data, while Crete default=saved=0x40 does.
Native load order is Construction(15), then slots0..30 except15/4, then Mind(4).
Only a saved mask of 0 or Properties(0x40) is supported. Unknown saved bits are
explicit errors because the stream has no component length with which to skip.
Default bits are retained raw; decoding does not instantiate default components.

Properties consists of Updatable, a version<=1, an unsigned count and per-entry
property class words. Class0xffffffff has no object payload. Other classes are
explicitly unsupported until their virtual Load formats are recovered.
Updatable version0 has NO ID word (4 bytes total); version1 has a signed ID word
(8 bytes total). In 50BCD0 the version helper reuses the original argument stack
slot, and 50BCFA tests that version before reading the ID. Negative IDs do not
register an updatable. The decoder preserves IDs without registering them.

After component payloads, native reads a current-component signed index.
Version>=2: word158.
Versions3..11: legacy word15C, which runtime then forces to -1.
Version>=4: byte presence and optional u16-length UTF16 owner lookup string160.
Version>=5: byte presence and optional word164.
Version>=6: localized key UTF16. Only map localization mode0 is supported here.
Version>=8: words168/16C.
Version>=10: word170.
Version>=11: four counted lists at runtime offsets118/120/128/130. The first
three read 4 bytes per entry; the last reads 12 bytes per entry. Native loops
interpret counts as signed: negative counts consume zero entries. Raw counts
are retained. Positive counts are checked against remaining bytes before reserve.
Version>=13: word174. The serialized actor layer ends here.
Unknown field names intentionally use native offsets rather than guessed roles.

Source boundaries
-----------------
Crete KINGDOMS first actor: version at111, tail at127, end195. Actor bytes84;
tail bytes68. Saved Properties consumes16 bytes (updatablev1/id0, propsv1/count0).
Dunkirk KINGDOMS first actor: version111, tail127, end179. Actor bytes68;
tail bytes52. Saved mask0 does not serialize its default Properties component.
The following word is the separately decoded GKingdom Updatable version; no
following-record search or hard-coded source boundary is used by production.

Tests and boundaries
--------------------
cargo test -p aa-mapworld --test actor_payload: four passing tests.
cargo clippy -p aa-mapworld --all-targets -- -D warnings: passing.
Rust compares all payload fields and consumed lengths against both original
execution entry modes, checks every proper truncation of all112 synthetic tails,
checks suffix bytes remain unread, and rejects unknown components/classes/modes.
Source actual records and instruction pins are in native-actor-payload-fixtures.

Not implemented by this module: actor version0 automatic assembly; serialized
components other than Properties; non-sentinel property objects; nonzero
localization modes; definition-driven component construction; all component,
owner, GTID/updatable registry mutations; transient-state clearing; post-load
reference/name/number fixups; gameplay, save roundtrip or complete world restore.
All raw reference words and UTF16 code units are preserved for those later steps.
