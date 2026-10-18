# Phase 8 integration backlog (lead-owned)

## Current user priority (2026-10-16, after Development08)

Faithful skirmish first, multiplayer second, campaign third. Defer new
campaign-specific work. First battle target: original
`DeveloperMaps/(3v3)Firefight.RMP`; it restores cleanly but still needs native
session/map preparation (starting actors replace placeholders), world startup,
simulation service binding, commands/combat/AI and victory. Preserve released08.
See `docs/SKIRMISH-MILESTONES-20261016.txt` for the concrete dependency audit.

Cross-area items the native world must provide before ported components can run
together. Source: each area's `research/phase8-*/requests-to-lead.md`. Owner is
the area that implements the item; "world" means the runtime integration work
that follows map restoration and publication.

## Decisions taken

- x87 precision (corrected 2026-10-14): the simulation runs at PC53. D3D9
  (CreateDevice without FPU_PRESERVE) switches only the main/render thread to
  PC24; TGC_NetNode::Update 0x765410 runs on its own TGC_Thread whose CRT start
  keeps the Windows initial CW 0x027F, and no game code changes it there
  (research/phase8-tick/fpu-precision.md). UI/render-thread code uses PC24.
  The PE status flag is set on every tick (sticky, no FNINIT in AA.exe).
  Precision stays an explicit input everywhere. A dynamic confirmation
  (debugger/proxy DLL, fpu-precision.md section 6) has not been run.
- Oracle caveat: Unicorn 2.1.4 computes FPATAN/FSIN/FCOS/FYL2X via binary64
  and never sets PE; 80-bit results of those instructions are not ground
  truth (research/phase8-tick/emulator-limits.json).
- Single stores: Commander XP (Kingdom+0x230 / Commander+0xC) lives in
  `aa_economy::tick::CommanderExperience`; kingdom statistics (+0x32C) live in
  aa-economy; aa-body/aa-combat write through `add_statistic`.
- The strategic AI (KKC_SAI::Think 7A0080) is host-local: own RNG, may run on a
  worker thread, outputs only orders through the normal order path. Unit and
  regiment Mind states stay deterministic on the world RNG.
- Path queries on worker threads only from an immutable snapshot taken at the
  native point, epochs assigned and results committed in native order
  (research/phase8-movement/README.txt).
- Ownership splits: MGAI -> metagame; CAI move orders 520320/520440/5276B0,
  member return 52E9D0/52E900, region topology 7ACB00, water flood fill 539C40
  -> movement; other Mind-state Thinks in 515A00..539C40 -> AI; capture,
  ownership change, cooldowns -> combat; supply -> economy; decree decoding and
  NetworkSeams -> orders/net.

## World integration (after restoration publishes actors)

