Native trigger reconstruction: current acceptance scopes
=======================================================

aa-triggers now owns checked serialized records, the ordered world trigger
manager, eight actual manager-local operations and eight world-action wrappers. This is a substantive
engine module, not a playable tutorial or a claim that all115 opcodes work.

Data and native identity
------------------------
registrations.py executes original545EC0/549440/549720 registration calls, plus
original4C5DC0/4C6E40 schema initialization and548920 storage classification.
The result contains115 element IDs,45 argument classes and64 item kinds. Rust
tables are emitted from those captured values. Every relevant native function
and referenced original/v14 string is byte-pinned; source declarations are not
inferred from translated UI labels or source comments.

SavedManager::decode supports manager5 /Trigger4 /Element3 /Parameter4. It
preserves raw binary32/word values, enabled/repeat bytes, UTF16 strings, exact
source spans and source record order. Unknown versions and unresolved argument
defaults fail explicitly. Caller input budgets guard decoding allocations;
they do not cap gameplay entities or RAM. The caller verifies the enclosing
MapArchive chunk identity and supplies the native localization stream mode.

The original tutorial01.RMP has101 triggers,40 element kinds and9 initially
enabled records. tutorial02.RMP has84 triggers,38 element kinds and13 enabled
records. Both chunks decode completely. native_saved_writer.py reconstructs
the native memory fields and calls actual540A90→542B00→548C10 serialization:
all185 individual trigger records reproduce their original bytes exactly.
Only the stream interface and external MAPSTRINGS registration/save-context
leaves are replaced. Header/variable-table serialization ordering is outside
that native writer comparison. Inline localization and older save versions
are separate scopes; external MAPSTRINGS is the actual supplied tutorial mode.

Manager and evaluation
----------------------
Manager owns trigger live state, variables, saved strings/next-ID, deduplicated
temporary subject IDs and the incoming event queue. It ports54C280/5411D0,
54BDC0 iteration and4DB300 cleanup. Event and condition lists never short-
circuit. Conditional mode1 requires all conditions; other modes require any.
Actions run in order even after a false result. The post-action repeat flag and
current time are reread. Activation only captures time on disabled→enabled.

After each enabled trigger, the native temporary field/list/cursor reset is
preserved, including event-gate failures. The final event drain frees each node
before its payload and stops after popping a null-payload node, leaving later
nodes alive. Callback-produced mutations are retained. A failed manager update
becomes stopped so replay cannot silently duplicate partial effects.

The67 native manager cases execute the original manager/evaluator control flow
and cleanup;546AF0 is a documented boolean/mutation seam for this gate. Trigger
order and element lists are immutable during an update; editing/deleting those
native list nodes during callbacks is not claimed. These cases do not establish
gameplay support for any element opcode by themselves.

CoreProgram separately implements actual546AF0 opcodes0/1/16/80/81/91/92/93:
true, numeric comparison, activation time, trigger enable/disable and variable
set/increment/decrement. Actual541900 validity uses the recovered parameter
count/classes. Numeric helpers retain original int conversion, precision and
binary32 stores; comparisons evaluate the right operand before the left.
Missing variables produce the original diagnostic/zero or failed update behavior.
Nonfinite values and unproved variable-ID locale cases fail explicitly.

327 native PC24/53/64 cases execute actual546AF0, native validation/schema setup,
parameter register conversion, trigger lookup/time writes, comparisons and
variable arithmetic. String ownership, dictionary access/insertion, heap and
logging are explicit seams. Field writes execute in the original image. These
are distinct from the manager's sequencing fixtures.

World integration still required
-------------------------------
WorldProgram now composes the actual control and property-admission modules
for opcodes54/61/62/63/64, stores actual presentation fields for43/44, and sets
the canonical Kingdom victory byte for114. All201 native world-action cases
match. WORLD.txt names precise boundaries and required canonical runtime
services. Name/definition lookup, command handler and property insertion must
still bind to the ActorStore. Unsupported opcodes and Kingdom context selectors
fail explicitly. Speech/objectives and live actor event/condition production
remain separate work; the complete tutorial has not been run successfully.

Current tests: eight Rust tests across saved records, manager ordering, native
intrinsics and world actions. Strict aa-triggers all-target Clippy passes. An initial check was
blocked by an aa-mapworld dependency lint; its owner fixed it, and rerun passed.
Independent review and final Windows executable integration remain separate.
Original game/assets/releases/saves are read-only throughout this work.

Reproduce in order:
  python research/phase7-triggers/registrations.py
  python research/phase7-triggers/generate_registrations.py
  python research/phase7-triggers/decode_census.py
  python research/phase7-triggers/native_saved_writer.py
  python research/phase7-triggers/build_saved_fixture.py
  python research/phase7-triggers/native_manager.py
  python research/phase7-triggers/native_core_program.py
  python research/phase7-triggers/native_world_program.py
  python research/phase7-triggers/record_findings.py
