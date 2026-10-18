Actual held Mover InitExisting576700
===================================

Scope and ownership
-------------------
load_mover_init_world validates the actual Loading registry owner, Actor
constructor storage, complete same shell/runtime proof, canonical source Bank
identity/category/flags/source PC, component mask and real typed Mover owner.
It mutates the original ActorShell components.state and Thing flags. No raw
Actor+100 mirror or duplicated persistent Mover state participates.

Native576700 reads Mover+56 first. Zero returns true without World+2C, pending
+C or state lookup. Nonzero reads actual current World+2C. For version<22 it
reads actual scheduled_head; null requires50AC00 queue0/data0 before state
activation. This service is explicitly unavailable and no queue is invented.
Otherwise the same native-proved saved_actor_state::set owner executes
4D64F0(24,true): an already active state returns unchanged, constructed flags
suppress the live enter effect and complete the state/flag stores, and a live
first activation retains its new low bit then stops at required4D4A90. The
actual low-bit partial write remains committed even after failure. Retrying
that failed initialization could skip an effect and is not a valid recovery.

The public version argument is a lazy getter of the real shared World+2C word,
not a copied source-setting or guessed default. It is reached only after
nonzero+56. A caller without the actual word can supply a precise required
error; it must never supply a guessed version. Native +14 is4B2830 return.

Native evidence
---------------
native_mover_init.py executes unchanged original/v14 separately at all3 PCs,
using descriptors/masks from the prior native477FC0 registration oracle. Its
288 explicit valid field cases use falling0/1/255, versions0/21/22/30, actual
pending presence, state24 presence and live/constructed flags. Results:
204 complete,48 required50AC00,36 required4D4A90;576 fresh image machines.
There are109 address-byte pins:107 executed and2 intercepted required entries.
4D64F0 executes fully until return or its exact first required effect entry.
No required service returns mocked success. No-falling probes poison the
World pointer; all rows verify actual lazy version reads and actor writes.
Mover vtable+8/+14 is pinned to576700/4B2830. This fixture does not execute
+14 separately; its common4B2830 body is executed by the Properties/typed
constant proofs in this same folder. Probe fields are explicit test inputs,
not a claimed complete native saved allocation.

Actual evidence and reproduction
--------------------------------
The actual allocation is GTID15284 horse (native category UNIT, class3), an
animal rather than a military combat unit. Source PC is Double53.
The ignored test selects the first actual nonfalling Mover in stopped Kursk,
keeps its real Box and complete shell, runs all3 live PCs and uses a panic
getter to prove that actual+56zero never requests a world version. The
source Bank's definition PC is retained separately. No military-unit,
whole-record Init or battle-readiness claim follows from this diagnostic.

  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  $env:CARGO_TARGET_DIR='target/s-state-binding-review'
  cargo test -p aa-runtime --lib load_mover_init_world::tests --release --locked --offline -j4 -- --include-ignored
  python research/skirmish-20261016-load-component-init/native_mover_init.py

Leave AA_MOVER_INIT_CAPTURE unset for immutable replay. The author-only
capture path is absolute/create-new; native --write is generation before
freeze. Validation/frozen hashes appear in mover-findings.json once complete.

Unbound reached services remain actual queue scheduling50AC00 and live state
enter4D4A90; this module does not implement the later simulation tick/Mover
movement, property/FX state effects or full Actor lifecycle.
