# Independent combat-source completion dependency audit

The shortest supported path is a **CV dependency slice over one canonical source
owner**, followed by native means and ActorData cache evaluation. It must not be
labelled completion of the whole `4BD800` source lifecycle. Its prerequisite
passes also maintain construction, affiliation and placement collections used
elsewhere in the game.

Native `4BD800` calls `4BB480`, `4BB130`, `4BB730`, `4BBF90`, `4BB590`, then
per-Actor `45F7A0`, editor-name processing `4BD440` and replacement verification
`4BC7F0`. Means precede cached ActorData values. `4BB590` checks the exact
`BF800000` constructor marker and calls `4618B0`; zero is never a substitute for
an uncomputed value. `Experience.fake_cv_value` is a different field, `+4F4`.

## What the three prerequisite passes change

| Native path | Reached writes | Relation to means and cached CV |
| --- | --- | --- |
| `4BB480 -> 4612E0 -> 45FAE0` | Actor upgrade predecessor/direct lists `+4D0/+4D4`, `+4C8/+4CC`; inherited model `+4E0`; copied Object shape words `+50..+64` | Disjoint from the CV inputs. Settlement extent reads corner **ActorData `+3C`**, not copied shape `+50..+64`. |
| `4BB480 -> 45FBB0` | Actor affiliation `+488/+48C`; inverse owner links `+444/+448` | Disjoint from means/CV. These lists are read by the next construction-selection pass, so they cannot be skipped while pretending that pass ran. |
| `4BB130 -> 492C20 -> 492B90 -> 4C2290 -> 4C21E0`, then `45F7B0 -> 45F240` | New wrapper, new cloned layout and parameter buffer; selected owner's append list `+490/+494` | Original default `+2F0` layout, its data and parameter array are unchanged in the audited clone path. |
| `4BB730 -> virtual +30`, `471A10 -> 4B6BA0` | Placement-group weighted-choice collection at receiver `+18/+1C`, new weighted nodes | The virtual is a constant predicate in all 24 native Thing kinds. No observed Actor, Property or Ability CV field mutation occurs. |

Independent native write-set proof executes nine valid typed-object scenarios
on both pinned PEs at PC24/53/64: **54 cases**. It covers direct/indirect/duplicate
upgrades, existing model references, duplicate affiliation insertion, layout
cloning/selection append and repeated weighted choices. Protected CV input/cache
words remain unchanged. Successful allocation is an explicit seam; invalid
graphs, allocator reentry/failure and full parent-pass execution are not claimed.

`static-vtable30-audit.json` independently checks actual constructor MOV stores,
vtable slots and predicate bytes for all 24 native Thing kinds. Their `+30`
dispatch resolves only to constant true `4F0500` or false `4C9FA0`.

**Typed identity is a material precondition:** ActorData `+64` is an Object
shape word; AbilityDefinition `+64` is attack power on a different allocation.
An offset-only untyped map must not confuse or alias those object classes.

## Required canonical owner and leaves

1. Retain native registration provenance and one stable identity per source
   definition. The manager root list is reverse successful **native-kind-2**
   registration order. Resolve authored references once, then retain canonical
   identities without applying Translate again. A recursive child can be a
   derived kind-19 RandomActor even though kind 19 is absent from the manager
   root list; enforce the actual transitive closure rather than assuming it.
2. Build immutable, completed leaf inputs: component flags `+174`; completed
   local/shared Ability data (power, recharge, ordered animations); Property
   bias values and historical reference order; Body defense/health; Mover
   speed; Performer bias; Organization expanded slot groups and canonical
   parameters, including repeated/null slots; Denizen's signed, ordered
   weighted inventory. Preserve shared identities and mutations from source
   loading rather than rebuilding each actor's shared inputs independently.
3. Bind the ten typed effective numeric values, including WorldRule default
   handling or source-pinned absence, and a separate explicit source-phase FP/
   CRT context. Restore PC is not source PC. The normal proven controls are
   `007F/027F/037F`, with masked exceptions and round-to-nearest. The current
   diagnostic FP binder does not itself certify observed original startup;
   unmasked exception contexts must not be described as native-success proof.
4. Execute full manager means, using either the borrowed `4BBF90` coordinator
   over the same owner or the proved immutable kernel when only declared
   read-only leaves are reachable. Settlement bit 14 must be source-proven
   absent or backed by actual completed extent fields. Do not use a ten-unit
   subset as full manager means or invent an extent from raw wall diameter.
5. Keep a separate mutable cache state initialized from actual constructor
   fields: `+1E4 = BF800000`, `+30C = 0`, `+36C = 0`. Execute `4BB590/4618B0`
   in native root/recursive order over this shared state. Later reads see
   earlier cache writes. Repeated children count repeatedly; native cycle
   breaking depends on the early zero store, not a fabricated visited-set
   result. Required errors leave committed prefixes observable.
6. Bind the effective `CombatValueCSV` UVar before skipping its stream. An
   explicitly empty effective string permits the no-output path. Unknown or
   nonempty profile/application scope must remain explicit; a diagnostic fresh
   scope is not an observed original-profile receipt. The formula's lazy typed
   ranged settings also need the same effective source owner.
7. Publish completed cache values only after the full requested root walk
   succeeds. Retain source-owner, precision, numeric/CRT/CSV context identity
   with the cache; do not reuse values from a different context or source
   revision. Keep completion of the CV slice separate from completion of all
   source startup, world Init and live Performer values.

For actual acceptance, capture the current full source dependency input, run
native `4BBF90` and `4BB590/4618B0` with original `45E090`/CRT arithmetic, and
compare all reached cached words. The previously proved ten-definition subset
and pure formula fixtures remain useful references, not full-bank acceptance.

The author's current preliminary census reports 1,028/1,028 manager inputs,
212 Performer definitions and 155 mean-eligible definitions at PC24. That is
an input-availability observation; this audit does not certify the forthcoming
shared source Bank or its completion outputs before independent review.
