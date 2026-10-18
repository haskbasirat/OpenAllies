# Requests to lead (phase8 saved components: Organization slot 7, Transport slot 24)

Both loaders are ported and oracle-proven in aa-actors (my files only). They
are not wired; the restoration agent owns the dispatch. No Cargo change made.

## 1. Restoration agent: wire Transport (slot 24)

Files: `aa-actors/src/owned.rs`, `aa-mapworld/src/{saved_components,restore_components}.rs`.

* Add `Component::Transport(Box<aa_actors::transport_state::TransportState<D>>)`
  (slot mapping `ComponentSlot::Transport`, `map_specialized` passthrough).
* Constructor (native 58B9A0, factory slot 24):
  `TransportState::<String>::new(owner: ActorId, registered_resource_count: usize)`
  where the count is the registered resource registry count `[B3A660]+28`
  (the same value `EconomyState::new` uses).
* Load (native 58C810) at actor stage slot 24:
  `TransportState::load(&mut self, bytes: &[u8], defs: &mut impl organization_state::Definitions<String>) -> Result<usize, LoadError>`
  returns the bytes consumed (the rest of `bytes` belongs to later stages).
  `Definitions` = `layout_data(&[u16])` (463FA0, KKC_OrganizationLayoutData
  registry B3AB00), `actor_data(&[u16])` (4BDBA0, B39F58 class 0x2B),
  `layout_info(&D) -> LayoutInfo` (LayoutData `+8/+18/+1C/+20/+28/+2C/+34/+38/+44/+48/+50`;
  Transport only needs the entry count) and `name(&D)`.
  All 47 Transport payloads of the 80 shipped maps load (native and Rust);
  all have an empty cargo reference and no layout, so no lookup is called.
* Transport has no Updatable record (no saved-ID registration).
* Its component virtual `+20` is `7C7840` (not the shared "none" 580F70);
  I did not port it (actor_finish.rs is yours).

## 2. Restoration agent: wire Organization (slot 7)

`SavedComponent::decode_prefix(7, ..)` cannot work: the payload embeds the
complete member records (DYNAMIC_THINGS record format, no length prefix),
loaded by `4FC200 -> 4FAE80` in the middle of the Load, and later fields
depend on them. Call instead, on the state built by owned.rs (579BD0):

```rust
aa_actors::organization_state::load(
    org: &mut OrganizationState<String>,
    bytes: &[u8],                       // from the slot-7 stage start
    owner: &OwnerFacts<String>,         // definition, gtid (+14), has_mind (+70),
                                        // object_10 ((+14C > 4) && [+148]+10),
                                        // default_layout (ActorData+2F0 layout object)
    facts: WorldFacts,                  // world_version = GWorld+2C (GWORLD version,
                                        // after ActorPrefix::upgraded_world_version),
                                        // time_e4 = GWorld+E4 bits
    world: &mut impl OrganizationWorld<String>,
    pc: Precision,                      // simulation PC53
) -> Result<usize, LoadError>           // bytes consumed
```

`OrganizationWorld` (in organization_state.rs) lists every native call in
order with its address. Methods marked "Fact" are memory reads; all others
must be bound to the real ported behaviour, or return an explicit error
(never Ok) while unported. Bindings I found (grep of crates/):

