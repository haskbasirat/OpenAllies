Concrete held Properties InitExisting and AfterInit
=================================================

This is a bounded runtime component binding, not a completed Actor/Kingdom Init.
The actual stopped tutorial02 first Kingdom has GTID781, canonical definition
kingdom (source Special/Things/thing_kingdom.tgi), class4, component mask0x40 and
one actual owned Properties component. The same typed allocation is retained.

Native PropertiesComponent vtable A3E3DC has:
  +8  ->6A71D0: B0 01 C3 (mov al,1; ret)
  +14 ->4B2830: C3 (ret)
These are genuine constant leaves. They do not read component fields or world
services, perform floating-point arithmetic, mutate state, allocate or schedule.
AfterInit+14 is distinct from PostCreate+C581880, whose initial-property list
application is not bypassed or claimed complete by this module.

load_component_init_world::{init_existing,after_init} takes the actual mutable
held live_object_init::Record. It validates the registered Loading owner,
actual Actor constructor storage/native class, canonical source/template and
category, actual Bank source precision/owner, final source flags/component mask,
actual Properties slot payload and owner. No field/state copy is retained.
Missing source or payload/owner mismatch explicitly refuses before the leaf.
Other reached component leaves report their required native address/services;
there is no generic all-component true/no-op fallback.

Native proof
------------
native_properties_init.py executes the actual vtable-selected native bodies in
separate original/v14 images at Single24,Double53,Extended64. It makes6 fresh
machines and12 leaf calls. There are3 executed address-byte pins, no seams,
and no reads/writes of component allocation storage. The allocation bytes are
explicitly opaque poisoned storage, not a fabricated serialized game allocation.
This proves unconditional leaf behavior; it does not claim constructor/Load,
whole Actor Init, or native reconstruction of the typed saved allocation.

actual-properties.json separately captures the real owned typed Properties
payload from source-completed diagnostic tutorial02 restoration. Its actual
source PC is Double53, independent of the three supplied live PCs. The test calls
both production methods and verifies the same Box address, all Properties
values and the complete shell debug state remain unchanged. It tests explicit
missing-source/missing-slot/stale-owner refusals with prior state restored.

Reproduce without modifying any original game file:
  $env:AA_GAME_ROOT='E:\Games\Axis and Alies'
  $env:CARGO_TARGET_DIR='target/s-state-binding-review'
  cargo test -p aa-runtime --lib load_component_init_world::tests --release --locked --offline -j4 -- --include-ignored
  python research/skirmish-20261016-load-component-init/native_properties_init.py

The Python command is immutable replay by default. --write is author generation
before fixture freeze. AA_COMPONENT_INIT_CAPTURE is an author-only absolute
create-new output path; leave it unset for ordinary actual-data replay.

Remaining work
--------------
Nonconstant Body,Mover,Mind,Performer,Organization,Element and other component
Init/AfterInit services require their actual mutable world owners. Actor refresh,
required-object attach, global properties, zones, queues, TrackInfo, FX and later
Kingdom initialization are separate bindings. No full map readiness, playable
battle, multiplayer or game completion percentage is established here.

Source/native/actual hashes and final test/review status are in findings.json.
