PHASE 4: ORIGINAL DESCRIPTOR REGISTRATION AND ACTOR MOVEMENT PAYLOAD
=================================================================

Implemented boundary
--------------------
aa-orders::native exposes the complete original 357-entry descriptor table and
the exact wire payload of KKC_TellActorMoveOrderRequest (47) / Decree (66).
No authoritative scheduler, network ingress, gameplay permission bypass, path
selection, collision handling or Mover update is added. The previous history and
dispatch implementation remains sequential.

Registration proof
------------------
Original CRT __cinit 0096EC76 walks the actual function-pointer array
00B21000..00B27964 in ascending address order. All 357 direct callers of the
order descriptor constructor 00766E40 occur in that array. The fixture executes
the original loop, every order initializer, constructor 00766E40 and append
00766DE0. Each native descriptor receives the current count at +4 and appends
itself to the table at [00B27A84]; handler pointer is +0, CString name is +8.

Non-order CRT initializers are deliberately skipped. String allocation,
ASCII-to-UTF16 construction and concatenation are exact-value fixture seams;
native assignment, retain/release and the actual ordinal assignment execute.
Original and accepted v14 whole images are pinned. All visited original code
windows and the unmodified CRT table are independently replayable from the JSON.
No live native process or changed executable is used.

The table has repeated SelectionClearOrderRequest/Decree names and repeated
KKC_EditorChangeGTeamRelationRequest/Decree names. They retain different ordinals
and handlers. The table is not sorted, deduplicated or reconstructed from names.
This is the original/v14 image-specific table, not a portable protocol-version
negotiation claim. The caller must reject a different native protocol version.

Complete movement decoder/serializer evidence
---------------------------------------------
0060F760 decodes a CString through 00410B50 into payload+4, then three32-bit
values into +8/+C/+10 through original 0040D5D0. 0060D240 is the corresponding
serializer, using original 0040D420. The string count is unsigned16 code units;
the string body is read/written in one count*16-bit operation. Its original
byte-block/fragment packing is not equivalent to repeated scalar16 operations.

Forty-five paired native writer/decoder cases cover bit offsets0..8 and empty,
ordinary, embedded-NUL, surrogate and longer selector strings. Raw words include
signed zero, infinities, NaN payloads and arbitrary bits: decoding preserves them
without silently imposing later game-coordinate validity policy. Original
CString buffer growth is an allocator seam; actual004049A0 buffer length/zeroing,
00410B50 string copy, complete0060F760 decoder and0060D240 writer execute.
Rust also compares its new variable-buffer reader against all891 prior scalar
fixtures; malformed/truncated/unsupported inputs remain typed errors.

Complete bounded processing/dispatch evidence
--------------------------------------------
0060F830 is the actual decree handler. It decodes, invokes0061E830 and may mirror
to another output stream. Fixtures disable the external mirror flag, execute
the real handler, and disclose logging/diagnostics as observational seams.

0061E830 runs the following sequence:
1. Call0040EAA0. False returns immediately.
2. Resolve actor via004F8E90(&selector). Null returns immediately.
3. Actor virtual+34 receives command ID5. False returns immediately.
4. Construct actor command using004CA760 ->004D9E20, then pass it to004D9B60.
5. Destroy temporary command with004CA2D0. Dispatcher return is ignored.

The command has flags5C when a definition is present; destination at+18/+1C
comes from the first two payload words. The +20/+24 auxiliary vector comes from
original zero constants00B279C0/C4, and+28 is zero. Payload+10 is decoded but not
used here. Native bool fields+30/+3C are single bytes, not uninitialized padding.
The portable MoveCommand contains semantic data, never native pointers/vtables.

Five process fixtures execute the entire guard sequence and real construction /
cleanup with global permission, actor lookup, actor eligibility and generic
component dispatch as explicit seams. Six additional packet fixtures execute
real0074D710 registry lookup ->0060F830 ->0061E830, including two ordered moves,
every rejection stage and a dispatcher false result. The native by-value packet
reference consumes one owned reference; one of the initial two remains. Rust
uses its safe Arc-backed packet interface without native raw-pointer ownership.

This is explicit-actor movement, not the unported TellSelectedCommandOrder or
TellActorCommandOrder selection/action payload. The source caller0060E890 parses
the selector and three numeric arguments (third defaults to1), but parsing that
console/script argument syntax and request-host authorization are not ported.
The generic process permission and alias lookup stay opaque callbacks: an actor
GTID is not inferred from a string by parsing a decimal/hexadecimal number.

Integration
-----------
Use native::registry with existing dispatch. In its handler call
native::decode_payload(id, cursor). Only MoveDecree may reach process_move;
MoveRequest must enter the separately implemented host/request policy. Unknown
descriptors fail immediately before consuming their payload. Never skip them or
invent a record length. Supply verified actor resolution and command permission
through MoveContext, and deliver command type5 to the actual actor/Mover bridge.
Schedule dispatch only when the recovered admin/world protocol requires it.

Callbacks are trusted and errors remain nontransactional, as in Phase3. Never
automatically replay a partially handled batch. This crate neither chooses a
command cadence nor creates a second authoritative simulation worker.

Reproduce
---------
python research/phase4-orders/recover_orders.py
cargo test -p aa-orders
cargo clippy -p aa-orders --all-targets -- -D warnings

Twelve Rust tests pass (six retained Phase3 tests and six new Phase4 tests).
The original data, accepted v14 runtime and all native profiles remain read-only.
