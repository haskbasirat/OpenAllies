# Phase 8 AI - requests to the lead

Requests for changes outside the AI agent's files (crates/aa-mind/**,
crates/aa-runtime/src/{mind_world,mind_actions}.rs, research/phase8-ai/**).

## Resolved since round 1

1. **aa-trig unstored 445400 result** - done by the aa-trig owner
   (`aa_trig::signed_angle_extended`). Bound as
   `aa_mind::rout_score::native_angle`; the rout test checks it on all 522
   captured calls (same binary32 store, within two binary64 ulps of Unicorn's
   binary64 FPATAN).
2. **aa-sai crate** - declined by the lead; SAI stays in `aa_mind::sai`.
3. **SAI host design** - agreed (host-local SAI, own RNG, orders only).
5. **aa-runtime compile break** - resolved by the lead.

7. **SAI actor travel time** - done in round 3 on the movement helper's
   `aa_navigation::route_cache::route_cost` (`aa_mind::sai::travel`).
8. **Topology for map analysis** - 7AC0D0 is ported on its own region model
   (see open item 13 for the binding).
4. **GoalBase conversion** - done in aa-mind (`aa_mind::sai::bridge`, the
   lead's request: field-for-field GoalState, unlinked Roster lists only).
9. **SAIData DataNode reader** - done (aa-rules dependency added by the
   lead; `aa_mind::sai::rules_data`).

## Open

6. **Services to bind in aa-runtime when the owners have them** (planning
   list, no change requested now): zone queries 54EFC0/54F8B0, 4F46E0 open
   position, 52A690 reachable delta, 520440/520320 CAI moves (movement),
   GTID registry and SVar values (runtime), organization status 578140,
   engage range 578470, relation 4D4FF0 and halt 4D3FA0 (aa-actors /
   organization owner), 520D90 chase predicate, denizen component 569300/
   56AEC0 (denizen owner).

10. **Oracle harness hazards for every agent** (informational, please relay):
    * A harness that maps page 0 with FS base 0 (to serve fs:[0]) silently
      accepts null-pointer reads of offset 0..3. Mine now faults any page-0
      access whose instruction lacks the 0x64 prefix (research/phase8-ai/
      native_machine.py `_fs_prefixed`); other harnesses (e.g.
      research/phase8-combat/harness.py) may want the same check.
    * Unicorn 2.1.4 does not quiet a signalling NaN on `fld m32` (hardware
      does, with #IA masked). Avoid SNaN inputs in oracles or special-case
      them.
    * A missing allocator seam (6F7810) makes native code run the real heap
      and crash; if UcError is treated as a native fault, compare recorded
      events before accepting a fault (my tests do).

11. **SAI manager owner (whoever builds the SAI runtime object): route cache
    lifecycle.** `travel::RouteServices` needs the persistent solver
    `[[B36534]+234]`, the shared busy byte and the manager cache
    `[[B3652C]+40]`: `RouteCache::build(region count)` where `7A0D40` runs,
    `None` where `7AC9A0` runs (`7ACB00` topology load, `7ACE50` destructor),
    `RouteCache::reset` for `ResetPathPlanner` `5D8560`.

12. **Data manager owner: pending DataRef resolution.** The SAI profile and
    ego loaders queue DataRefs on `[B36530]+824` (vtables A17054 profile ego
    refs, A17038 filters, A1703C tech priorities, A17040 company templates,
    A17044 settlement templates, A17048 combined-arms elements, A1704C
    recruit requests; `aa_mind::sai::ego::DataRef`). Their resolution pass
    and the referenced managers are outside this agent's range; only the
    profile's `4AE340` is ported here.

13. **SAI map: binding `map_analysis::SaiMap` to the movement Topology.**
    7AC0D0 reads per region: index (+0), the first rectangle's tile terrain
    (`7B9320`, service `region_terrain`), the neighbour distance hash +5C
    (`7BC590`, built by the topology builder), the per-player entries
    (+1C/+20, SAI-player owned) and the static/mobile actor lists (+34/+28,
    filled by the simulation). The movement agent's `Topology` carries the
    neighbour distances in bucket order; the SAI manager owner should build
    `SaiMap` from it.

14. **Actor/combat owners: services of 7AC0D0.** `4D0180` combat value
    (body +68, organization +30, component +74 via `57DC40`, +88 +10),
    actor virtuals +104 (speed) and +108 (owner with +188 index), `4CEDA0`.
    `MapWorld` names them; `4D0180` is small and could be ported by the
    combat/actor owner with an oracle.

15. **Restoration agent: Organization saved Load `524B80`.** Done:
    `aa_mind::cai_member_added::member_added` (oracle-proven, 200 cases).
    `aa-mapworld/src/organization_restore.rs` `mind_member_added` can call it
    with a `MemberAddedWorld` (Mind words +24/+28, member Mind virtuals +3C
    and +40, `522890`, `[[B36550]+E4]`, the arrival list +2BC).

16. **aa-rules (definitions agent): exact Thing / Property registries for
    `RulesData`.** `4BDBA0(ids, 0x2B)` uses `ActorCatalog::find` plus
    `native_category(..).native_kind == 2` (the aa-metagame inference for the
    Thing virtual +38) and `463FA0` the authored `Property` root order. If
    aa-rules gains the native Property registry or a proven Thing +38
    predicate, `RulesData` should switch to them (handles are opaque words;
    only identity and hash order depend on them).

