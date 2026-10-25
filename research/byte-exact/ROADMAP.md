# Byte-exact roadmap for aa-play (read-only audit, 2026-10-24)

Goal: make `crates/aa-play` produce the same bytes as AA.exe, one system at a
time, while the playable game (aa-play + `crates/aa-bevy`) never breaks.
Paths are relative to `E:\Games\Axis and Alies\rust-rebuild` unless absolute.
"(inferred)" marks a conclusion that was not read directly from code or data.
Effort: **S** = at most one agent session, **M** = 2 to 5 sessions, **L** = more than 5.

## 1. Headline findings

1. **None of the exact ports runs live yet.** aa-runtime still stops during map
   restoration. The latest stop, in `qa-output/development10/immediate-world-45.log`,
   is "Supply InitExisting 58B0E0/AfterInit 4B2830 ... not yet bound". Each exact
   function is a library behind host traits, so aa-play can act as the host.
   `crates/aa-runtime/src/live_*_tick.rs` are useful reference bindings, for
   example `live_kingdom_tick.rs` for the economy.
2. **The exact-port integration has started.** The fog owner is writing
   `crates/aa-play/src/zones.rs` (aa-zones adapter, modified 2026-10-24 22:51) and
   has added aa-zones to `crates/aa-play/Cargo.toml`. It is not yet listed in
   `lib.rs`. aa-play already calls aa-rules, aa-mapworld, aa-assets,
   aa-navigation, aa-numeric and aa-animation.
3. **Five structural conflicts recur in every system:**
   - Ids: aa-play uses Vec indexes, the original uses GTIDs.
   - RNG: aa-play uses xorshift64* (`world.rs:293`), the original uses the LCG 0x85D840.
   - Scheduling: aa-play runs the systems in a fixed sequence (`sim.rs`), the
     original runs per-Updatable queues inside 0x4F9970.
   - Numbers: plain f32 against x87 PC53.
   - Iteration: Vec order against native list orders.
4. **Recorded films decode with existing pieces, and they carry the
   original's own checkpoints.** Every film contains `KKC_SetSimSyncValueOrderDecree`
   (311) records holding {S index, S hash} about every 17 world ticks (section 5).
5. **All three recorded films are random-map games** (`KKC_WorldCreatorRandom`,
   `rmc_southerneurope`). Replaying them needs an exact random map generator
   (RMG), which has not been ported. Alternatively, record a new film on a stock RMP map.
6. **`aa_numeric::Binary` is backed by BigUint and allocates on every
   operation.** Per-tick exact systems need a fast PC53 path (F3).
7. **The strategic AI (SAI) cannot be byte-exact in live play.** Its RNG is
   seeded from QPC (`aa_mind::sai::manager::SaiRng`, ctor 0x7A0AA0) and it runs on
   wall-clock fiber budgets. This does not block film parity: the SAI's
   decisions leave it as orders, so films record them (inferred from 0x7AF220
   `KKC_SAICommandData::Send` and the order path).

## 2. aa-play systems → exact ports

### 2a. Coverage

