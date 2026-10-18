ORIGINAL MAP PREPARATION COORDINATOR -- 2026-10-16

Implemented
  crates/aa-mapworld/src/map_prepare.rs
  resolve_placeholders_and_random_things: native 4FDA70..4FDCDC.
  This coordinates original traversal and required replacement callbacks.
  It does not start a mission, create actors or publish a runnable map.

Native evidence
  native_map_prepare.py executes ORIGINAL AA.exe under guarded Unicorn.
  Every executed instruction is checked against accepted v14. The entire
  621-byte function is also identical in both binaries.
  Original: 5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26
  v14:      3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1
  Function: 5b7d5c648981fe0b5abdfb19e4aae6f2954ad7897aec7fbd8ec8c70372e374c8
  Native fixture native-map-prepare.json SHA256:
  32bd111d6515423b822ef6e8ec3371a1c9053927bc29b07d8dd0ee5031ff24bb

  32 cases at PC24, PC53 and PC64: 96 rows; 168 unique native instructions.
  90 successful traversals, six guarded native null-owner faults. The
  coordinator contains no floating-point arithmetic; precision variants
  check that no hidden FP dependency appears. The fixture records 225
  replacement requests, 177 random-selector calls, 354 RNG reads/log calls,
  and six missing-substitution diagnostics.

Ordering findings
  - Signed IDMGR+60004 live count is reread after each processed object.
  - 4EFBE0 receives one mutable cursor initially zero; never sort/snapshot
    objects instead of supplying the native slot walk.
  - Category is definition+18 -> +C, not runtime class at +1C.
  - Eligible placeholders call actor vf108 THREE times. The second answer
    supplies Kingdom+190; object+4 is reread; the third supplies Kingdom+234
    Nation. One cached owner or definition is observably different.
  - Missing second/third owner dereferences null natively. Rust returns
    NullOwner(2/3), preserving the observed prefix instead of crashing.
  - Missing Nation substitution logs actor+14 and continues. Replacement
    vf38 tests full EAX, not AL (the oracle returns 0x100 for true).
  - Random object category 22 uses definition+170; actor category 23 uses
    definition+5C0. Both log world RNG on B3655C then B36560, rereading it
    after callbacks, and pass the actual RNG to 4B6FF0. A fixture logger
    deliberately mutates RNG to expose stale-value caching.
  - Null random selections still reach 4FD7F0/4FD8C0; never silently skip.
  - Process the batch (4FC4A0), then destroy it (4F6EB0), even for an empty
    live table. The native normal return is true.

Explicit seams / limits
  Resolver allocation/construction/destruction and member constructors;
  GTID iterator; vf108 owner; Nation substitution 473370; replacement vf38;
  random choice 4B6FF0; profiler/sync/debug logs; replacement capture
  4FD7F0/4FD8C0; final drain 4FC4A0. Fixtures script responses and changes
  at these boundaries. The harness's scripted random-choice seed mutation
  is NOT an implementation/proof of the weighted selector's own algorithm.

  Rust replay supplies native read/return values and checks every call and
  argument in order, consuming the complete trace. It proves this coordinator,
  not the callees. Empty batch values appear only in the replay mock; no
  permissive default production host exists. The host owns real canonical
  objects, definitions, Kingdoms, RNG and replacement batch. queue_replacement
  must capture native parameters/property transfer at the call, and the drain
  must implement actual creation/removal/publication. Collecting IDs and
  returning success is not integration. No runtime/client binding was added.

  Initial vector capacity eight is not a gameplay limit: use growable host
  batches. No eight-entry cap was added. Native allocator failure and C++
  exception cleanup are not proven. On host error Rust stops and leaves
  prior effects committed; callers must not retry as a transaction.

  Page-zero guards allow only FS-prefixed SEH link access. The shared machine
  traps unseamed allocators/imports, verifies original/v14 instruction bytes
  and uses fresh trampolines. Null-owner rows stop at actual native faulting
  reads, not at an allocator or emulator failure.

Validation
  python research/skirmish-20261016-map-prepare/native_map_prepare.py
  produced the NEW fixture; script refuses to overwrite an existing fixture.
  CARGO_TARGET_DIR=target/s-metagame
  cargo test -p aa-mapworld --test native_map_prepare --release --locked
    --offline -j4 -- --nocapture
  PASS: one test, all 96 native rows, zero failures/ignored. See replay.log.
  No broad gameplay test, starting-actor creation or match was run.

Next
  See docs/SKIRMISH-MILESTONES-20261016.txt. Original Firefight contains six
  corps and twelve bunker placeholders needing real callbacks. Next bind/prove
  Nation lookup and native replacement capture/drain, sharing existing actor
  creation/world ownership. Do not duplicate kingdom_start.