| method | native | existing port |
|---|---|---|
| updatable_table_word0 / updatable_slot_occupied / register_updatable | 50BCD0 on GWorld+284 | aa-runtime updatables.rs / restore_components UpdatableRegistration (register at the native point, slot 7) |
| load_member_record | 4FAE80(ctx,&out,owner,0) | aa-mapworld restore.rs record loader: must become reentrant (nested record, owner = org owner, flag 0); return (None, n) where the original returns null |
| actor_modifier / modifier_apply | 4CF110 / 501C90 | aa_body::properties: Properties+1C ModifierState; 501C90 == `adjust_with_floor(0x20, value, settings ModifierAttributeClampAdd +1A4, pc)` (verified from the instructions). Note 4CF110 is non-null whenever the owner has Properties, so real regiments DO call 501C90 |
| morale_contribution | ActorData+2C0 | aa-rules definition field |
| has_property | 45EEF0 | aa-rules (native_denizen.rs references it) |
| combat_value | 5788B0 | aa_combat::organization::combat_value |
| set_owner_state | 4D64F0 | aa_actors::state::set |
| set_leader | 579A00 | aa_movement::formation::set_leader; must also store Organization words[0x40] |
| follow_leader | 57C800 | aa_actors::element_position / aa-combat organization |
| owner_4d1350 | 4D1350 | aa_actors::ability_flags |
| state_group4 | 4D49B0(owner,4) | aa_actors::state::group + the StateData +20/+24 words |
| formation_refresh | 57BEF0 | aa_movement::formation |
| find_recruit_single / recruit singles | 4E2A60 / Kingdom+214/+218 | aa_economy::kingdom_registry |
| definition_list_2d8 | 45EC60 (ActorData+2D8) | referenced in aa-combat organization.rs |
| apply_property / remove_property | 4CEF10 / 4CEF70 | aa-body property_api.rs |
| owner_object_notify | 5147C0 | aa-combat organization (callback) |
| owner_4d4cf0 / owner_4cf0f0 | 4D4CF0 / 4CF0F0 | referenced (aa-combat, aa-economy supply) |
| member_flag_4d1080 | 4D1080 | referenced (aa-combat morale, aa-mind) |
| owner_object_164 / object_refresh | 4D1900 / 4B2830 | referenced (aa-mind, aa-actors components) |
| damage_type | 48D500 on B3AAF8 (KKC_DamageType) | aa-rules native_load |
| **no Rust reference found** | 5705A0 (Element set organization), 561760 (Body), 4CED10/4CEBB0 (experience source), 524B80 (Mind member added), 5784C0, 4CEF80, 4E1200 (recruit pairing) | need ports (or explicit unported errors) |

Handles: `words[0x40]/[0x44]` hold `ActorId::raw()`; `words[0x70]/[0x74]` the
formation handles your `LayoutInfo.formations` returns; `words[0x98]` the
damage-type handle; `words_34` recruit item handles. `created_at` = +B4,
`position_xy` = +BC/+C0 (`organization_state::native_word` maps offsets).

## 3. Blocker chain for real maps

All 1082 shipped Organization payloads (versions 8/9/10) start a nested member
record right after the layout (917 have a member in the first slot). Their
members are Element actors whose own records include Element Mind (slot 4,
unported, aa-mind). So regiments load only after (a) the reentrant 4FAE80
record restore above and (b) the Mind saved loader. Herds/flocks have the
same structure (e.g. flock_snowowlsingle in tutorial01).

## 4. Small requests

* `ground_states::ResourceVector::new` is private; transport_state.rs builds
  the 4CD440 vector itself (same rule). Making it `pub` would let me reuse it.
* `LayoutDefinition.definition` cannot be null; native 4C30C3 stores null to
  Layout+8 before the "Unknown IDS" throw. Only the partial state after that
  error differs (documented); no action needed unless you want Option<D>.

---

# Round 2 requests

## R2-1. Restoration agent: switch Denizen to the slot list (clears 168 records / 40 maps)

`denizen_state.rs` is mine now; I kept the old `Definition { inventory_is_empty }`
and `DenizenState::new` so catalog.rs and aa-mapworld tests keep compiling.
Please switch over, then I (or you) delete the legacy struct:

* owned.rs: `Definitions.denizen: Option<Vec<aa_actors::denizen_state::InventorySlotDefinition>>`
  and construct with `DenizenState::with_inventory(owner: ActorId, slots: &[InventorySlotDefinition]) -> DenizenState`.
* catalog.rs (mask bit 10): fill it from
  `ActorCatalog::native_denizen_inventory(id, pc)?.slots`, field by field
  (`target`, `worker`, `militia_or_air_defensive`), same head-first order.
  For all 168 shipped records this list equals the original constructor's.
* Load (DenizenPayload::apply, version >= 3), after `byte_21`, either
  `state.apply_occupants(&occupants.values, health, pc) -> Result<(), LoadError>`
  (your decode-first path) or the exact streaming
  `state.load_occupants(bytes_at_count_word, health, pc) -> Result<usize, LoadError>`.
  `health: FnMut(&str) -> Result<u32, LoadError>` returns the target's
  ActorData+284 bits (45D730, null modifier) =
  `native_ground_definition(target).attributes.body.health.bits`; return an
  explicit error for a target without a Body block (all shipped targets have one).
* The "No constructed Inventory slots" comment in DenizenPayload::apply no
  longer holds once slots exist: the first `min(count, slots)` pairs are applied.

## R2-2. Restoration agent: Transport requirement

actor_finish.rs `required_by_ground_slot`: `ComponentSlot::Transport =>
Some(aa_actors::transport_state::REQUIRED_COMPONENT)` (Mover; vtable A40100
+20 = 7C7840 `mov eax,3`, proven with the original 4CFB00).

