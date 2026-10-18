# aa-orders

## Phase 8: every original order, request/decree flow, transport and sync

* `payload`: wire schemas of all 358 native order descriptors (decode and
  encode with one schema per native payload class, `Field` lists), request ->
  decree re-serialization (`reserialize`), explicit `SchemaContext` game data
  and `Resolver` reference resolution. Proven against the original readers
  and writers (research/phase8-orders/order-traces.json, tests/order_traces.rs).
* `flow`: the `TGC_RequestDecreeOrder<T>::ProcessRequest` template and decree
  handlers (checks, decree/mirror writes, synchronizer entries, process call)
  for every descriptor (`flow_table.rs`, generated from order-flow.json;
  tests/order_flow.rs).
* `net::transport`: datagram header, sequencing, ack bits, RTT (tests/net_transport.rs).
* `net::sync`: TGC_Synchronizer desync hash (tests/net_sync.rs).
* `net::admin`, `net::seams`: request/decree streams and histories, host
  execution, client replay, film record/playback, and the order part of the
  tick driver's NetworkSeams (composition test tests/net_lockstep.rs; the
  aa-runtime adapter is research/phase8-orders/network-seams-adapter.rs.txt).
* Changed: `commands::ParameterSet::negative_count` (native negative counts
  read no entries and are written back unchanged); `dispatch_observed`.
See research/phase8-orders/README.txt and research/phase8-net/README.txt.
Process functions (the gameplay/lobby effect of each decree) remain host
callbacks except move/actor/selected/set-player (phases 4/5).


Recovered packet/history mechanisms and the verified native descriptor table for
the independent Rust runtime. The Phase 4 `native` module adds an actor-movement
payload. Phase 5 `commands` adds generic selected/actor-command wire fields,
current-player context and guarded delivery boundaries. This remains neither
a complete gameplay command implementation nor a network protocol or tick driver.

## Generic commands and player context

`commands::decode_payload` accepts the verified selected request/decree84/86,
actor request/decree85/87 and player request/decree320/298. It delegates47/66 to
the unchanged movement decoder. Requests retain separate enum variants; decoding
a request never authorizes it. Other350 descriptors remain unsupported errors.

Supply `RegistrySizes` from the actual command, object, parameter, parameter-set
and extra registries, plus an explicit `DecodeBudget` for parameter entries.
`CommandWire` preserves the eleven-bit presence mask, count-sentinel references,
raw32 scalar/vector values, optional parameter list and both boolean fields.
Mask bit10 is retained even though it has no conditional field in this decoder.
The reference values are registry indices, not authored IDs or native pointers.
After decoding, the runtime must perform actual registry resolution and the
object-definition virtual type check; no default registry ordering is invented.
A decoded null parameter-set definition is not a usable native object: the native
constructor dereferences it. Treat that as an invalid resolution, not a new set.

`CommandWire::encode`, `ActorCommandWire::encode` and `WireWriter` reproduce the
original raw packing. They do not finalize packet headers, assign authority,
send a request or choose a tick. Writers start with zero storage at an explicit
`Position`. Errors are nontransactional. Actor-command payloads append only a
16-bit slot. `ActorSlotReference` is intentionally different from generation-tagged
GTID: native lookup requires an occupied slot and virtual+B4, without generation
comparison. No numeric-selector conversion is performed.

`process_actor` preserves native permission -> linked-GTID test -> optional group
preparation -> command delivery. The permission callback can replace the resolved
command. `process_selected` checks the established player's selection, retains an
explicit first-actor/default-command inspection callback, then passes the whole
selection to the still-unported0x4ED360 operation. It does not distribute commands
itself, choose formation offsets, resolve paths or manufacture selection ownership.

`process_set_player` requires exact ID+film-mode lookup in native list order.
For live players, an authorizing admin rejects a different ordering client before
mutation. Accepted live players update their client's associated player then the
admin current player; film players update only current player. Missing live players
clear current player. Missing film players call film-stop, then store the abort
flag, preserving callback mutation ordering. These are explicit host callbacks,
not a bypass of request authentication or a replacement network session.

