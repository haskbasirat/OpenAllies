Stored Body InitExisting field binding
=====================================

Native addresses: Body vtable A3B0E8+8=562290, +14=4B2830 RET.
562290 calls actual Construction565070 and Properties-context4CF110 before
its nonconstant561D40 child. If the initially captured health/max comparison
differs, it rereads both fields, writes maximum into health, THEN enters
561E50(old_health, token0, notify1, source0). State21 skips health reads.

The saved load adapter now delegates to ONE new body_init_fields coordinator.
The new stored_body_init_world adapter validates current typed Actor storage,
GTID/key generation, canonical source proof/factory/flags at actual movedBank
source PC, actual Body owner, and the genuine cached ActorBindings Arc for
fresh entities. It lends the SAME Body box from ActorStore; it never converts
live storage into a LoadShell, moves a component, clones it for production,
installs a second Bank, or keeps a separate pose/health authority.

APIs:
 init_existing(&Catalog, &mut Staged, ObjectKey, livePC)->Result<bool>
 after_init(&Catalog, &ActorStore, &Bank, ObjectKey)->Result<()>
An internal direct-store entry supports bounded component-constructor tests.
The actual fresh caller map_actor_init_existing delegates Body0 directly;
its owner is world_startup_gap. All other source/caller fields remain theirs.

Supported finite Body scope:
 - actual state21 true: complete, no health/max/Construction service reads;
 - current finite health==maximum and no selected Construction target:
   complete, no health stores or world services;
 - selected Construction target: actual current Properties context guard,
   explicit561D40 required before any new Body store;
 - unequal finite health/max: native health=max store retained, then explicit
   required561E50 with actual requested health. No successful health effects
   or fresh/full Actor Init are claimed at that boundary.
AfterInit is the actual constant RET, with portable typed-source preflight.
Invalid portable provenance/owner checks are not native invalid-pointer parity.

Proof and checks
----------------
The existing original/v14 native_body_prefix.py fixture was freshly replayed
immutably:216 rows/432 machines/all3PC,70 address-byte pins consisting of
68 executed instructions and two required entries561D40/561E50. It executes
real Construction and context getter bodies. Finite values only; no new NaN,
allocator/SEH/whole-native-loader claim. Native fixture SHA256:
bc1f52ae1970c405b75ee3aeb7b6aa227abd5ae123a02f48fa5259142ed7bd3c

Current production cargo check --lib --release --locked --offline -j4 passed
19.96s. Test-only LTO=false/codegen-units16; shipping defaults unchanged.
No full runtime link was started here, per parent's shared-cohort policy.

New Rust tests saved/pending root cohort:
1. Shared finite coordinator replays all216 native prefixes (test host only).
2. Ignored actual source Body/Construction/Properties constructors for
   usa_corps, gbr_corps, ger_bunker at all3livePC/source53, preserving exact
   Body and RuntimeObject addresses, source Bank ownership and RNG. These
   are explicit component constructor laboratories, NOT initialized Staged
   or naturally spawned map actors. Source-present slots reached by562290
   are constructed; no claim is made for unexecuted other component Init.
   Genuine constructor health is not altered for successful calls. Separate
   labelled damaged-field probes test retained max store/required561E50,
   then compare the same fields/allocation. Foreign Bank and forged source
   flags refuse before mutation.
Changed saved adapter's existing216-row +actualhealthy regression is queued
as well. No passing current Rust replay is claimed until parent reports it.

Original files/assets/vendor archives/profiles are untouched. No hardware
limit change, broad Body damage/death/capture/effect service, mission startup,
playable battle or full-rewrite completion claim is made by this checkpoint.
