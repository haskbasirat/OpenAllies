BOUNDED SKIRMISH SOURCE/SELECTION SERVICES -- 2026-10-16

Implemented
crates/aa-runtime/src/skirmish_start.rs:
  WeightedTable.add / choose:4B6BA0 and4B6FF0 with the actual shared
  aa_simulation::NativeRng85D840, explicit PC24/53/64 and no unit-float store.
  substitution:473370, native head-first identity matching, distinguishing a
  missing entry (native diagnostic) from an existing null replacement.
  Definitions.bind: canonical Thing registry metadata, Nation placeholder
  source pairs, and RandomObject/RandomActor contribution tables.
  placeholder_for_actor: reads actual ActorStore owner+E8 and Kingdom+234,
  verifies the supplied definition token names the actual runtime definition.
  require_replacement_drain remains an explicit error. This is not a completed
  map_prepare World host, and it does not claim4FDA70 or4FDCE0 startup success.

Ownership / source identity
The existing Bank domain ActorDefinition denotes canonical real ThingData
identities here, including ObjectData. The historical domain name does NOT
mean every Thing is an Actor. Category/native_kind/runtime_class remain
explicit. Existing ordinary ActorDefinition tokens are reused; there is no
second mutable definition bank, actor world, or RNG.

Registered canonical definitions are matched to public resolved root views
using exact path+hash+span provenance. A canonical object is never run through
source.find again. This matters for chaparral_wall, which remains a registered
object even though a later translation redirects lookup by that string.
Authored reference strings use the original one-hop lookup behavior.

ActorCatalog's native_load.rs projects thing_owner in native registration
order (no sorting); binding preserves that order and each node's ordinal child
order.4B6BA0 prepends each accepted positive contribution. Totals round after
each addition, so neither sorting nor reversing the reduction is valid.
The native table constructor79E130 initializes head/total to zero; recovered
RandomObjectData49CC20/RandomActorData469980 constructors invoke it.

Temporal lookup restrictions
Nation48C050 placeholder keys and RandomObject/RandomActor targets are looked
up immediately during the source loader, not after all files. Their targets
must be registered at or before the caller (native file order, then root span).
An immediate reference touched by a Thing/Unit/Projectile/VFX translation is
explicitly unsupported until a historical alias registry is supplied, even if
that alias may be valid. All those root tags share the native Thing manager.
Nation replacement targets are genuinely deferred475280 and use final lookup.
Accepted installed data needs none of these unsupported immediate aliases.

Native proof
native_selection.py / native-selection.json:
  195 weighted cases at PC24/53/PC64, six consecutive actual RNG draws each.
  Both original and accepted v14 binaries execute independently.4B6BA0 table
  insertion and4B6FF0 selection execute unchanged; only allocation is seamed.
  Five473370 cases cover duplicates, hits, null targets and missing entries;
  only its missing diagnostic is seamed.4,126 native leaf calls total.
native_nation_pairs.py / native-nation-pairs.json:
  Six original parser48C050 placeholder-prefix/475280 fixup cases. Exact
  child iteration, prepend and CString/deferred construction run natively.
  Canonical Thing and placement-group lookup, localization/heap/log services
  are explicit seams. Execution stops at depot_name before unrelated Nation
  presentation/faction/depot loading. Missing keys/targets preserve the observed
  native prefix/throw; Rust returns an error without exposing a partial catalog.
native_random_source.py / native-random-source.json:
  Five original parser/contribution-window cases.49F06C..49F132 and
  4688D4..468995 receive the documented surrounding loader register/frame
  inputs; iterator/GetString/GetFixed/type checks and4B6BA0 are unhooked.
  Target lookup is a recorded seam. Multi-node/repeated contributions, positive,
  zero/negative weights, missing targets and wrong kinds are covered. The full
  preceding Actor/Object loaders are NOT executed by these window tests.
  Base ObjectData before ActorData ordering is also supported by the recovered
  enclosing loader call relationship; the test invokes these leaves in that order.

Original source-prefix/window fixtures pin every executed instruction against
v14. They execute the original image; they do not claim a second full v14
source-parser execution. The weighted/lookup fixtures execute both binaries.
Independent review replay files are listed separately in this directory,
including A5-filled native constructor proof and both executable hashes.

Validation
Six scoped Rust tests pass with AA_GAME_ROOT set (tests.log):
-all195 weighted rows and5 lookup rows;
-six Nation prefix cases andfive Random contribution-window cases;
-forward target rejection, shared-manager alias rejection, canonical-shadow
 preservation, nonfinite-input failure and null-token normalization;
-installed1,959 Things,5 Nations and56 random tables (all56 populated);
-correct five-nation corps substitution and actual owned Actor/Kingdom
 lookup, mismatched token and missing Nation errors.
The owned lookup case is an explicitly constructed fixture, not an actual
player assignment or completed mission. Original maps/releases are unchanged.

Limits
Finite supported x87 arithmetic only. Finite sums that overflow binary32 are
rejected before insertion; native would commit the node and Inf. That failure
case is outside parity. A nonempty invalid selection preserves its RNG draw
then errors instead of walking off a native list/crashing. No fallback choice
is invented. Source constructor/reload/error-cursor parity and allocation
failures are not claimed. Unbound source classes with contributions are refused.

Next concrete integration work
4FDA70 already has aa-mapworld::map_prepare's native-order coordinator. To bind
its replacement callbacks, port4FD7F0/4FD8C0 capture (position/direction/owner,
requested GTID, required-marker linkage, eligible self-source static properties),
4FC500 record copy/append and4FC4A0/4FBA50 processing.4FBA50 removes/destroys
eligible old placeholders before creating replacements, then transfers properties.
ActorStore.construct_new/initialize_object/construct_ground supply early owned
construction stages only. aa-mapworld::fresh_creation ports4F9C10/Object leaves
but still requires real FreshWorld placement/spatial/blocking/visibility/name/
scene/FX callbacks.4FA590 Actor creation also requires its actual PostCreate and
owner/component services. These cannot be replaced by collecting IDs and
returning success. Root retains the runtime ownership/publication work.
Full4FDCE0 additionally needs actual4F57F0, kingdom4E6DB0 property grants,
fog551FD0/551F90,4F61A0, map verification, city-victory rule updates and sun
selection in order. Campaign work remains deferred behind skirmish/multiplayer.

Backup
lib.rs.before is the pre-export module file. Only one new export was appended;
all production behavior is in the new module and no original game file changed.