## R2-3. Restoration agent: bind my callees in OrganizationWorld

* `body_set_organization(member)` -> `aa_actors::organization_callees::body_set_organization(member, Some(owner), w)`
  (needs Updatable `50B220`/`50B1C0` on the member's Body: aa-runtime updatables).
* `owner_experience_token()` -> `organization_callees::experience_source(owner, facts)` (4CED10).
* `set_experience_source(member, token)` -> `organization_callees::rank_changed(member, 0, token, w)` (4CEBB0(member; 0, token)).

## R2-4. Combat agent (Organization / Performer / property wrappers)

* **5705A0** Element attach, the mirror of your `detach_membership` (570670):
  `Element+18 = slot; Element+10 = organization owner; Element+1C = 1;
  4D1350(member); 4CF0F0(member); if [B36528] != 0: 7A0C20([B36528]; member)`
  (no dying check, unlike detach). Called by 57CBA0 as
  `OrganizationWorld::element_set_organization(member, slot)`.
* **5784C0** owner property propagation to a new member, the mirror of the
  578540 loop in your `remove_member`: for each owner+78 entry e (count reread):
  if member Properties present AND definition(e+C)+0x42 byte != 0 AND NOT
  (member has Experience AND e+8 == 7) AND e+10 == 0:
  `4CEF10(member; e+C, e+8, owner)`. Note the extra +42 test 578540 lacks.
  OrganizationWorld::member_added_5784c0.
* **4CEF80** (aa-body property_api, sibling of your 4CEF70 wrapper):
  `actor+78 ? 581280(property, source) : 0`; 581280 = 5032D0(component+10
  list, property, source, exclude 0): first record r with r+C == property and
  (source == 0 or GTID r+14 resolves to source). OrganizationWorld::has_applied_property.
* **57DCF0** Performer ability rank gating (callee of 57E010 / my
  `RankWorld::performer_rank_changed`): per ability, 4BE4D0/4BE4F0(rank+4)
  toggles the +18 enabled byte.

## R2-5. AI agent: 524B80 (CAI.cpp, Mind range 515A00..539C40)

Organization Mind "member added", called by 57CBA0 on the owner Mind
(`OrganizationWorld::mind_member_added`). Not ported by anyone.

## R2-6. Economy agent: 4E1200 recruit single pairing

`item+C = member+14 (GTID); if member Element (+80): Element+14 = item`.
Your `RecruitSingle.actor` is item+C. OrganizationWorld::pair_recruit_item.

---

# Round 3 requests

## R3-1. Restoration agent: replace the two actor_finish.rs guards

In `finish_unlinked_actor` the guards "Actor name collision/action-table
validation is unavailable" (nonempty `+DC`) and "Actor numbered-name
assignment requires current owner state" are both the original
GActor::Load tail 4D971B..4D99C2. The "Actor158 saved identity validation"
guard is its first step too. Replace all three, at the same point (after
`+174` and the 4D67F0 refresh), with:

```rust
let mut number = actor.words.get(&0x170).copied().unwrap_or(0);
aa_actors::actor_names::finish(
    aa_actors::actor_names::SavedName {
        required_158: actor.words.get(&0x158).copied().unwrap_or(0),
        key: &mut actor.string_dc_utf16,
        text: &mut actor.string_e0_utf16,
        number: &mut number,
    },
    actor_version,             // saved actor version (signed compare with 0xE)
    &mut name_world,
)?;                            // Err(LoadError::Warning) = native 858A00 throw
actor.words.insert(0x170, number);
```

`NameWorld` bindings:
* `gtid()` actor +14; `definition_name()` ActorData IDS;
  `display_name()` ActorData+10 = GetLocalized("name") (stored localized
  text; a `#key` resolved through the string table, 5B5620);
  `numbered()` ActorData+1B0 (your `actor_numbered`);
* `gtid_live(id)` your restored GTID table (generation + live object);
* `localize(key)` the 4B9F40 string table ([B36544]; None = not found);
* `strings_from_stream()` = 59E520(ctx), i.e. your localization mode != 0;
* `map_string_exists(key)` = 4BAA20 lookup in [B36544] without creating;
* `warn(message)` -> RestoreReport event (5A1040 map warning);
* `owner_kingdom()` owner virtual +108; `owner_word_e8()` actor +E8;
* `assign_number(owner_e8)` -> one `aa_actors::actor_names::NumberedNameCounters<D>`
  per Kingdom (`assign(&actor_definition)`); not saved, built in load order.
  4D6F10 (owner change) also calls 4E4980 when the new owner is set and
  ActorData+1B0: use the same counters there.

## R3-2. Definitions agent (FYI)

`display_name()` needs ActorData+10 exactly as 47D960 stores it
(GetLocalized("name"): stored text, `#key` resolved via 4B9F40 with an empty
default). If no API exposes it yet, please add one.

---

# Round 4 requests

## R4-1. Restoration agent: run the PostLoad pass after the last record

Native place: GWorld::Load 4FC600 at 4FD3B3, right after DYNAMIC_THINGS
(every Team, Kingdom, required object and dynamic record restored and
registered) and BEFORE 4F1E10 / InitForLoadedMap / the class-changed
replacement tail. Call once:

```rust
aa_actors::post_load::post_load_pass(&mut post_load_world)?;
// Err(NullDereference) = native access violation; Err(Undefined/Unsupported)
// = stop (effects of earlier objects stay committed, as natively).
```

`PostLoadWorld` bindings (every method is one native field or helper):
* `highest_slot` = the GTID table's `+6000C` as your registry maintains it;
  `object_in_slot(slot)` = the record registered in that slot (its own
  GTID and class: Thing/Team -> `Inert`, Object/Area/Camera/Doodad (2, 6,
  7, 9) and A28D18 -> `Object`, Actor (3) -> `Actor`, Kingdom (4) ->
  `Kingdom`, class 8 -> `Group`); `live(id)` = generation + live record;
  `object_class(id)`; `object_flags(id)` = restored `+8` word.
* `required_object` = restored `+50` (Object version >= 2 word);
  `definition_name` = ThingData `+10` (= aa-rules
  `ActorCatalog::native_display_name`, R3-2); `log_required_not_loaded` ->
  RestoreReport log; `required_object_lost` = virtual `+12C` (GObject
  4ECC50, GActor/Kingdom 4CFF70): NOT ported by anyone; report an
  unsupported error if a tutorial record reaches it.
* `has_component`, `actor_list` / `remove_actor_list_node` (restored
  `+118/+120/+128` GTID lists), `actor_kingdom` = actor `+E8`,
  `element_organization` = `ElementState.organization`, `word_16c` =
  restored actor `+16C`.
* Queue helpers on the component's Updatable (the `aa_world::queues` owner
  must identify that component's Updatable, as leaf_init.rs does):
  `has_scheduled` = the native 50B220 walk (R4-2 (a); NOT `Queues::has`);
  `has_any_scheduled` = `Queues::has_owner(owner)` (Component+C);
  `add_periodic` = `Queues::add_periodic(queue, owner, data, rng)`;
  `add_after` = `Queues::add_after(3, owner, data, delay_bits)`;
  `cancel_scheduled` = `Queues::cancel_owner(owner)`;
  `remove_scheduled` needs R4-2; `random` = the SAME world `NativeRng`
  instance's `next_u32()` (4F02B0 + 85D840).
* `unit_supply_enabled` = SVar UnitSupplyEnabled (aa-runtime settings);
  `body_shortfall_rate_bits` = ThingData `+2A8`; `world_version` = GWORLD
  `+2C`; `wall_flag_42c` = ThingData `+42C`.
* `denizen_commands` = Denizen `+40` (empty while Denizen command objects
  (51B580) stay refused by your decoder); `command_members` = command `+C`;
  `set_mind_command` = 52A020 on the member Mind `+30` (AI agent's state).
* Defender: `DefenderState.entries` (index 0 = head), `abilities`,
  `word_1c`; `ability_target` = Ability `+14` in the world arena.
* Hq: `hq_release_items` (+20/+24), `hq_index` (+10): needs R4-4.
  Kingdom: `kingdom_hq_slots` = `[+234]+40` (None for a null `+234`),
  `kingdom_hq_taken` / `push_kingdom_hq_taken` = `+36C` (prepend),
  `kingdom_group_*` = `+34C/+350` groups, `recruit_item_*` = `+214/+218`
  items (`+8` byte, `+10` actor, `[+4]+10` name, virtual `+8`),
  `increment_kingdom_224`, `organization_detach_recruit_item` = 578DA0
  (R4-3), `map_warning` -> RestoreReport warning.

Then set `LoadCompletion::AwaitingPostLoad` records to their post-load
state. Tutorials reach only ported PostLoads (tutorial-post-load-inventory.json).

## R4-2. Tick agent (aa-world queues): 50B220 walk and 50B1C0 remove-matching

(a) 50B220(queue, data) is NOT an owner-list test. Natively it starts at the
Updatable's most recent entry (Updatable+4) and follows the entry's queue
BUCKET successor (+8), matching byte +18 == queue and +14 == data on any
owner's entry; the Updatable's own entries in other buckets are not seen
(proven: native-post-load.json `updatable_has_50b220`, 8 native runs;
`aa_actors::post_load::native_has_scheduled`). `Queues::has(owner, q, d)`
checks the owner's entries instead. Please add the native walk, e.g.
`Queues::has_from_owner_head(owner, queue, data)`: start at the owner's
first (most recent) event and follow `Node::next` in its bucket. It matters
whenever the Updatable already has entries (PostLoad after 50BCD0 restored
saved entries); with no entry both answer false (so InitCreate users such as
leaf_init.rs 562338 are unaffected while the list is empty).

(b) 50B1C0 remove-matching:

`Queues` has `has`, `has_owner`, `add_periodic`, `add_after`, `cancel_owner`
but no public way to remove the owner's entries matching one
`(queue, data)` pair. Native 50B1C0(queue, data) walks the Updatable list
(+4, next +10) and removes EVERY entry with byte +18 == queue and +14 ==
data (50ADE0 each), returning the count. Please add e.g.
`Queues::remove_matching(owner, queue: u8, data: u32) -> Result<u32>`
(with your cursor handling as in `cancel`). Body PostLoad 561DF0 needs it
(`PostLoadWorld::remove_scheduled`); Organization 561760 (round 2) too.

## R4-3. Economy / organization owners: recruit-item seams of 4E4220

Kingdom PostLoad 4E4220 calls, for a recruit item (+214) whose `+10` actor
is dead: the item virtual `+8(1)` and 4E1C70(-1), which calls
`578DA0(item, -1)` on a live actor's Organization component (+7C) and
`Kingdom(+E8)+224 += 1`. 578DA0 and the item virtual are not ported (the
restoration README already lists recruit singles +214 as not done).
`PostLoadWorld::{recruit_item_virtual_8, organization_detach_recruit_item}`
are the hooks; nothing in the tutorials reached them in the oracle sense
(it depends on the saved +214 contents).

## R4-4. Hq component (slot 30) saved Load missing

aa-mapworld `SavedComponent::decode_prefix` has no slot 30; aa-actors
`owned::Component` has no Hq state. tutorial02 has 2 Hq records
(tutorial-post-load-inventory.json). Native: ctor 574490 (vtable A3CA4C,
Updatable A3CA24, 0x2C bytes), Save 573F40, Load 574040, PostLoad 5736B0
(ported here: `hq_post_load`). I can port ctor + Load next if you assign it.

## R4-5. Unported PostLoads (not reached by the tutorials)

* Settlement 586F60 -> 586BC0(0): recreates the settlement's GThings
  through 4F9C10 (world creation; restoration/creation owner). Returns
  `Error::Unsupported` today.
* GGroup (saved class 8, vtable A29490) 4DDFA0: 4D3560 then 4DDCD0 and
  515F40 (Mind range) on `+1A4`. Returns `Error::Unsupported`.
* Virtual +12C of 4EAA30 (4ECC50 / 4CFF70): see R4-1.
* After the pass: 4F1E10 (only map settings +A8 in {1, 4} and 59E520) and
  VWorldState::InitForLoadedMap 4F5290/4F1C20 are unported (inventory in
  README round 4).

## R4-6. Combat agent: Transport payload 58BA90 hook for summon::apply

`aa_actors::transport_state::TransportState::load_payload(cargo, name,
layout, start, drop, registered_resource_count, &mut impl PayloadWorld)
-> Result<bool>` (always true natively). `PayloadWorld`:
`cargo_upkeep(cargo, layout)` = 56F580 = `economy_values::layout_values`
(Upkeep); `slot_groups(layout_data)` = LayoutData `+28` group parameters;
`refresh_owner()` = 4CF0F0 on the Transport's owner. From summon (4CCA70):
`layout` = the 4C2290 copy of the summoned object's ActorData `+2F0`
default layout when the cargo is an organization (else None); `name` =
empty; `start`/`drop` = the effect's points. Oracle:
native-transport-payload.json (9 cases).

## R4-7. Restoration agent: R3-2 is available

`NameWorld::display_name()` (R3-1) =
`catalog.native_display_name(&definition_ids, &mut |key| lookup_4b9f40(key))`
(aa_rules::native::display_name; `Ok(None)` from the lookup = missing key
-> empty text, as 4B9F40's B43B48 default).

---

# Round 5 requests

## R5-1. Restoration agent: HQ slot 30 decode + the extended PostLoad world

* Saved slot 30: call `aa_actors::hq_state::HqState::new(owner,
  ThingData+530)` at construction and `state.load(bytes_at_component_start,
  &mut hq_world)` (returns the bytes consumed; includes the 50BCD0 Updatable
  record) from your `SavedComponent::decode_prefix` dispatch. `HqWorld`:
  the GWorld+284 Updatable table (as for Organization), `owner_kingdom` =
  owner +E8, `nation_badge_count` = `[Kingdom+234]+40`, `nation_name` =
  `[Kingdom+234]+10`, `owner_name` = owner virtual +24 (4D99D0: the
  actor_names display / numbered name), `log` -> RestoreReport.
  `owned::Component` has no Hq variant yet (owned.rs is not mine): please
  add `Hq(Box<HqState>)` and bind `PostLoadWorld::{hq_release_items,
  hq_index, set_hq_index}` to `pending.clear()` / `badge_index`.
* `PostLoadWorld` gained (round 5): associated `Definition` / `Layout`;
  virtual +12C fields (`clear_required_object`, `definition_flags_1c`,
  `set_blocking` = actor_creation::set_blocking for actors / 4EE840 for
  objects, `set_object_flags`, `world_kingdoms` = GWorld +154, kingdom +190
  / +18C, object +58 / +5C memory records, UI agent hooks, `refresh_memory_
  record` 4EC4A0, `settlement_space` = ThingData +34, tile size, [B39F58]+74,
  positions, zone count +14C, `move_zone` = aa_zones::zone::move_to_position);
  recruit items now `recruit_item_organization` (+10) and
  `set_recruit_item_available` (4E0EC0); Settlement (`settlement_count`,
  `settlement_entry` ThingData +47C, `settlement_definition` 45D7C0,
  `object_z`, `create_gthing` = actor_creation/world_creation 4F9C10,
  `set_settlement_thing`); Group (`group_target` +1A4, `set_group_target`,
  `group_radius_bits` +1B4, `clamp_position` = aa_zones::zone::clamp_position).
  `recruit_item_virtual_8` and `organization_detach_recruit_item` are gone.
* After `post_load_pass`, call `aa_actors::world_load_tail::after_post_load`
  (LoadTailWorld: `world_start` = aa_economy::world_start::world_start; the
  session reads [B36550]+E4 / [B36558]+8; GWorld +148 names; +22C/+230;
  the 404F60 log), then the conversion agent's replacement tail.

## R5-2. Lead: aa-zones from aa-actors (optional)

aa-actors does not depend on aa-zones, so my ports (and component_virtuals.rs)
reach CreateZone / Move / clamp through world methods documented with the
aa_zones function they bind to. A direct call needs `aa-zones` in
crates/aa-actors/Cargo.toml and a Cargo.lock update (yours to decide).

## R5-3. Lead / zones agent: three addresses in the zone-creator list

568040, 563E10 and 58CF50 (research/phase8-zones README "zone creators") do
not create zones: 568040 is a pure predicate over 8 lists of 8 bytes (entries
0..4 empty) used by the Settlement update 587290; 563E10 is the Character
Updatable's deferred update (+18); 58CF50 is the Wall Updatable's periodic
update (+14). The zone 5 / 8 / 3 creators are Denizen +14 568070, City +14
563FE0 and Wall +14 58D3B0, already ported (with 566DB0, 585940, 58B0E0,
579DD0) by the actor-creation agent in component_virtuals.rs. Tell me if you
want the three updates ported anyway.

## R5-4. Actor-creation agent: one 4BB000

`4BB000` exists three times (actor_creation::zone_radius and
component_virtuals::settlement_zone_radius, both private, and my
`post_load::discovery_zone_radius`, pub, oracle-proven through 4CFF70 at
PC24/53/64). One shared pub function would remove the copies.

## R5-5. Tick agent / lead: aa-world queues edit (done, FYI)

Per the lead's instruction I added `Queues::has_from_owner_head` and
`Queues::remove_matching` to crates/aa-world/src/queues.rs (additive; all
existing aa-world tests pass) plus crates/aa-world/tests/native_queue_links.rs.
leaf_init.rs (562338) uses `Queues::has`; it only differs from 50B220 when
the Body Updatable already has entries (never at InitCreate).
