Stored Actor registration4D62F0 / grid4CFEA0

NEW stored_actor_registration::register(catalog, &mut Context, key, livePC)
binds the existing aa_actors::actor_registration coordinator to the actual
published ActorStore allocation. Context loans the same ActorAccess, Mind
Arena, moved source Bank, SpatialWorld, current variables, clock, inventory,
presentation, queues and scheduler owners through stored_actor_state_effects;
the same ZoneMap, global PointShape, Session and mission-fields owners are
borrowed separately. No whole World, LoadShell, component or mutable owner is
cloned, detached, installed into a temporary registry or reconstructed.

The completed actual Bank validates the fixed native42-entry ActorState
manager (477FC0/474270/473490; mode2 source cannot append identities). The
count is captured once by the unchanged native-proven coordinator. Actual
components.state low/high are reread for each index. Every active entry calls
stored_actor_state_effects::enter on the SAME Context with previousState=None;
generation/class/source ownership is rechecked even on a child error. Earlier
state/property/FX/queue mutations are retained. Source arithmetic precision
comes from the actual Bank, independently of the live registration PC.

The grid half uses the SAME published SpatialWorld member/cache and actual
Bank definition token, validates canonical source geometry and native44/48,
then reuses the exact4CFEA0/554CC0 iterator. SessionE4 is reread at every tile;
mission_fields.started48 is read only ifE4==3. There is no saved-GWORLD or
missing-World fallback. Native tile/set/entry order is retained. The source
shape is immutable definition metadata; temporary scalar call parameters are
not a second placement owner.

The existing515620 entered prefix was factored once into EnteredFields; its
saved wrapper still calls the same City child with the same actual record,
identities and RNG. The stored wrapper currently refuses positive slot5/4/3,
City8 and reached Detection discovery children explicitly. Those refusal
points are required-owner boundaries, not native exceptions or successful
positive effects. A no-Detection flags4 entry and native irrelevant bits can
complete. No missing component/zone service is converted to success.

Validation:
- Own current runtime production check02 PASS59.48s, locked/offline/release/j4,
  target/s-skirmish-start-review. This also typechecks the parent's fresh
  stored_world_registration4F9510 and50C220 composition. Check01 stopped only
  at the parent's in-flight SpatialIndex flag-method addition; log retained.
- Exact production shared-kernel host:3/3PASS0.08s. It replays the existing
  fixed table/state-loop fixtures, all300 iterator cases, and all108 new
  entered-prefix cases. Only include_str fixture path relocation and exact
  production fragment hosting are used; no alternate gameplay algorithm.
- Fresh immutable registration replay:6 actual constructor tables +708 body
  cases =714 separate original/v14/all-PC executions. Declared constructor
  allocation/CString/insertion and state/grid child seams remain qualified;
  intercepted bodies are not claimed executed.
- Fresh immutable grid replay:300 rows/600 original-v14 machines; unchanged
  native fixture and932 instruction pins. This includes actual515620 plain
  paths, but not positive component/discovery bodies.
- New515620 proof:108 rows/216 original-v14/all-PC machines,34 executed pins
  and5 separately identified stopped entry pins. It proves first required
  owner order and the actual Actor+B8 presence gate. It stops BEFORE positive
  owner getter/discovery instructions; it never supplies successful children.

Portable binding guards deliberately refuse stale/foreign generation, class,
source, still-Staged placement, mismatched published source/cache and live PC.
These are ownership checks, not claims about native invalid-pointer/SEH or
allocator failure equivalence. No source/rules parser, manifests, lockfile,
original files, saves, profiles or release packages were changed.

The complete new ActorAccess/Bank adapter is compiled, not yet directly
executed by an actual fresh mission at this checkpoint. The parent's natural
selected-mission runner is the acceptance path; it is currently resolving an
earlier fresh Mind child. Existing native coordinator proofs and focused field
kernel tests do not establish full initialization, successful4F9510, full
zone effects, completed view publication or a playable battle.