Phase5 evidence is in `research/phase5-orders/README.txt`. Three mixed native
packets exercise actual dispatcher order298 followed by87/86, including an empty
selection; the earlier Move/history tests remain unchanged.

## Verified movement payload

`native::DESCRIPTORS` contains the original 358-entry CRT registration order.
The20261017 correction recovers omitted initializer9E65F0/OrderVersionAckRequest
at323; earlier323..356 shift to324..357. See the correction evidence in
`research/skirmish-20261017-order-registry-correction/README.md`. The direct
acknowledgement is not a generic request/decree template and has no Shape.
Several names occur twice: use the descriptor index, never a name as its identity.
`native::registry()` supplies this table to `dispatch`. The encoded index is nine
bits. Movement request 47 and movement decree 66 share the complete payload
decoder but have different roles; `native::decode_payload` keeps them distinct.
The other356 payload handlers in `native::decode_payload` explicitly return `UnsupportedNativeDescriptor`.
An unknown payload cannot safely be skipped because its length is not known.

`MoveOrder` retains an unsigned16-length UTF-16 selector and three raw32 words.
Its string uses a single native bit-buffer read; decoding each character as a
separate scalar would produce the wrong cross-byte packing. Embedded NULs and
unpaired surrogate units are preserved. Do not parse this selector as a numeric
actor ID: native `0x4F8E90` performs alias/GTID lookup, a separate actor subsystem.
The native process uses the first two words as XY and ignores the third; no
meaning, speed effect or finite-float constraint is invented for that third word.

For an accepted movement decree, `native::process_move` reproduces the exact
`0x61E830` guard sequence through a caller-supplied `MoveContext`: global process
permission, actor resolution, actor command-type5 permission, then command
delivery. It constructs the destination pair, two zero auxiliary words and a
zero +28 word. Delivery does not imply successful movement: the native function
ignores the generic actor dispatcher result. The actor command component,
permissions, alias resolution and verified scheduling point must be integrated
separately. This is the explicit `KKC_TellActorMoveOrder` path, not a claim that
all normal selection/right-click commands use this payload.

See `research/phase4-orders/README.txt` for exact instructions, native fixtures,
source identity and the omitted request authorization/mirror paths.

| API | Native evidence | Supported behavior |
| --- | --- | --- |
| `Cursor::read_uint` | `0x40D5D0` | Exact byte-block/fragment scalar packing, widths 0–32. |
| `read_possibility` / `registry_bits` | `0x411FA0`, `0x40C7C0` | Registry index width for 1–65,536 possibilities and checked index decoding. |
| `dispatch` | `0x74D710`, `0x766F20`, `0x766CE0` | Ordered descriptor decoding, resume position, two-flag abort gate, opaque handler boundary and join-specific diagnostic suppression. |
| `History` | `0x74EC30`, `0x74CDB0`, `0x750880` | Packet window, biased ring indexing, capacity doubling, extension, shrinking end and monotonic retirement. |
| `consume_next` | `0x74FD10` | One history entry, scratch/resume lifecycle, reread current index, retain history, then advance. |

The implementation has no authoritative worker pool. Dispatch and mutations
remain sequential. No wall-clock timing, command cadence, deduplication policy,
gameplay payload, native registry order, game action or purchase is inferred.

## Runtime integration

1. Construct a `History` with the actual retained index and required capacity.
   Store an already obtained packet using `put(index, Some(Packet::new(bytes)?))`.
   `None` represents the original null packet; `[5]` is a nonnull header-only
   packet with zero body bits. They remain distinct.
2. Supply `Registry::new(names_in_verified_registration_order)` and your verified
   payload handlers. Registry names are caller data, not discovered by this crate.
   The original type count determines the encoded index width.
3. Call `consume_next(&mut history, &mut client_state, decode_callback)` when the
   independently verified admin scheduler requests it. The callback can connect
   `dispatch(packet, resume, &registry, diagnostics, &mut gate, handler)` to the
   real gameplay implementation. Each handler uses `Cursor` for its known fields.
