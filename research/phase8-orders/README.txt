20261017 CORRECTION: the historical357-entry completeness claim below was
incorrect. Actual CRT9E65F0 was omitted by Ghidra calls.tsv. The current
registry contains358 entries; old323..356 shift+1. The original report/source
snapshot is preserved under skirmish-20261017-order-registry-correction/
historical-357. Current trace/flow artifacts are regenerated using the actual
corrected CRT context; new323 has empty payload but no generic template Shape.
See that correction directory's README.md/findings for current acceptance.

HISTORICAL PHASE 8 - PLAYER COMMAND PATH: 357 RECOVERED ORDERS
=======================================================

Source: original E:\Games\Axis and Alies\AA.exe (SHA256 5d2548a2...0d26),
never modified. Every oracle checks that each executed byte window is
identical in the accepted v14 image (3bda77d7...c5c1). Rust: crates/aa-orders.
Inventory: inventory.json (one row per descriptor and per command type).

1. HOW A PLAYER COMMAND REACHES THE SIMULATION
-----------------------------------------------
  UI / hot command / AI
    -> request emitter, one native function per request (descriptor-write-
       sites.json; e.g. 0x61E960 KKC_TellSelectedCommandOrderRequest,
       0x61EA20 actor command, 0x60DC90 move, 0x60C250 attack): lock the
       NetNode (+8), drop when admin+0x2C0 is clear, default player =
       admin+8, then 0x74B360 writes TGC_SetPlayerOrderRequest (320, 32-bit
       player id) when the player differs from admin+0x210, then the 9-bit
       request id (0x766D00) and the payload into the request stream
       admin+0x254.
    -> admin tick: 0x752DA0 seals the stream into one request packet
       (history admin+0xC, 200 kept); the broadcast-request channel carries
       it to the host (research/phase8-net/README.txt).
    -> host admin tick 0x752E60 -> 0x751B60 per peer: SetPlayer decree for
       the peer (0x7509F0), then 0x74D710 dispatches each request to its
       ProcessRequest (template TGC_RequestDecreeOrder<T>, e.g. 0x61FB40):
       checks -> decree written to admin+0x1F8 -> the decree's process
       function runs on the host -> optional film mirror.
    -> 0x752EA0 seals decree packet N; every client replays packet N at its
       admin tick N (0x74FD10 -> 0x74D710 -> decree handler -> process).
  Gameplay commands are command DEFINITIONS, not separate orders: the 39
  native actor command types (0x4DA2F0: 0 kill, 1 bombard, 2 guard,
  3 repair, 4 construct, 5 move, 6 stop, 7 retreat, 8 rout, 9 explore,
  10 disband, 11 unpack, 12 pack_organization, 13 build_structure,
  14 commission, 15 recruit, 16 set_rally_point, 17 recruit_location,
  18 structure_cancel_queued, 19 construction_cancel, 20 sell, 21 upgrade,
  22 raze, 23 sally_forth, 24 recall, 25 air_attack, 26 air_recon,
  27 escort_on, 28 escort_off, 29 attach, 30 detach, 31 hq_attach,
  32 hq_detach, 33 set_formation, 34 pack, 35 research, 36 cancel_flare,
  37/38 group_behavior_enable/disable) are ActorCommand definitions
  (registry 0xB3AA58) carried by the generic command wire (field order
  0x61E6B0/0x61EBA0) of KKC_TellSelectedCommandOrder 84/86 (the selection;
  0x61F210 -> 0x4ED360) and KKC_TellActorCommandOrder 85/87 (one actor slot).
  Move and attack also have explicit orders (47/66, 46/65). Formations,
  research, recruiting, building, repair, sell, rally points, pack/deploy,
  retreat, stop, special operations all travel this way; their effects are
  the actor command components (not in this crate). Team commands
  (team_commands.tgi, 14 IDS) use KKC_TeamCommand 22/23; metagame commands
  (mg_command.tgi) use KKC_TellMGCommandOrder 355/356; chat uses 58..60 /
  77..79; selection 12..19, 331/332.

