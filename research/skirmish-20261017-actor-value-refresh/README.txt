Immediate ActorInit combat-value refresh adapters

NEW load_denizen_value_refresh::refresh(Catalog, &mut Record, live PC)
executes native569E40 against the same held Denizen Box. The native head-first
Inventory order is the actual constructed/saved Vec order. Both +10/+14 reset
stores and each accumulated binary32 store happen on the real component.
Null/stale occupant GTIDs use their actual canonical target definition; a
resolved Actor without a Performer skips that slot instead. Existing occupant
Performer+60 and Body health/max come from actual owned payloads. Source
ActorData+174 and null-modifier45D730(+284) use the completed Bank's source PC;
ActorData+1E4 requires that Bank's verified completed source result. A valid
other held occupant, missing target, wrong payload/owner or unsupported finite
arithmetic is an explicit error with prior stores retained.

NEW load_organization_value_refresh::refresh has the same API and detaches only
the actual Organization Box temporarily. It reuses the existing proved
aa_combat::organization_combat_value coordinator. The additive private
saved_organization_combat::recompute_at_source verifies actual Bank ownership
and separates immutable source layout/mask precision from live arithmetic PC.
The old public diagnostic recompute entry and algorithm are unchanged.
Both adapters return the original component allocation on Result success/error.
No cloned Actor/Organization, constructor-only combat value or success-only
copyback is used. Panic/unwind recovery is not claimed.

Native Denizen fixture: native_denizen_value.py defaults to immutable replay.
141 paired cases / 282 separate original-v14 executions, all three x87 PCs,
102 observed executed instruction pins. Actual569E40,57DC40 and null-modifier
45D730 bodies execute; zero successful intercepted bodies. Explicit synthetic
head-first lists, stale/current generations, Body/Performer/source scalars and
rounding-sensitive finite values are fixture inputs. --write refuses overwrite.

The actual public test loads original tutorial02 through the verified source
capture03 installation and partial managed restoration. It calls the two
refresh boundaries on actual held allocations and saved relations. This does
not claim that omitted ActorInit/PostCreate/startup services ran, that original
Performers have been refreshed, that military combat works, or that a battle
is ready. The captured member Performer values are observed current values.

Remaining scope: native57E430 positive Performer evaluation requires actual
Properties instance/source lists, runtime ability/source identities, mutable
runtime settings/UVars/diagnostic ownership and CRT environment. Existing pure
combat_value::performer_value arithmetic alone is not a concrete world binding.
Settlement585650 currently has no typed RestoredComponent payload in this owner;
source census absence is not permission to replace a reached call with success.
Denizen/component InitExisting and later startup/tick services remain separate.
Full playable-skirmish acceptance belongs to root integration checks.

Final scoped validation (frozen 2026-10-17):
- Cargo current production build passed. Whole runtime lib-test binary built
  after current SAI dependency update; Denizen kernel replays all141 rows.
- Frozen current Cargo test executable final-immutable-unit-actual-replay.log
  passes2/2 with capture variables unset: all141 kernel rows plus actual map.
- Actual Denizen owner7742 usa_airfield has6 real null-occupant saved slots and
  positive completed-source contributions. Actual Organization owner4099
  regiment_usa_hq_infantry_unpacked has5 observed member slots and current/max0;
  that is not evidence that its member Performer values have run Init.
- Captured boundary native replay:6 paired /12 executions,98 executed pins,
  no successful seams. Both untouched images match all3 live PCs and Rust.
- Original public actual harness capture and immutable replay each pass1/1.
- Independent source/ownership plus fresh141/282 native replay accepted in
  independent-source-review.json; independent actual Rust replay is not claimed.
- Earlier failed build logs are preserved as superseded diagnostic attempts
  (foreign interface edits or the repaired test-only held-borrow lifetime).

All agent jobs for these two refresh adapters are reaped. Original assets,
executables and prior releases were not modified. The root integration build
and full map/battle/startup/tick acceptance remain separate.