| aa-play module (owner) | exact port: crate::module and native addresses | proof (oracle fixtures) | refusals and gaps |
|---|---|---|---|
| data (core) | aa-rules typed readers 464BD0/4BF990/4C3610/4605B0/582C40; native registry order in aa-runtime `catalog::Catalog::native_data_registry`, `definitions::Bank` | phase3-rules, phase8-definitions | FactionData+0x3C `experience_max` not loaded |
| map (core) | aa-mapworld `archive`/`WorldInventory`/`header`/`terrain`; aa-spatial height 4F0E50 and `height_layers`; aa-heightfield | phase5/6 terrain, phase6-mapheader | `MapHeader::decode` refuses derived creators (`header.rs:305`). aa-play heights come from `aa_assets` render heights, not 4F0E50 |
| setup (core) | aa-economy `kingdom_init` 4E5DF0, `kingdom_start` 4E6DB0/4CDF10, `world_start` 4F1E10; aa-actors `actor_creation` 4F9C10/4FA030/4FA150 and `registry` 4EFDB0; aa-runtime `configure_world.rs` (random nation/colour 474B40) and `map_replacement_queue.rs` | economy world start 360; about 55 aa-actors fixture files | `actor_creation::Error::Unsupported`: GTID-handle services, non-actor shells, 4EB490/4EBB80 with +58 > 0 |
| economy (core) | aa-economy `tick` 4E9C40 (with 4E12D0, 4CDFD0, 4E37D0, 58ED00), lib 4E3940/4E1F60, `kingdom_registry` 4E6180/4E4470, `statistics`, `transfers` | 13 fixtures; tick 414 cases / 1008 executions; registry 360/2625 | no "not ported" variant; seams 42D4B0 (events) and 4E9720 |
| production (core) | aa-production queue 5836E0/582950/582AE0/589E00/588D60/589420/5894C0/588CD0/589BD0/589F90; `research` 584790; `purchases` 574C10 (attach); `structure_state` 565400 | 13 test files, 300 to 900 fixtures each | the recruit spawn 4FA2C4/4FACA0 depends on aa-actors creation |
| deploy (core) | aa-actors `mobile_deployment` 575940; aa-production `structure_state` 565400/564DC0/565140 and `construction` | phase8-economy, actor fixtures | unpacking replaces one actor with another, which needs the setup creation path |
| supply (core) | aa-economy `supply` 58AE10/58ADB0/58ACE0/4D0A80/4D0C90/4D7CE0/56F2B0; aa-combat `regeneration` 57B5A0/57AEB0/57AD30; aa-zones ZOS | supply 270 / 2064 | the supply zone creator 58B0E0 is not in aa-zones |
| path (movement) | aa-navigation `solver` 53E1F0, `search` 53D8D0, `endpoint` 53D340/552640, `cost` 53CB70, `materialize` 53D500/53CF40, `region` 53DC00, `route_cache`, `topology` 7AA9B0/7ACB00 | phase6 search/cost/endpoint/solve/materialize; route cache 84 cases / 1092 searches | aa-play skips the region pre-pass 53DC00; 7AA180 (max rectangle width) is an explicit input |
| formation (movement) | aa-movement `formation` 5797E0/578250/57BEF0/57BA80/579A00 plus the 2048-entry direction table | `native-formation.json` 477 | slot masks above 31 are not aliased |
| movement (movement) | aa-movement `mover` 577E50, 5773C0/5774A0, `falling`, `aircraft` 5612F0; aa-navigation `company_move` 520320/520440/5276B0/525740, `formation_follow` 52FF00, `low_path` 53C170, `uai_collision` 534A40, `action_go` 5300A0; aa-mind `element_move` 537B90/5379C0, `node_reached` 530AE0 | mover 1251; company move 756; think 660; formation follow 498; low path 411; uai collision 636 × 3 precisions | not ported: aircraft 55F230/560A70/55EDC0, Mover save/load 576D50/576E90. `element_update` 531D30 has no fixture |
| combat (combat) | aa-combat lib 4CA400, `admission` 4C8F20, `execution` 4C9550/4C9C90, `performer` 57F4D0, `engagement` 579490, `death` 4F1A90, `capture` 4D1120/4D5230, `ownership` 4D8450, `combat_value` 45E090; aa-body `damage` 5625C0, `credit` 4D57C0; aa-mind `cai_engage` 523DD0, `uai_kill` 5340C0 | 183 proven (aa-combat README round 3), e.g. combat value 480/108/180 | SUMMON deferred 4CC890; CRT pow x87 body 96D8ED (`X87PathUnsupported`) |
| morale (combat) | aa-combat `morale` 578140…57A0A0, `organization` 57C8E0/57CD70; aa-mind `cai_rout` 521240/51F190, `rout_select` 52B530 | morale 711; org tick 186; member removal 114 | — |
| ai, ai/* (opponent) | aa-mind `sai/*` (7A0080, 7B4AD0→7A9840, 7AC0D0, 7AF220, 7A5E10); aa-navigation `topology_builder` 7ACB00, `route_cache` | phase8-ai 52 fixtures / 24,393 cases; phase8-sai 5 | not ported: goal +58 execute, military overrides, attack waves, 4A7CD0. aa-play `regions.rs` uses square blocks instead of 7ACB00 |
| victory (opponent) | 4E9960/4E9A40 as `aa_economy::tick::TickWorld` leaves, ported in `aa-runtime/src/live_kingdom_tick.rs`; aa-triggers `objectives` 4E7050/4E2120/4E9720 | economy tick fixtures | 4E9720 SetMissionResult is a seam |
| fog (UI) | aa-zones `set`/`query`/`tile`/`zone`/`solver`/`flood`/`component`/`detection` | 9 fixtures; 189 of 247 functions proven | zone creators 566DB0/568040/563E10/58CF50/585940/4D7230; memory record 4EB9F0; reveal-all 551F90/551FD0 |
| player_orders (UI) | aa-orders 358 descriptors (`native_registry.rs`), `payload/*`, `flow`; aa-runtime `player_order_*` (guess 4DA570, decree, world); aa-production sell 565500 | order traces | gameplay process handlers (766CE0 leaf) are external |
| special_ops (combat) | aa-combat `special_ops` 4CDE00, `summon` 4CCA70, `payment` 4E2590, `periodic` 5818D0; aa-economy `transfers` 4CA680 | about 600 plus 160 | enchantment kinds outside FLAG/ADD/RESIST/PERIODIC_DAMAGE |
| mapgen (mapgen) | **no port of KKC_RandomMapGenerator 5925E0**. Pieces only: aa-heightfield brush (559470), aa-spatial `water_areas` 539C40/539E30, aa-metagame `rmc` 4A5420 | — | aa-spatial leaves 5972F0/597530 and the brush stages 5526C0/559FB0/5528F0/551140/551B40 as traits |

### 2b. What connecting each one takes

| aa-play module | traits to implement (file) | data aa-play must add | conflicts with aa-play's layout | effort |
|---|---|---|---|---|
| economy and victory | `aa_economy::tick::TickWorld` with `TickParts` (`tick.rs:125`); `kingdom_registry::RegistryWorld` (`:90`). Template: `aa-runtime/src/live_kingdom_tick.rs` | Kingdom +1A0/+1AC/+1B8/+1C4 resource vectors, +190/+1F0/+1F4/+334 words, +214/+220, Commander +230, statistics +32C | aa-play recomputes contributions every tick. The original adds and removes them incrementally (570020) and runs 4E9C40 every tick on queue0 (1 bucket). Commander XP is a single store shared with special_ops | M |
| production | `aa_production::World`, `Recruitment` (quote twice, debit, reserve), `research::Research`, `purchases::*` | queue records with native timers in binary32; actor state +F0 mask (0x1B/0x1D/0x1E) | spawning needs GTIDs (F2) | M |
| supply | `aa_economy::supply::SupplyWorld` (`supply.rs:120`); aa-combat `organization::World::regenerate` | SupplyActor +14/+8/+100/+104/+8C/+80/+7C/+78/+D4/+E8/+44/+48/+148; tile relay 54F8B0 | needs ZOS zones from zones.rs; the component tick runs on queue1 (1 s, 16 buckets, the bucket drawn from the RNG) | M |
| path | `search::SearchWorld`, `solver::World`, `materialize::World`/`PredecessorGrid`, `endpoint::World`, later `region::World` | `SearchGrid` (tile flags +0x20, stamps +0x28, epoch +0x48) and a blocked outer ring, which `map.rs` already has | the solver scratch (+0x24/+0x28/+0x2D, GMap+0x48, busy byte 0xB3D43D) must become one store shared with zones and water, in aa-spatial (backlog decision). A private copy is acceptable while path runs alone (inferred) | M |
| formation | `aa_movement::formation::FormationWorld` (`next_random_u32`, `mind_leader_changed`, `detached_member`) | organization payload +0x18…+0xC0, slot arrays with null holes | draws the world RNG (F1) | S–M |
| movement | `GroundWorld`/`TickWorld` (`aa-movement/src/lib.rs:152`, `mover.rs:70`); `NavigationWorld`, `CompanyWorld`/`CompanyThinkWorld` (about 35 methods); `LowPathWorld`, `ContinuationWorld`, `UaiCollisionWorld`; aa-mind `element_move::World` | MoverState (+0x10, +0x44, +0x48); actor +0x110 overlap list; Properties +0x1C; Mind state arena (embedded states by native offset); tile +8 actor lists in head-first order | requires F2 (GTIDs) and F4 (per-Mind Think scheduling). The world RNG is drawn in ActionGo 5300A0, scatter 5233F0 and the formations | L |
| combat and morale | `aa_combat::World` (`lib.rs:105`), `execution::World`, `admission::World`, `organization::World` (`:43`), `RemovalWorld` (`:348`), `morale::World` (`:148`); aa-body `damage::World`, `credit::World` | Organization +5C/+60/+64/+84/+88/+90/+B8/+D4/+D8; actor flags +8/+100/+104/+118; prepended lists (+128, +130); LIFO death queue world+100 | needs native list orders and GTID handles that resolve to null when stale | L |
| fog (in progress) | `ZoneWorld`, `FloodWorld`, `ZoneActors`/`VisitWorld`, `ZoneObjectWorld`, `ActorDetectionWorld`, `DetectionComponentWorld` (already in `zones.rs`) | detection words +0x54, memory +0x58/+0x5C, zone slots +0x148 | zones.rs keys actor i as i+1. Re-key to GTIDs after F2, because the ZonePool interning order depends on identity | M (in progress) |
| special_ops | `execution::World::other_effect`, `periodic::World`, `property_flags::Effects` | enchantment instances | commander XP comes from the economy store | M |
| player_orders | `aa_orders::flow::OrderHost` / `TryOrderHost`, `payload::{Resolver, SchemaContext}` | Thing slot to actor map (F2); native registry counts | `Command` enum to decree mapping (H4) | M |
| ai | `sai::engine::EngineWorld`, `assign::AssignWorld`, `manager::ThinkHost`, `goal_priority::GoalWorld`, `map_analysis::MapWorld`, `travel::TravelWorld` | regions from 7ACB00, SAI player lists that prepend | QPC seed and wall-clock budgets (risk R5) | L, do last |
| mapgen | a new exact RMG port; existing pieces above | the world RNG after 0x85D880(seed) and the GWorld::Init 0x4F27E0 draws | `mapgen/rng.rs` uses PCG32 | L |

## 3. Missing game modes

| mode | what exists | what blocks it | first step | effort |
|---|---|---|---|---|
| Films / replays | Writer: aa-runtime `film_resource.rs`, `film_header*.rs`, `film_create.rs`, `film_creator.rs` (proven by `research/skirmish-20261017-film*`). Reader pieces: `aa_mapworld::MapArchive` (TGCK with zlib), `aa_orders::net::admin::AdminNode::play_film_record` (0x751A40), `aa_orders::net::sync::Synchronizer` (0x77D050) | no film-file reader; derived creators refused; aa-netcode `local_frame_host.rs:461` returns `required("actual retained positive replay source26C")` | harness H1–H3 | M |
| Multiplayer | aa-netcode (99 files; session, lobby, transport, `NetworkSeams`); aa-orders admin/decree lockstep and history (keeps 1000); sync A/S 302/314 and 299/311 | Rust-to-Rust play only needs aa-play to be deterministic for one binary. Playing against AA.exe needs every system exact plus SyncA/SyncS parity | Rust-to-Rust lockstep over aa-orders decrees mapped to `Command` | L |
| Saved games | aa-runtime `restoration.rs` plus `saved_*.rs`; aa-mapworld `restore*`; aa-metagame `save` 938EE0/93A3B0 | restoration stops (above); `aa_metagame::chunk_file::ChunkFileReader` refuses compressed chunks (`chunk_file.rs:332`; aa-mapworld `archive.rs:158` can inflate them); aa-play has no save or load at all | a non-native aa-play save (serde) for players; native SAV after F2/F4 | L |
| Campaigns | aa-triggers: all 115 element types proven, 79 used by the 26 missions, 3993 mission instances dispatch; aa-runtime `campaign_manager.rs`, `trigger_world.rs` | one host must implement `world_program::World` + `elements::Services` + `actions::Effects` (about 60 methods) + `extended::Effects` over aa-play; mission maps; scripted SAI enable/disable (`aa_navigation::control`) | trigger host over aa-play for one tutorial map | L |
| WW2 world map | aa-metagame: 377 of 1053 functions ported, 341 proven (24 fixtures, 5 campaigns): turns, resolve, path, MGAI, save, launch 948EF0 | hosts `KingdomRelations`, `resolve::Engine`, `LaunchEnv`, `MgaiHost`/`MgaiEngine`; string tables 4B9F40; UI; battles use RMC random maps | launch `battle_setup` 948EF0 into `SkirmishSetup` and return `MgState::rts_winner` (MGState+0x48), then `process_rts_result` | L |
| Editor | only order payload schemas (`aa-orders/src/payload/editor.rs`, descriptors 154–297) | no editor port | none planned | L |

## 4. Cross-system requirements for byte-exactness

### 4.1 RNG

- **Native generator.** 0x85D840: `w = state*0x41C64E6D + 0x3039` (u64); `state = w as u32`; return `(w >> 16) as u32`.
  - Seed 0x85D880/0x85D8A0: `((s<<16)|s) ^ 0x8E4B12A5`; a zero result means the default state.
  - Bounded draw: `% n` with no rejection (0x40C8A0).
  - Unit float (0x45D400, inline 0x4F62BB):
    1. Read the draw as a signed integer; when negative, add 2^32.
    2. Multiply by `0x3DF0000000100000`.
    3. Round to the precision control, then store as f32. The result can equal 1.0.
  - Ported in `crates/aa-simulation/src/{arithmetic.rs,random.rs}` (`NativeRng`, `from_native_seed`, `next_bounded`, `native_unit_float`), proven by `research/phase4-rng/native-rng-vectors.json` (11,345 cases).
- **World stream.** The state is GWorld+0xD4. KKC_Sim::CreateGWorld 0x76AA90 seeds it with 0x85D880(creator seed); GWorld::Init 0x4F27E0 follows.
  - The getter 0x4F02B0 logs `(int)m_rng.GetSeed()` (the state) to SyncA, then SyncS (`aa-runtime/src/configure_world.rs:124`).
  - **Every getter call therefore changes the film's S hash, even when it draws nothing.**
- **Draw sites.**
  - `calls.tsv` lists 58 callers of 0x4F02B0 and 133 of 0x85D840.
  - The census `research/phase8-tick/rng-sites.json` has 209 sites, 66 of them through 0x4F02B0.
  - A periodic-queue registration 0x50B2C0 draws once to pick its bucket (`docs/WORLD-ORDER.txt` §6, `aa_world::queues`).
- **aa-play today.**
  - xorshift64* `World::random()` (`world.rs:293`, f32, 24-bit). There is a second private copy at `movement.rs:605`.
  - RNG grep matches per file: ai.rs 9, combat 3, economy 1, morale 5, movement 1, special_ops 1, ai/goals 20, rai 4, value 2, special_ops/sai 4. The ai, goals, rai and value matches include `SaiRng` uses.
  - mapgen uses its own PCG32 (`mapgen/rng.rs`, about 50 matches across `mapgen/**`).
  - `ai.rs:222` seeds `SaiRng` from `world.rng`, which the original does not do.
- **Other streams that must stay separate:** SAI `[0xB36528]+0x2C`, the application RNG 0xB39714 (`time32`), the view RNG `[0xB3653C]+0xD4` and the metagame `MetaGameRng`.

### 4.2 Tick and system order

- **Frame.** 0x75CA70 runs at most one step per frame.
  - A world tick is 0x752AE0, then film record 0x74D640 or playback 0x751A40, then 0x76F790, then **0x4F9970**.
  - Otherwise it runs an admin tick 0x755290: decrees, and `SetSimSyncValue` through 0x74E670.
  - In playback, decrees run immediately before 0x4F9970, so aa-play's `step(commands)` has the right shape (`docs/WORLD-ORDER.txt` §1).
  - The speed decree is order 73; speed changes the interleaving only, never what a world tick contains.
- **0x4F9970** (`aa_world::World::step`) runs in this order:
  1. fog callback;
  2. phase 1;
  3. first tick 0x4F96D0;
  4. `tick++`, `end = f32(i32 tick * step)` with step = f32(1/16);
  5. deferred payloads +1A4;
  6. queues +1AC (every tick), +1C0 (1 s, 16 buckets), +1D4 (5 s, 80 buckets), +1E8 (deferred);
  7. phase 4 and the trigger manager 0x54C280;
  8. phase 5 and queue +208;
  9. phase 0 and the retained-list scan +110.
- **Registration.** Updatables register through 0x50AC00 (queue 0: Kingdom 4E5DF0, Organization, Performer, …), 0x50AC60 (queue 1: Body, Character, Economy, …), 0x50ACC0/0x50AD20 (queue 3: GThing, CAI, …) and 0x50AD80 (queue 4: UAI 0x52DC10).
- **aa-play.** `sim.rs` runs ai → economy → production → deploy → movement → combat → morale → supply → victory → player_orders → fog → special_ops, system by system. This is exact only for systems that tick once per kingdom per tick (economy). Everything with Think delays (CAI/UAI states, 1 s and 5 s components) needs the queue scheduler (F4).

### 4.3 x87 precision

- **Decision.** The simulation thread runs at PC53 (CW 0x027F) and sets PE every tick (`research/phase8-tick/fpu-precision.md`). This has not been confirmed dynamically (§6 of that file).
- **aa-numeric.**
  - `aa_numeric::Binary` is exact but backed by BigUint.
  - aa-economy keeps its own frozen copy (`binary.rs`, `x87.rs`); aa-mind has `x87.rs`; aa-runtime has `player_order_input/x87*.rs`. These need consolidating.
- **Trigonometry.** aa-trig uses lookup tables (exact) plus host x87 assembly for 0x445400 and 0x52FD90 (x86_64 only).
- **Oracle limit.** Unicorn computes FPATAN, FSIN, FCOS and FYL2X in binary64 and never sets PE (`research/phase8-tick/emulator-limits.json`).
- **F3 (inferred, must be proven).** For normal-range results, PC53 add/sub/mul/div/sqrt round once to 53 bits with ties-to-even, the same as IEEE binary64. Host f64 therefore matches, followed by an f32 store when the native code stores binary32. Fall back to `Binary` for subnormal or out-of-range values. Gate it with differential tests over every existing fixture.

### 4.4 Iteration orders that must be kept

| order | native rule |
|---|---|
| GTID enumeration | ascending slot (0x4EFBE0) |
| kingdom vector | GWorld+0x154 with null holes |
| tile actor list +8 | head first |
| queues | insert at head; bucket drawn from the RNG; every 1000th deferred callback skipped |
| deferred payloads +1A4 | forward; `next` read after the callback |
| removal +108 and death queue world+100 | LIFO |
| retained +110 | prepends wait for the next scan |
| engage lists | +128 and +130 prepend |
| organization member slots +0x28 | null holes; the count is reread on every iteration |
| zone-set interning | depends on map-wide history (one global `ZonePool`) |
| SAI player lists +0x4C/+0x54 | prepend (0x79FC40) |

aa-play iterates `Vec` order (creation order) everywhere.

### 4.5 Actor ids

- **Allocator.** `aa_actors::registry::IdentityRegistry` (`crates/aa-actors/src/registry.rs:66`; 4EFDB0/4EFEC0/4F0190) behaves as follows:
  - 65,536 slots, with slot 0 reserved;
  - `id = generation<<16 | slot`;
  - Add uses a next-fit cursor that wraps from 65535 to 1;
  - Remove increments the slot's generation;
  - live objects are capped at 0x8000 ("no IDs left").
- **Map load.** Saved ids are registered without the cap, from the IDMGR chunk (0x4F0230) and DynamicThings. aa-play already keeps them in `map::PlacedThing::serialized_id`.
- **Orders.** Things are referenced by their 16-bit slot (`aa_orders::payload::Field::Thing{slot}`). Without native ids, a film order cannot name an aa-play actor.
- **aa-play today.** `ActorId` is the `Vec` index and is never reused. aa-bevy uses the `World` API 219 times in 27 files, so keep `ActorId` as the façade and add a GTID side table (F2).

## 5. Film parity harness

### 5.1 What the three original films contain

The films are in `E:\Games\Axis and Alies\reconstruction\qa-runs\compare-20261022\game\Profile\data\Films`. The numbers below were updated on 2026-10-24 from the `crates/aa-parity` decoder (aa-orders schemas); the first draft used an ad-hoc bit reader. Run `cargo run -p aa-parity -- film <path> --game-root <root> --inferred-registries` to reproduce them.

- **Container.** Each film is a TGCK v2 file with 5 chunks, all zlib except MAP:
  - `MAP` (4 bytes, zero);
  - `MAP_HEADER` (about 12.5 KB);
  - `MAPSTRINGS`;
  - `SessionPlayers` (48 bytes: side01 human, side02 AI);
  - `Orders`.
- **Orders.** The Orders chunk splits cleanly into `[u32 len][packet]` records, **one per world tick** (0x74D640). A 1-byte record (`05`) is an empty tick.
- **Decrees.**
  - Every non-empty record starts with `TGC_SetPlayerOrderDecree` (298). Player ids are 1 (human, side01) and 2 (computer, side02).
  - The other decrees are:
    - 311 `KKC_SetSimSyncValueOrderDecree`;
    - 333 SelectionSet, 16 SelectionAdd and 18 SelectionClear;
    - 86 `KKC_TellSelectedCommandOrderDecree`;
    - 328 UpdateKingdomName.
  - **Every 86 command in the three films is ordered by player 2, the SAI.** The QA runs gave the human no orders. This confirms that the AI's decisions are in the film.
  - The ActorCommands in film 0 are guard 229, retreat 49, set_formation 30, kill 27, move 19, recruit 10, unpack 4, group_behavior_enable 2 and bombard 1.
  - The 311 records hold {S last index, S hash} and come every 17 world ticks, from tick 17.
- **Creator.** The MAP_HEADER creator class is `KKC_WorldCreatorRandom`, with `rmc_southerneurope` and biome `chaparral`. Its `rng_seed` matches the seed in the original's log. The header's own rules carry WorldRuleMode 3 (film).

| film | log | creator seed | Orders bytes | world ticks | non-empty | decrees | 311 checkpoints | first checkpoint (tick, count, hash) | at tick 34 | last count |
|---|---|---|---|---|---|---|---|---|---|---|
| LastFilm_0 | none | 3421562015 | 71,865 | 10,711 | 919 | 2,655 | 630 | 17, 30, 53151 | 51, 29082 | 46,851 |
| LastFilm_1 | `Logs/log-4-ok.log` | 1373806882 | 30,538 | 5,037 | 337 | 771 | 294 | 17, 30, 23631 | 51, 7386 | 15,304 |
| LastFilm_2 | `Logs/log-6-ok.log` | 972257557 | 1,624 | 256 | 20 | 46 | 15 | 17, 30, 2327 | 51, 51390 | 350 |

**Registry counts.** These counts decode every record exactly to its packet end, except one record of film 1 at tick 486, which needs the SharedProperty `[0xB3AA2C]` count:
- proven: ThingData 1959 and ActorCommand 39;
- inferred: Formation 5, OrganizationLayout 10 and AbilityDefinition 53.

With the proven counts only, 39, 14 and 1 records stop early with an explicit error.

**What the S log holds.**
- **Clock entries.** Per world tick the driver logs exactly one entry to S, `g_gworld->GetGameTimeBeginTick()` (`aa-netcode/src/local_frame_host.rs:410-441`).
  - A clock value k/16 has zero low 16 bits, so it only multiplies the hash by 37. All film hashes come from the other entries.
- **Other entries** (catalogued on 2026-10-25 in `SYNC_LOG.md`): S receives only:
  - the decree name + id of every S-logging decree;
  - the world-RNG seed logs: getter 0x4F02B0 and the inline loggers 0x4F4A20/0x4FDA70. Each getter is followed by exactly one draw.

  Markers, bools and dumps go to A only. Film 2's first checkpoint is 17 clocks + 10 decree entries + 3 getters.
- **Counts per interval.** The first interval holds 30 entries for 17 ticks, so 13 are not clock entries. Later intervals hold 19 to 33 entries per 17 ticks.
- **No clock-only interval.** In the three films no interval holds clock entries only.

**Harness status (2026-10-24).** H1–H3 are built in `crates/aa-parity`, together with the decree → `Command` mapping and the S-log checker. The first comparison ran: aa-play on a generated map with film 2's seed and players, no decrees replayed, against film 2's checkpoints.
- aa-play logs 19 entries by tick 17 (17 clocks and 2 getters); the film has 30.
- None of the 15 checkpoints matches.
- The gap is the S entries aa-play does not emit yet: about 11 in the first 17 ticks and about 2 to 14 per later interval.

Run it with `cargo test -p aa-parity --test first_comparison -- --ignored --nocapture`.

**Other native checkpoints in the logs.**
- `KKC_Sim::CreateGWorld - RNG Seed`.
- RMG stage lines: terrain normalized weights (RNG-free), team and kingdom marker balance scores, marker coordinates, failed-creation coordinates.
- `StaticMapAnalysis complete with 983 rectangles, 339 regions, and 37 islands` (7ACB00).
- `actor N changing to kingdom sideNN` (GTID slots).
- The binding lines `bind player ... kingdom ...`.

**Sync dump.** The original can dump its sync history to files: UVars `SyncLogPath`, `SyncLogSimPath`, `SyncLogSimRecordingPath` (default `synclogs/synclog_film_record_`) and `SyncLogBackupPath`, registered at 0x41F9D0. The dump path is 0x75BBC0 → 0x75B8D0 → 0x77D0D0. What triggers it was not verified. A playback mismatch logs `FILM OUT OF SYNC(%d)` (0x752F60) and stops playback (0x751EB0).

### 5.2 Exists or missing

| piece | status |
|---|---|
| TGCK plus zlib chunk reader | exists: `aa_mapworld::MapArchive::decode_chunk` |
| MAP_HEADER base fields (rules, parameters, seed) | **done**: `MapHeader::decode_any_creator` (additive, `crates/aa-mapworld/src/header.rs`) reads `KKC_WorldCreator`, `KKC_WorldCreatorLoad` and `KKC_WorldCreatorRandom` (RMC settings), plus the film WorldRuleMode 3; `MapHeader::decode` is unchanged |
| Per-tick decree decode | **done**: `aa_parity::Film`, using the native registry, the aa-orders schemas and `aa_parity::Registries` (counts from `load_v14`; SharedProperty still missing) |
| Sync hash | **done**: `aa_parity::SLog` over `aa_orders::net::sync::Synchronizer` (reset 0x77D900(0, 0), native `validate` rule) |
| Decree → `aa_play::Command` | **done**: `aa_parity::Mapper` through the `NativeIds` table aa-play must provide. Mapped: kill, move, stop, disband, unpack, recruit, set_rally_point, sell, attach, hq_detach, set_formation, pack, research, plus 65, 66 and 70. The other 26 ActorCommands have no aa-play equivalent (`classify::ACTOR_COMMAND_MAPPING`) |
| Thing slot → aa-play actor | **missing** (needs F2) |
| Same initial world | **missing** for these films (needs an exact RMG); for an RMP film, aa-play `map.rs` and `setup.rs` plus native creation order |
| Native state oracle beyond the hash | **missing**: either the sync dump above, or a memory sampler during the original's film playback (GWorld `[0xB36534]`, tick +0xDC, kingdoms +0x154; inferred feasible) |

### 5.3 Shortest path to a first comparison

1. **H1 (S).** New crate `crates/aa-parity`: RFM → chunks → SessionPlayers → per-tick records. Compare the seed and the players with the logs (passes today by hand).
2. **H2 (S).** Make `crates/aa-mapworld/src/header.rs` accept `KKC_WorldCreatorLoad` and `KKC_WorldCreatorRandom` (additive).
3. **H3 (S–M).** Produce a JSON decree timeline through `play_film_record` with a recording host, listing every 311 checkpoint. This is the first output.
4. **First RMG-independent comparison (S, needs F1 and G1).** aa-play emits an S log (GameTimeBeginTick plus a getter entry at each world-RNG use) and compares {index, hash} at tick 34.
   - If the counts differ, the gap is the missing marker and getter entries. Fill it from the sync dump or from the 0x4F02B0 caller list.
   - The RMG-free log checkpoints can be compared at the same time: terrain normalized weights against aa-play `mapgen` BuildTerrainWeights 0x598260.
5. **First behavioural comparison (M).**
   1. Record a new short film on a stock RMP map with the hidden-desktop tools in `compare-20261022/tools`.
   2. Replay its decrees through H4 into aa-play.
   3. Compare kingdom stockpiles and production timers per tick for the first N ticks: the economy is RNG-free and runs every tick on queue0.
   4. Take the native values from the S hash, plus a memory sampler or the sync dump.

**Verdict.** The harness is feasible, and most of the decoding already exists. The three existing films are good for decode, timeline and S-hash work now. They are not good for behavioural replay until the RMG is exact. A stock-map film unlocks economy and production parity first.

## 6. Ordered plan

**Guard rails** (they come first and keep the game playable):
- **G1 (S).** Add `crates/aa-play/src/trace.rs` (new): a per-tick digest (RNG state, stockpile bits, actor pos/health bits, events) plus an optional S-log emulation through `aa_orders::net::sync::Synchronizer`. Record golden baselines of today's behaviour.
- **G2 (S).** Add an `ExactSystems` set chosen in `SkirmishSetup`, fixed for the whole battle. The f32 path stays the default until a system passes its parity gate. Never switch mid-battle. Keep the `World` public fields as a projection for aa-bevy.

| # | item | effort | owner and files | depends on | why this position |
|---|---|---|---|---|---|
| 1 | H1–H3 film decoder and timeline | S–M | harness: new `crates/aa-parity/**`, additive `crates/aa-mapworld/src/header.rs`, one `Cargo.toml` members line | — | measuring comes first; no aa-play file is touched |
| 2 | G1 and F1: trace, native RNG and clock (`NativeRng::from_native_seed`, `native_unit_float` at PC53, `time = f32(i32 tick * step)`, mapgen draws from the world RNG, SAI seeded separately) | S | shared, by agreement: `aa-play/src/world.rs`, `sim.rs`, `lib.rs`, new `trace.rs`; mapgen owner: `mapgen/rng.rs` | — | everything else draws from it; random outcomes change, so re-baseline the tests |
| 3 | S1 economy and victory exact (`aa_economy::Economy::tick` 4E9C40 through `TickWorld`; 570020 incremental contributions; 4E9960/4E9A40) | M | core: `economy.rs`, `books.rs`, `victory.rs`; tell the combat owner (commander XP store) | 2 | most proven, RNG-free, visible on the HUD |
| 4 | S2 path solver exact (`aa_navigation::solver::solve` 53E1F0 with search/endpoint/cost/materialize) | M | movement: `path.rs` | 2 | proven, self-contained, visible routes |
| 5 | F2 GTID side table (`IdentityRegistry` seeded from `PlacedThing::serialized_id` and IDMGR; next-fit allocation in `spawn_actor`) | M | shared/core: `world.rs` (`spawn_actor`), `setup.rs`, new `ids.rs` | 2 | needed for film order replay (H4), zones re-keying and all list orders |
| (now) | S3 fog over aa-zones | M | fog/UI: `zones.rs`, `fog.rs` | re-key after 5 | in progress |
| 6 | H4 decree→Command driver and H6 stock-map film; first economy/production comparison | M | harness: `crates/aa-parity/**` | 1, 3, 5 | first behavioural parity |
| 7 | S4 production queue exact (5836E0…589F90, research 584790, attach 574C10) | M | core: `production.rs` | 3, 5 | visible recruit timing |
| 8 | F3 fast PC53 arithmetic and the x87 helper consolidation | M | numeric: new `crates/aa-numeric/src/fast.rs` | — (parallel) | blocks per-unit systems for performance |
| 9 | S5 formation exact (5797E0/578250/57BEF0/57BA80) | S–M | movement: `formation.rs` | 2 | small, proven |
| 10 | F4 queue scheduler: `sim.rs` → `aa_world::World::step` 0x4F9970 plus `aa_world::queues` registrations | L | shared: `sim.rs`, `world.rs`, all `tick()` entry points | 5 | prerequisite for every Think-driven system |
| 11 | S6 supply and regeneration (58AE10…, 57B5A0) | M | core: `supply.rs` | S3, 10 | |
| 12 | S7 mover and element movement (577E50/5774A0, 537B90, 53C170, 534A40) | L | movement: `movement.rs` | 8, 10 | |
| 13 | S8 combat and morale, S9 CAI/UAI states | L | combat: `combat.rs`, `morale.rs`; opponent: aa-mind Think hosts | 10, 12 | |
| 14 | S10 deploy, player orders, special ops | M each | core `deploy.rs`; UI `player_orders.rs`; combat `special_ops.rs` | 5, 10 | |
| 15 | S11 exact RMG (5925E0 stages, checked against the log checkpoints) | L | mapgen: `mapgen/**` | 2 | unlocks the existing films and WW2 battles |
| 16 | S12 SAI | L | opponent: `ai.rs`, `ai/**` | 10 | not needed for film parity |
| 17 | modes, in this order: films (record and play in aa-play), Rust↔Rust multiplayer, saved games, campaigns (aa-triggers host), WW2 (aa-metagame hosts), editor | L each | new modules | the items above | |

**Lanes that can run in parallel** without file conflicts:
- harness (`crates/aa-parity`, `aa-mapworld/src/header.rs`);
- core (`economy.rs` → `production.rs` → `supply.rs`);
- movement (`path.rs` → `formation.rs` → `movement.rs`);
- fog (`zones.rs`, `fog.rs`);
- numeric (`crates/aa-numeric`);
- mapgen (`mapgen/**`).

**Serialize** (one agent at a time, by agreement): `world.rs`, `sim.rs`, `lib.rs`, `books.rs` and the workspace `Cargo.toml`. Items 2, 5 and 10 are the only ones that change shared files substantially.

## 7. Risks

- **R1.** BigUint arithmetic per operation may be too slow at 16 Hz × game speed (up to 64×). F3 must land before per-unit systems switch.
- **R2.** Ported functions are not transactional: they return errors after partial writes and must not be retried. Policy: an exact-path error in a battle ends parity (log it and mark the trace), never panics the game, and never falls back to f32 mid-tick.
- **R3.** aa-bevy depends on the `World` fields (219 uses in 27 files). Keep the façade and project the native state into it after each tick.
- **R4.** All three existing films need the RMG, which is not ported. Record stock-map films early (H6).
- **R5.** SAI decisions are nondeterministic natively (QPC seed and fiber wall-clock budgets). A byte-exact live AI is out of reach; replays use the recorded orders.
- **R6.** aa-trig uses host x87 assembly (x86_64 only, CPU-dependent transcendental results possible, inferred). Unicorn oracles for FPATAN/FSIN/FCOS/FYL2X are binary64 approximations. PC53 is not dynamically confirmed.
- **R7.** Which simulation functions log to S is not catalogued (42 marker and 379 int-logger callers, A and S mixed). An S-log census is needed before S-hash parity beyond the first checkpoints.
- **R8.** Half-native layouts: the trait methods ask for native fields aa-play lacks (Mind arena, +0x110 overlap list, slot holes, the shared solver scratch). Add them in side tables owned per system, never by repurposing shared fields.
- **R9.** Switching to the native RNG and scheduler changes current outcomes. Re-baseline the aa-play tests (`crates/aa-play/tests/*.rs`, 13 files) once per item rather than letting them drift.
- **R10.** The GTID cap of 0x8000 conflicts with the "no limits" goal. Keep the native cap in exact mode (decision in `research/phase8-integration-backlog.md`) and lift it only with a versioned save and wire format.
