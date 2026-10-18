PHASE 4: ACTOR IDENTITIES AND COMPONENT LIFECYCLE
===============================================

Scope and integration
---------------------
aa-actors provides a native GTID registry and the proved component-selection,
dependency and lifecycle loops. It is useful runtime state machinery, not a
claim that an authored ActorDefinition is now a complete playable actor.
Complete GWorld Create/Deploy includes unported placement, ownership, random
orientation, nested actors, abilities, component constructors and publication.
Those are not approximated by zero values or guessed defaults.

Registry
--------
GTIDManager is [B364C0]. Its generation vector is manager+4 (65536 u16),
object table +20004 (65536 pointers), count+60004, cursor+60008, highest+6000C,
and occupancy vector pointer+60010 (2048 words). GThing ID is object+14.

4EFF60 Reset sets generations/table/bits to zero, count0, cursor1, highest0.
4EFDB0 Add checks count<32768, scans the cursor until a free slot, composes
generation<<16|slot, updates table/bit/count/highest, and advances with65535->1.
4EFB70 seeds a saved generation; 4F0190 AddID checks table/generation and writes
the supplied ID without the ordinary Add capacity gate.
4EFEC0 Remove clears the slot/bit, decrements count and increments u16 generation
with wrapping. Highest scans downward and can reach-1 after the last removal.
4EFCF0 validates generation/table then calls the object's IsActor virtual+B4.
4EFBE0 enumerates occupied slots ascending. 4EFB90 exchanges two live objects'
IDs and table entries; the null-first-pointer overload is not ported.

trace_registry.py executes these complete original x86 functions. Five sequences
contain33 operations with every nonzero generation, table identity, occupancy
word, cursor, highest and count compared to Rust. A separate original count-gate
fixture exercises32768. Tests also fill32768 Rust entries and validate checkpoint
failure behavior. IsActor, allocator-independent synthetic object memory,
diagnostic logging and load-error exception formatting are explicit seams.
No native game process is launched.

Components
----------
497FA0 registers31 component indices. The fixture executes this initializer and
captures only its4976B0 registration sink. Body0/Mover3/Mind4 are confirmed.
GActor table starts+60; actor raw state is+100/+104; definition mask is+174.

4D21A0 initial selection order:
15,0,2,1,3,5,6,7,8,9,10,11,12,13,14,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,4.
Existing slots are skipped. Except Mind4, the definition mask must contain the
bit. Construction is state lowbit21; the separate predicate uses lowbits25/26
or highbit4. Construction15/Body0/Properties6/Wall12/Structure13/Detection22 are
independent of those predicates. Element8/Economy9/MobileDeployment19 require
!construction OR special. Other ordinary slots require !construction AND
!special. City29 uses explicit4D2100(29) definition predicate. Mind4 needs only
!construction; actual4CF480 chooses its subclass from definition bits7/23/8 or
returns null. No generic Mind instance is invented.

4CFB00 visits current slots0..30; virtual+20 returns required slot or-1.
Missing dependency logs and returns false immediately. On success selection
calls4B2830 if Construction exists. Original network/owner publication after
selection is excluded from the port; the oracle disables it via owner-query0.
Runtime reassembly's param=true newly-created-list init is also excluded.

4D6F10 InitCreate performs substantial base/ownership/stance/nested creation
before its component loop. The oracle enters the exact4D71B7..4D71E2 loop with
the original register and record stack layout; it stops at loop success/failure
boundaries. This explicitly does NOT execute or replace that earlier work.
The loop calls virtual+4(record) ascending and stops on first false.

4D7230 InitExisting calls base4EBE50 and required-object4D1FD0, then loops from
4D7287: virtual+8 for present slots other than4, then slot4 last. After every
callback succeeded, virtual+14 runs ascending on the current table. Oracle
base/required-object callbacks are explicit success/failure seams, record=null
omits unported record-owner resolution, and execution stops at4D72F3 before
subsequent required-object notifications, state and ability work.

4CF300 PostCreate calls base4EBA40, virtual+C(record,flag) for current slots
ascending, then owner/city notifications. The fixture uses explicit owner0 and
flag1 to exclude those later notifications; the port exposes only component loop.

trace_components.py runs85 scenarios: each definition bit, state combinations,
present/null factories, constructor state changes, dependency failures, every
major lifecycle failure location, and table mutation during callbacks. Native
4D21A0 uses original linked-list append/free and4CFB00; factories/virtual methods,
successful allocator and construction leaf are seams. Full-function paths check
stack balance/callee-saved registers; scoped loops use documented stop addresses.
The Rust tests compare the complete ordered callback trace, result, table and
raw state. Assembly failure leaves created state in place, as native does.

Construction evidence retained for future ports
------------------------------------------------
4F9C10 GWorld CreateGThing dispatches category+1C; category3 allocates GActor
through4D6BB0 after checking the creation-record type. It calls virtual E4/E8
initialization, then EC PostCreate, publication4F9510 and queue50C220, or destroys
failed construction through4F5490. These outer operations are NOT implemented.
4EE460 base initialization either allocates a new GTID or seeds/registers the
record's saved GTID. Registry mutations are therefore already possible before
later component failure; automatic rollback is not added by this crate.
4EF2A0 GThing constructor copies explicit definition/position and records world
time; facing has axis-locked/random/normalization branches. 4D6BB0 actor ctor
zeros31 component pointers and state, but also initializes owner/required object
and other fields. No complete C++ object layout is represented by Components.
Body ctor561960 uses45D730/45D760 derived getters; Mover ctor576920 uses verified
definition fields. Their actual numerical construction belongs to aa-body and
aa-movement. Authored inspector strings are not substituted for those getters.

Orders boundary
---------------
4F8E90 calls4154D0 alias lookup; found alias+4 supplies GTID, otherwise raw
CString pointer bits are tested against generation/table/IsActor. The portable
orders crate correctly keeps raw UTF16 and a mandatory resolution callback.
Numeric-string parsing, fabricated aliases or map-placement IDs are not supplied.
Orders Delivered means the native generic dispatch was invoked; its ignored
return does not prove motion accepted or completed.

Reproduce / limits
------------------
python research/phase4-actors/trace_registry.py
python research/phase4-actors/trace_components.py
cargo test -p aa-actors
cargo clippy -p aa-actors --all-targets -- -D warnings

All executed original instruction ranges are hash-bound and equal in accepted
v14. Original and accepted whole images are pinned. Research emits only its own
evidence. No existing data, runtime, profile, save or launcher is modified.
This work does not prove networking, save decoding, kingdom spawn, full actor
creation/destruction, worker ordering, pathfinding, AI, abilities or combat.