4. Retire history only when the caller's verified lifecycle permits it. Consuming
   a packet does **not** erase that packet. The native host's 1,000-entry retention
   policy and its packet-assembly code are not implemented here.

Both `Gate` fields retain opaque source roles: session+0x24 and admin+0x2E5.
The dispatch loop decodes the next descriptor before testing `session && admin`.
If true it stops before executing that descriptor. The outer client boundary
still resets resume/scratch fields and advances its index after this stop.
This is deliberately not renamed to a guessed pause, replay or cancellation API.

`ClientState.field_250` is copied into `field_284` before decode. Resume byte/bit
fields correspond to +0x248/+0x24C; byte=-1 selects the normal packet start.
After a successful decode callback the native code rereads the current history
index, ensures it exists, resets both resume fields to -1 and scratch fields to
zero, then increments that reread index. A callback may mutate index/history;
`consume_next` preserves this behavior through an owned `Arc` packet reference.

## Packet details

The first three bits encode the trailing unused-bit count. Body begins at bit3.
The cursor preserves native byte/bit state, including bit offset8 instead of
normalizing every byte boundary to the next byte. Cross-byte scalar reads are
not a conventional continuous LSB-first accumulator: whole middle bytes occupy
low result bytes and the first/last fragments are combined above them. Using a
generic bit-reader without the recorded native fixtures would corrupt indices.
`with_bounds` exposes the primitive serializer boundary; `for_packet` additionally
validates the three-bit header and legal body extent/resume position.

`History` reproduces `(index_bits + 0x80000000) % capacity`, not `index % capacity`.
It supports non-power-of-two initial capacities. Growth doubles until sufficient
and relocates only the retained logical window. Shrinking end does not immediately
clear stale slots; re-extension nulls those slots, matching the native assignments.
Retirement clears old retained entries and can move beyond end to an empty window.

## Safety boundary and limits

Native finite, representable inputs are the parity domain. Malformed padding,
truncation, invalid registry index, backward retirement, index exhaustion and
non-progressing handlers return typed errors instead of reproducing native
assertions, access violations, signed overflow or an infinite dispatch loop.
The native scalar-width routine itself only represents registry counts through
65,536; this is a format-function boundary, not a gameplay unit limit.

Callbacks are trusted engine code. Errors are nontransactional: earlier handler
effects may already exist and scratch fields may have changed. **Never retry a
failed partial batch automatically.** `NoProgress` is detected after one handler
invocation. No rollback of external gameplay is claimed. Native diagnostic
callbacks are represented as observational events; their hypothetical side
effects are outside the contract.

History allocation uses checked capacity arithmetic and fallible reservation.
There is no fixed packet-count or memory-budget cap. Packet ownership, snapshots,
event traces and caller handlers still consume memory; the runtime must apply its
own observed resource budget. No whole-process OOM recovery is claimed.

## Evidence

`research/phase3-orders/original_orders.py` runs unchanged original x86 in isolated
Unicorn: 891 cursor cases, 15 registry widths, 14 dispatch scenarios, seven history
sequences and eight complete client-boundary scenarios. It executes real packet
reference counting, history growth/copy, CRT vector construction/destruction and
wide-string comparison. Allocator/free/log sinks and diagnostic callbacks are
explicit boundaries. Synthetic handlers call the real bit-read function for a
chosen opaque width; they do not represent actual gameplay decrees.

The Rust tests compare field values, cursor representation, descriptor/handler
order, diagnostic positions, physical history slots and logical window entries.
Additional negative fixtures exercise fail-closed behavior and nontransactional
partial errors. Callee-saved registers and stack balance are checked in the x86
producer. All selected code windows are hash-bound and unchanged in accepted v14.

Run `cargo test -p aa-orders` and
`cargo clippy -p aa-orders --all-targets -- -D warnings` from `rust-rebuild`.
Original files, existing releases, profiles and the native game are not modified.