| Item | From | Notes |
|---|---|---|
| Client drives frames via `aa_runtime::simulation::Simulation::step(now)` | tick | aa-client depends on aa-runtime |
| `NetworkSeams` implementation | tick -> orders/net | decrees only in admin ticks |
| Schedule economy Tick (vtable 0xA29B50 slot 5 = 0x4E9C40) | economy | Updatable registration |
| Runtime research refunds call `aa_production::research::refund_research` | economy | not a plain credit |
| Bind economy event callbacks, victory checks, construction/research triggers | economy | presentation/triggers |
| Removal list 4F65B0 drain, scheduling of 57C8E0, shortfall RNG on the world RNG | combat | one removal path with aa-world request_removal |
| Storage: actor overlap list (+0x110), Properties byte +0x1C, Mover falling fields, Mover schedule word, Aircraft component | movement | before mover::tick is wired |
| Organization payload -> `aa_movement::formation`, world RNG | movement | |
| Element-move adapter `formation_ready` -> `formation_follow::formation_speed` | movement | |
| UAIStateMove+0x78 (time of last low-path solve) needs a field in the UAI host state; hosts pass it explicitly today | AI | from the low_path port |
| `LowPathWorld` adapter in aa-spatial; ActionGo 0x5300A0 via `try_action_go` | movement | done: aa-runtime spatial_world `low_path_world` (compiled + unit tests) |
| Runtime hosts for movement adapters: `PlacementHost` leaves (0x4F2220, 0x4D36F0, +0x168, 0x585280/0x585160, +0x100, +0x164, tile +0x10 / node +8, +0x7C/+0x40); `LowPathHost` (source virtual +0, actor +0x110 overlap list, world time, 0x4D10A0); `region::World` / `endpoint::World` hosts calling `SpatialPlacement`; aa-mind `world_4f46e0` -> `SpatialPlacement::nearest`; aa-mind `UaiCollisionWorld` host -> `uai_collision_world` helpers | movement | research/phase8-movement/requests-to-lead.md items 9-15 |
| CAI `CompanyThinkWorld` binding; aircraft step `TickWorld::aircraft_step` -> `aircraft::move_tick`; topology loader duties (clear 0x7AC9A0 first, max-width input, teardown); water-area ids in `TerrainCell.byte_2c` | movement | |
| SAI route cache: build at 0x7A0D40, drop at 0x7AC9A0; `route_cost` for 0x7A1B30 | SAI (research/phase8-sai) | AI agent finished; SAI goal work continues in a new SAI area |
| Map-analysis services 4D0180, actor virtuals +104/+108, 4CEDA0 | combat/actors | research/phase8-ai/requests-to-lead.md item 14 |
| Organization restore calls `aa_mind::cai_member_added::member_added` (524B80) | restoration | |
| Bind ported AI (rout chain, state Thinks, SAI goal priority) with its services (ZOC/ZOG/ZOS, 4F46E0, 52A690, 520440, 578140) | AI | services list in research/phase8-ai/README.txt |
| Kingdom relation virtuals (+0x140/+0x138) mapped to runtime relations | metagame | |
| Presentation: damage numbers 50D0B0, messages 42D490 | combat | client |
| Native world implements `aa_ui::hud::HudWorld` (economy bar, object display, floaters, idle buttons, message log, minimap, status bars, ModelWidget models); session implements `aa_ui::staging::StagingState` | UI | research/phase8-ui/requests-to-lead.md 6-7 |
| Trigger seams (`aa_triggers::elements::Services`, `actions::Effects`): UI command interpreter 50D120, audio, placement/creation, transports, abilities, command processing via `aa_navigation::control::process`, rotation via `aa_trig::rotate`, morale opcode 37 -> `aa_combat::morale::modify` | triggers | full list in research/phase8-triggers/requests-to-lead.md |
| Map load calls `aa_runtime::trigger_world::load_map_triggers` with the LanguageIDS game variable | triggers -> restoration | MAPSTRINGS + TRIGGERS v4/v5 |
| New trigger seams (heroes, teleports, music, fog tile pass, morale 579750 via `organization_modify_morale`, ...) | triggers | research/phase8-triggers/requests-to-lead.md |
| Combat bindings: BeginDeath -> `begin_death::begin_death` (aa-body Event::BeginDeath, RemovalWorld::begin_owner_death, TransferWorld::begin_death, projectile deactivation); capture transfers -> `walls::transfer_wall`/`transfer_component_88`; siege -> `wall_complete`/`intruder_near_wall`/`propagate_under_siege`; RemovalWorld::detach -> `detach_membership`; regeneration spawn -> `spawn_member(.., false, ..)`; performer track_target -> `performer::track_target`; aa-mind uai_kill/element_move -> 57DD90/57DEA0 | combat | research/phase8-combat/requests-to-lead.md section 9 |
| Actor creation 4FBF40 / fresh creation 4F9C10 / DynamicDoodad ctor 4DCB30 (shared entry point) | conversion (classes 2/9 done) / actor-creation (class 3) | |
| `property_flags::Effects` six new services (resource_count, refresh_actor_4cf0f0, create/destroy_reverse_ability, enchantment_type, allied); enchantment instances via `Instance::enchantment`; 5818D0 tick uses `periodic_from_tables`; `admission::effects_for` reads `ability_effects` | combat | research/phase8-combat/requests-to-lead.md section 11 |
| Special operations: order -> `special_ops::use_ability`; `execution::World::other_effect` -> per-kind Apply ports; `periodic::World::expired` -> `enchantment_expired`; `IntruderWorld::within_wall` -> `intruder_within_wall` | combat | section 10 |
| Combat `summon::apply` transport cargo -> `TransportState::load_payload` (58BA90); layout = 4C2290 copy of ActorData+2F0 when cargo is an organization, empty name | combat | saved-components R4-6 |
| Post-load pass `aa_actors::post_load::post_load_pass` after DYNAMIC_THINGS, then 4F1E10 / InitForLoadedMap / conversion tail | restoration | saved-components R4-1 |
| Note: aa-runtime economy_values_world.rs `production_delta` now reads typed `ModifierState.production` (combat edit, approved) | combat/economy | |
| Class-changed conversion hook: `class_conversion::on_placement_class_mismatch` in restore_dynamic + `replace_converted` at load end | conversion -> restoration | clears group 1 (7,879 records) |
| Section index buffer 4FC340 fixed 0x40004 bytes (65,536 entries): make growable | restoration | native cap to record |
| Zones / detection / fog of war (KKC_Zone 515970/515F40/514CE0, ZOC/ZOG/ZOS queries, detection memory 4D67F0, fog tile pass) | zones (new crate aa-zones) | |
| Decree process callbacks (game effects of each order); order 73 / 0x75D810 -> `Timeline::set_game_speed` | orders | research/phase8-orders/inventory.json |
| NetworkSeams adapter in aa-netcode bound into the client frame loop | orders | |
| Combat ChangeKingdom 0x4D8450 -> `aa_production::structure_state::enable_structure` (0x565400); `ConstructionLifecycle::{enable,disable}_structure` -> structure_state | economy | |
| World start 0x4F1E10: call `aa_economy::world_start::world_start` with kingdom list, GTID walk, actor+0x13C, owners, snapshot object, 0x58F010 tracking | economy | after restoration publication |
| Economy world seams 4F9C10, 554DD0/554330/54EFC0, 4F0B70, 555830, 46EE00, 4D6660, OnConstructionFinished/OnUpgradeFinished actor replace/create | economy | research/phase8-economy/requests-to-lead.md |