2. WHAT IS PROVEN (oracles + Rust tests)
-----------------------------------------
a) Wire encodings of all 357 descriptors   [order-traces.json, tests/order_traces.rs]
   native_order_traces.py runs every ORIGINAL handler under Unicorn. The
   input stream starts empty; each time the original reader 0x40D5D0 is
   entered the harness chooses a value and places it with the 0x40D420
   packing (asserting that the original reader returns exactly that value),
   so the original code itself determines field order, widths and
   conditionals. Requests continue through the original ProcessRequest and
   the original writer into the decree stream; emulation stops at the
   handler's own first 0x4012C0 (decree-type logging), before the process
   function. 8 seeds per descriptor, mixed and small-value policies.
   Result: 2,856 runs; 2,839 decoded to the stop point, 17 are native
   null-pointer faults (below). Rust (payload::decode/encode) reproduces
   every primitive read (width and value, CStrings as one buffer), the exact
   native end position (bit 8 kept), byte-identical re-encoding, and for
   requests every original decree-stream write and the decree bytes after
   payload::reserialize (resolution normalisation).
   Schemas: payload/complex.rs (hand-ported from the native decoders,
   data-dependent: command wire, team command, faction ability, file header
   0x5A1E90 family, world rules 0x511E50, RMC settings, MG command, trigger
   0x6B7E80 family, editor GThing, load-film, host load map) and
   payload/{game,lobby,editor}.rs (fixed sequences drafted by
   gen_schemas.py from the traces, each annotated with its native readers).
   Data-dependent decoding is explicit in SchemaContext: registry counts,
   TeamCommand flags ([[0xB36530]+0x270]+0x24[i]+0x24), WorldRule kinds
   (def+0x20), zero-width list budget. Name lookups (0x76A9C0 -> 0x41EEC0,
   metagame 0x959070) are Field::Named; the original looks names up as C
   strings (only units before the first NUL) and re-serializes the found
   object's own name; an empty 0x41EEC0 key returns null and the original
   writer 0x76A8C0 then dereferences null (Rust refuses).
b) Request/decree control flow of all 357 handlers  [order-flow.json, tests/order_flow.rs]
   native_order_flow.py runs every original handler to completion with the
   process function stubbed (direct call, inline block 0x61FA53..0x61FA9C
   for 87, or the order-object virtual Execute for editor orders, with its
   stack arguments popped). Requests: accepted / no ordering client
   (log line 0x114) / version not acknowledged (0x118) / film session
   (0x11C) / accepted with mirror. Decrees: mirror off/on. Recorded: sync-log
   entries (synchronizer A or S, label, value), rejections, process call,
   mirror query, every decree-stream and mirror-stream write.
   flow_table.rs (generated by gen_flow_table.py) holds each instantiation's
   shape; flow::process_request / process_decree reproduce all 1,242
   returned cases (of 1,248) exactly; the 6 native faults are refused.
   Notable native facts: chat requests (58..60) skip the version/film checks
   and decrees 77..79 are never mirrored; SetPlayer decrees never log to S;
   PlayerJoin suppresses the trailing "o.GetBitSize()"; the host executes the
   decree immediately in ProcessRequest; the mirror re-serializes after
   the process (so a permission callback's replacement is what is recorded).
c) Earlier phases (unchanged, still passing): descriptor table, cursor,
   dispatch, history (phase3/4); move process 0x61E830 (phase4); actor
   process 0x61E220, selected boundary 0x61F210, set-player 0x75D710 (phase5).
d) Synchronizer (desync hash) [research/phase8-net/native-sync.json, tests/net_sync.rs]
   and datagram sequencing [native-transport.json, tests/net_transport.rs].

3. FINDINGS THAT CHANGED EARLIER RUST BEHAVIOUR
------------------------------------------------
* Parameter-set counts (0x4C2C60): the native loop runs only for 0 < count;
  a negative count is stored as the vector size (0x50B530) and written back
  unchanged by 0x4C2050. Phase5 Rust rejected negative counts; it now keeps
  them (commands::ParameterSet::negative_count) with zero entries.
