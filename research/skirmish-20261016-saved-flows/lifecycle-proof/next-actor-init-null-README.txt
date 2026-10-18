NEXT BLOCKER: saved GActor::Init(NULL), 2026-10-16

This is a read-only implementation plan, not a new port or success claim.
Native/source hashes and exact critical windows are in next-actor-init-null.json;
next-actor-init-null.asm.txt contains the instruction listing. No tests/builds
or native oracle executions were performed for this plan.

Where it belongs
4FAE80 LoadThing calls virtual+E8(NULL) after successful+D8 Load when current
Thing bit7 is clear, before PostCreate(NULL,1),4F9510 and the next record.
For Kingdoms,4E5DF0 first calls4D7230(NULL). Only successful Actor Init and
the following Kingdom diagnostic allow the later flow initialization.
The new aa-economy::kingdom_init coordinator already requires actor_init;
do not satisfy it by zeroing flows or returning true. The global+DC PostLoad
pass is separate. Retain the bit7 deferred path and false-Init destruction
boundary in the outer loader. See the existing lifecycle audit README.

Do not call the fresh helper with a default ActorParams
aa-actors::actor_creation::init_existing currently documents the created,
nonnull path. Its object_init_existing/place helpers write the supplied
position/facing block and use GACD4F3640 placement. Saved NULL instead goes
4EFA40 ->4EF6A0(NULL) ->4F4590, with the actor's actual flags/pose/definition.
At4D7265..7281, NULL skips the complete+40/+44 identity lookup and+138 store.
Keep the actual+138 value left by LoadThing/Load; do not clear/recompute it.
At4D7532..755F, state bit21 suppresses Existing FX; otherwise NULL makes the
creation-argument condition true without reading+24/+3C. Call42A590 on
definition+E4 with(actor,0). The current helper's equality test is only the
nonnull branch. Its comment calling this a PostLoad caller is imprecise:
saved Init is+E8 in per-record LoadThing, not the later+DC pass.

Native order to implement
1.4EBE50(NULL): clear Object+4C, update scaled height+38, compute tile-related
  geometry/diagnostic, then4EFA40(NULL). False stops. On success:4EB6E0,
  4EA150, cached tile+44/+48,4F1F70, virtual+8C and reached4EB490/4EBB80,
  then4EBC70. Bind actual spatial/visibility owners and retained loaded fields.
2.4D1FD0 required-object initialization; false stops. It can write+158 after
  a successful lookup. Retain this write if a later phase fails.
3. For NULL, leave+138 alone. Component+8 in current slot order0..30 with
  Mind slot4 skipped, then current Mind last. Any low-AL false stops. Re-read
  presence/owners after callbacks; do not initialize detached clones.
4. Component+14 in current ascending slots0..30. Then re-read+158; nonzero
  resolves via real GTID generations and calls required-object virtual+128.
  A stale nonzero reference is a required error, not a silent skipped call.
5.4D4CF0,4D1350,4D67F0, in order. Existing pure action/overlap helpers can
  be reused only with actual owned saved state and all reached dependencies.
6. If current Thing bit29 is clear, read the actual global property reference;
  if nonnull and Properties exists,5035E0 then conditional5817A0. Then4D4130.
7. Present Organization5788B0, Settlement585650, Performer57E430, Denizen
  569E40, in that order. Presence alone does not make these no-op callbacks.
8. ActorData virtual+24 settlement-space gate; if Object+50 GTID is not live,
  create zone2 via4D69A0. On success compute4BB000 radius, resolve actual
  zone2 from+148/+14C, and call515F40 with current position. Never invent it.
9. If B36528 is present and current Thing bit0 clear, call7A0C20. The managed
  load capability represents a PRESENT inactive SAI; it is not permission to
  substitute an absent observer or skip the call without proving its leaf.
10. ActorData TrackInfo45E700/45E710: for nonempty lists find maximum kind1
  length using native comparisons, evaluate settings/floor/ftol/idiv in order,
  even if no kind1 entry is found; allocate50AB10/store+144 only if one exists.
11. Re-read state bit21 and apply the NULL Existing-FX gate above. Return true
  only after all reached services finish. Preserve every prefix on false/error.

Reuse inventory and limits
* Public actor_creation::init_required_object and track_period already encode
  the native decisions/arithmetic. Bind actual required-object/GTID services
  and loaded settings. track_period's finite-domain limitations still apply.
* Public component_virtuals::{leaves,init_existing,vf14} and discovery_zone_radius
  have native fixtures. Dispatch each existing concrete class, including the
  three Mind classes. True/no-op is allowed only for the proved constant leaf.
* Components::init_existing has the correct Mind-last and ascending+14 loop,
  but LifecycleHooks bool/void needs a retained error adapter if reused. Never
  convert an unavailable service into a successful callback or lose its error.
* ability_flags/action_publication, spatial overlaps, combat organization and
  performer value, zones and scheduling contain reusable pure pieces. Their
  saved per-record owners/services must still be connected. The attachment-
  specific SavedActions context is not a reason to manufacture an Organization.
* Fresh object_init_existing/place are private and need a null-aware route.
  Preserve loaded+58 and reached4EB490/4EBB80 behavior; the current fresh helper
  explicitly rejects+58>0. Team base_init_null is Team-vtable-specific and is
  not a drop-in Actor/GObject implementation.
* Fresh actor_creation::word returns0 for missing words; Object fields also
  have map_or(0) reads. Saved adapters must require initialized fields at the
  reached read instead of treating missing state as native null. Global-property,
  scene, tile, SAI and source-reference facts likewise need actual owners.

Suggested next implementation/proof
Add an explicit saved/null coordinator or an Option parameter route without
changing fresh callers' proven behavior. Borrow the in-progress actor and
already restored registry directly: the current record may be temporarily
marked Loading in that registry. Do not wait until ActorStore staging after
all records, and do not recreate actors/queues/components to run Init.
Bind the earliest required service first and stop explicitly on the next.
Prove original/v14 at PC24/53/64: NULL preserves nonzero+138; nonnull behavior
stays unchanged; base/required/Mind false; slot mutations between callbacks;
required attach and downstream failure prefixes; state21/NULL FX combinations;
loaded+58; old-map scheduling and current queue ownership; track arithmetic.
Then run actual Kingdom source fields through the adapter. Only after full
Actor Init success should4E5DF0 initialize owned flows and continue recruitment,
Team/Kingdom registration and later publication. No full match claim follows
from a successful flow reset or from callback-only coordinator fixtures.