## Code health

- Duplicate x87 helpers: `aa_economy::x87`, `aa_mind::x87` (and aa-numeric). Consolidate into aa-numeric once the agents finish their rounds.

## Data/loader items (restoration area, aa-rules / aa-mapworld)

- Parser accepts Units/USA/usa_carrier.tgi@2079 and Game/trigger_elements.tgi@37375 as native.
- Localized-key accessor on ResolvedNode.
- FactionData `experience_max` (+0x3C) loaded; commander state built with `with_maximum`.
- Recruit-singles (+0x214) restored.
- Terrain speed cap (actor+0xF8 TerrainData+0x38) from aa-rules terrain_speed.rs.
- aa-actors 4D49B0 and 4D64F0 (needed by combat).

## Zones / detection / fog (aa-zones, 189 functions proven)

- One shared GMap solver store (tile +0x24/+0x28/+0x2D, GMap+0x48 stamp, busy
  byte 0xB3D43D) for the aa-spatial water flood, aa-navigation searches and
  aa-zones. Decision: it will live in aa-spatial (lowest common crate);
  aa-zones' `ZoneMap::solver` and the navigation/water copies move to it at
  integration time.
- Seam bindings: aa-mind 54EFC0/54F8B0/54F3C0/54EF40; supply 515F40/514CE0;
  combat flare 515F40, begin_death 514CE0; production 54EFC0; triggers
  `fog_restore_tiles`/`actor_seen_by`; SAI region counters; UI 50C4C0; GActor
  tile change 4D6350; kingdom alliance change 4E0F90.
- Services to expose: aa-spatial shape/circle tile iteration 554DD0/554330;
  combat/AI engagement lists 0x56EDC0; property modifier 0x501C30.
- Zone queries are pure reads: may run on worker threads from an immutable
  ZoneMap snapshot taken at the native point, consumed in native order.
- Leftovers: zone creators in component paths -> saved-components agent;
  4D7230 -> actor-creation agent; SAI white fog 7B8E70 -> SAI agent; cheat fog
  inside GWorld update 4F9970 and reveal-all 551F90/551FD0 -> world
  integration; memory record object 4EB9F0 -> open.

## Deferred limit decisions

- Live object IDs: native 0x8000 limit (4EFDB0 "no IDs left") is kept for now
  because GTID indices are 16-bit slots in the original save and network
  formats. Lifting it needs an extended save/wire format (versioned so original
  maps and saves still load). Deferred, recorded 2026-10-15.

## Open questions

- Dynamic confirmation of the simulation thread's control word (not run).
- Which thread runs the metagame and the SAI think (precision per thread).
- Mapping Updatable registration callers to classes (heuristic today).
- Tick seams with no owner yet: world creation 0x76AA90 precision path; UI
  callers of SetGameSpeed; 0x76F600 message (UI); 0x41C180 timing stats (client).