* Request -> decree is not byte-copying: things re-serialize as the resolved
  thing's +0x14 slot (0 when unresolved), players as id (0 when not in the
  session list), definitions as index (count sentinel when invalid), network
  addresses as their 16-bit port, named references as the found object's
  name. payload::reserialize implements this; KKC_EditorReplaceTrigger
  (293/296) is refused: the native reader files elements into three lists
  (0x542A70/0x540F90) and the writer 0x6B7170 emits them regrouped without
  some element fields.

4. NATIVE FAULTS RECORDED (not reproduced, refused in Rust)
-----------------------------------------------------------
* 0x76A8C0 writing a null named reference (empty name key) in 115/117/321
  re-encoding.
* 0x511E50 switching on a null WorldRule definition (sentinel index) in the
  file-header / world-conditions decoders (95, 117, 122, 127, 131).
* Trigger element definition lookup 0x6B76A0 by raw index without bounds
  check (harness supplies 4096 definitions; out-of-range indices fault).
The harness marks a run 'native_null_dereference' at the first page-0
access other than an FS-prefixed access of [0..3] (the SEH link; FS base is 0
in the emulator, research/phase8-net/null_guard.py); later emulation is not
native behaviour.
Re-check after the lead's emulator caveats (round 2): extending the guard to
[0..3] found 96 trace runs (descriptors 64, 83, 91, 93, 94, 95, 117, 122,
127, 131, 321, 322) and 4 flow cases (cheat decrees 30, 35) that had read the
SEH word through objects missing from the synthetic environment
([[0xB36530]+0x4A0] in the settings ctor 0x511D90; [0xB39AB0]+0x1BC/+0x1C0
cheat vars at 0x5FAB6B/0x5FB10B). Both are now supplied; every recorded read,
write, event and status is byte-identical to the previous fixtures. All 17
trace and 6 flow faults remain and are genuine: 0x511EC9 (null sentinel
WorldRule index read by 0x511E50) and 0x76A8E0 (0x76A9C0 stored null for an
unknown RMC name; 0x76A8C0 dereferences it when the request is re-encoded).
The allocator seam (0x6F7810) was active in every faulting run.

5. SEAMS AND LIMITS (be precise about what is not proven)
---------------------------------------------------------
* Allocator, CString manager, logging, clock (0x6FB1F0), variable lookup
  (0x405370 -> zero-valued variable), address interning (0x783870 and
  ntohl/ntohs getters), named lookups (0x41EEC0 C-string key, metagame
  0x93E170/0x95D900) are declared seams. Definition registries are
  synthetic with unique counts (1000+3k) so each 0x411FA0 read identifies its
  registry; every thing slot resolves (type checks true); session players
  1 and 2 resolve; TeamCommand flags 0,1,2,8,0xB; def+0x20 = index % 4.
* Validity functions (0x5092F0 TeamCommand) are forced true to reach the
  writer; the cheat permission is set enabled. Their own rules (and every
  order's process function, i.e. the gameplay/lobby effect) are NOT ported
  here except move/actor/selected/set-player from phases 4/5.
* Random value placement covers the branches reachable with the synthetic
  environment; branches needing other game data were checked by reading the
  native decoders (see payload/complex.rs doc comments).
* No claim that the game is playable over the network; see phase8-net.

6. REPRODUCE
------------
  set PYTHONPATH=E:\Games\Axis and Alies\reconstruction\tools\python-packages
  python research/phase8-orders/static_calls.py
  python research/phase8-orders/native_order_traces.py      (~10 min)
  python research/phase8-orders/native_order_flow.py        (~10 min)
  python research/phase8-orders/gen_schemas.py --write
  python research/phase8-orders/gen_flow_table.py
  python research/phase8-orders/gen_inventory.py
  python research/phase8-net/native_transport.py
  python research/phase8-net/native_sync.py
  CARGO_TARGET_DIR=target/s-orders cargo test -p aa-orders --release --locked --offline -j 4
  Logs: validation-tests.log, validation-clippy.log.
