Saved Mind command restoration -- 2026-10-16

Scope
-----
Ported the actual saved-map actor command Load, and connected its decoded
values to the existing Mind saved Load. This is restoration work; it does not
implement a new command simulation or claim a playable battle.

Native evidence
---------------
4D9C80 constructs a 0x40-byte actor command (vtable A28450).
A28450+8 is 4DACE0; the exported Ghidra pseudocode has no separate file for
that function. load-instructions.txt records original executable instructions.
The Load accepts versions 0..3 and reads:

  command definition CString (4DAB60, B3AA58), flags, word10,
  ActorData CString (4C2E60 -> 4BDBA0 category 0x2B),
  position[2], auxiliary[2], scalar, trailing byte,
  boolean byte (v3), FormationData CString (v1+, 4DAC20/B3AB08),
  owned ParameterSet when flags bit5 is set,
  SharedPropertyData CString (v2+, 4DAAA0/B3AA2C).

Unlike network command decoder 61EBA0, these saved scalar/reference fields
are not conditional on flags. Empty CString length gives a null reference;
nonempty strings invoke registry lookup, even if their first unit is NUL.
Null command definition is accepted by saved Load itself. It must never be
replaced with a fabricated dispatch definition.

52C580 appends queued commands in serialized order. The current command in
base version3 is a separate allocation when its byte is nonzero. After the
temporary holds are released, each pending/current command has one reference.
Negative pending counts read no entries. Load does not dispatch commands or
schedule updates.

Code
----
crates/aa-mind/src/saved_command.rs retains every scalar bit and raw boolean
byte, and keeps named source references unresolved. ParameterSet bit5 remains
an explicit unsupported error.
crates/aa-mind/src/saved_load.rs now decodes pending/current commands.
apply_with_commands requires distinct nonzero already-owned command handles,
with one handle per saved allocation. It validates the bindings and all other
saved-state requirements before mutating the constructed Mind. Success moves
ownership into the pending queue/current command without extra retains.
The legacy apply API explicitly rejects command-bearing saves: it cannot
silently drop command payloads. The runtime catalog owns actual source-name
resolution, live command allocation, and transfer into ActorStore/Commands.

Proof
-----
native_commands.py executes original 4D9C80/4DACE0 and 536340/52C580 at all
three x87 precision modes. 369 cases cover versions0..3, arbitrary scalar
bits, flags, empty/nonempty reference names, noncanonical boolean bytes,
bad versions, pending queues0/1/3, separate current commands, null command
definitions in a full Mind, and negative pending counts. Native instructions
are checked against v14 and page-zero ordinary accesses are guarded.
Explicit seams are documented in the producer: byte I/O, CString storage,
allocation, named registry lookup, and the already-proven constructor ability
jitter. Lookup validity against authored definitions is not proven by this
oracle; that is an explicit runtime responsibility.

saved_commands_native.rs replays the native fields, queue order/ownership,
state stack and word stores; it also rejects all proper truncated prefixes
and invalid/missing command bindings without mutating the Mind.
saved_load_native.rs retains the existing full Element saved-state regression.

Real-map corpus
---------------
The prior 647 observed current-command blockers occur inside Organization
member records. A top-level DYNAMIC_THINGS placement scan alone misses them.
The diagnostic dump_restored_mind_commands test uses the real InspectPartial
restore path and exports exact MindPayload bytes from registered embedded
members. native_map_commands.py executes the original readers against those
bytes; native-map-commands.json records the resulting coverage and commands.
InspectPartial is diagnostic and cannot publish a battle.

Completed coverage: 647 command-bearing embedded Element Minds across57 maps,
593 distinct payloads, all647 current commands named "stop", flags0x40, raw
boolean/trailing bytes0, no other definition references or pending commands.
Every original Load consumed the exact payload length and left one current
command reference. Rust replay passed all647 cases. Together with369 synthetic
cases, 1,016 new original-code cases are retained. Final scoped tests:4 new
saved-command tests and1 existing saved-Element regression passed.

Provenance discrepancy discovered: all647 embedded DecodedSavedComponent
source_span offsets are local origins although the field documents an
absolute DYNAMIC_THINGS chunk offset. Example: Kursk horse15284 reports337;
its exact332-byte Mind payload occurs at724619. The native oracle verifies
each complete decoded payload occurs exactly once in the original chunk and
records reported_offset and verified_offset, plus original map hashes. This
checks already-decoded source bytes; it does not infer field boundaries by
scanning for names/prefixes. Production span metadata is outside this patch.

Remaining limits
----------------
Owned ParameterSet payloads, Regiment/Aircraft saved Mind Load, and nonempty
saved bombard-text application remain explicit unsupported errors.
Null command definitions cannot be represented by the pre-existing runtime
PlainCommand dispatch fields without a typed null representation; bounded
runtime rejection is required if that type has not been extended.
Raw u8 booleans are preserved by the decoder. Converting them into runtime
bool truth values does not prove exact saved-game reserialization.
Registry lookup, command dispatch effects and post-load simulation are only
proven where their separate production adapters/oracles explicitly say so.

Commands
--------
Set PYTHONPATH to reconstruction/tools/python-packages; run native_commands.py
and native_map_commands.py in this directory.
Set CARGO_TARGET_DIR=target/s-ai-uai:
  cargo test -p aa-mind --test saved_commands_native --test saved_load_native
    --release --locked --offline -j4
For the corpus, set AA_GAME_ROOT and PROBE_OUT to absolute paths:
  cargo test -p aa-runtime --test saved_payload_probe
    dump_restored_mind_commands --release --locked --offline -j4
    -- --ignored --nocapture
