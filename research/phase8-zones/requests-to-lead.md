# Phase 8 zones/detection/fog - requests to the lead

Owner files: `crates/aa-zones/**`, `research/phase8-zones/**`. Nothing outside
them was edited. `aa-zones` compiles standalone with the locked deps (it uses
aa-numeric, serde, serde_json, thiserror; aa-trig/aa-spatial/aa-heightfield are
declared but unused - keep or drop, no lockfile change needed now).

## 1. Shared GMap solver state (integration decision)

GMap tile +0x24 (ZoneSolver cost / FIFO link), +0x28 visit stamp, +0x2D
arrival direction, the GMap+0x48 stamp counter and the process-wide busy byte
`0xB3D43D` are shared by every `0x4F4050`-family solver: aa-spatial water
floods (`water_areas.rs`), aa-navigation path/region searches, and aa-zones
(`flood::SolverState`, kept in `ZoneMap::solver`). The zone flood reads its
start tile's +0x24 left by any earlier solver. The integrated GMap should own
ONE store for these words (and one stamp counter / busy byte) and lend it to
each solver. Stamps and arrival directions are solve-local in aa-zones (exact);
+0x24 is the only cross-solve value.

## 2. Restoration unsupported group "Actor detection-memory refresh 4D67F0"

`0x4D67F0` is NOT the detection memory: it is the actor overlap-list refresh
already ported as `aa_spatial::overlaps::refresh_overlaps` (movement agent).
`aa-mapworld/src/actor_finish.rs:126` should call it (restoration owner); 370
records in `research/phase8-restoration/unsupported-inventory.json`. The real
detection memory refresh is `0x4EBC70` = `aa_zones::detection::refresh_memory`.

## 3. Bindings for other agents' seams (planning list)

| seam (crate) | native | aa-zones function |
|---|---|---|
| aa-mind `RoutWorld::zone` | 54EFC0 (kind 4 ZOC, 5 ZOG) | `query::tile_enemy_zone` |
| aa-mind `RoutWorld::friendly_zone`, recover 52BB40 | 54F8B0 | `query::tile_friendly_supply` |
| aa-mind bombard `region_54f3c0`, uai_bombard | 54F3C0 | `tile::visible_or_no_fog` |
| aa-mind element_move `destination_has_kingdom_object` | 54EF40 | `query::tile_has_zone` |
| aa-navigation `sai_state::TileKnowledge` | 54ECB0 / 54EC70 / 559530 | `tile::{explored, visible}`, `query::supply_bit` (semantics agree: low = explored, high = visible, word18 = supply) |
| aa-economy `supply.rs` zone seams | 515F40 / 514CE0 / 54F8B0 | `zone::{move_to_position, release}`, `query::tile_friendly_supply` |
| aa-combat organization "flare" | 515F40 (ZOC type 4) | `zone::move_to_position` |
| aa-combat begin_death zone release | 514CE0 | `zone::release` |
| aa-production construction / settlement | 54EFC0 (type 5) | `query::tile_enemy_zone` |
| aa-triggers `fog_restore_tiles` (542950) | 54F930 per tile of the 554DD0 iteration | `tile::restore_fog` |
| aa-triggers `actor_seen_by` (+0x70/+0x74, elements 99/100) | 4D3B80 / 4D3C50 | `detection::{visible_to, explored_by}` |
| AI (SAI) region counters | 7A99A0..7A9A90 | called through `tile::SaiHook` |
| UI zone overlay | 50C4C0 | `zone::ZoneObjectWorld::presentation` |
| GActor tile change (actors/runtime) 4D6350 | 4EC270, zone loop, 4CFEA0 -> 54EF10 | `detection::tile_changed`, `component::move_actor_zones`, `zone::actor_entered_tile` |
| Kingdom alliance change (metagame relations) | 4E0F90 | `tile::kingdom_masks` (kingdom +0x228/+0x22C) |
| Detection component (actor+0xB8, updatable +8) | 56F140 / 56EC90 / 56EA10 | `component::{tick, activate, set_zod_radius}` (schedule with the Updatable system) |
| Tile fog/actor callbacks (+0x64/+0x68/+0x6C) | 4EC010 / 4EC1B0 / 4EA0C0 | `detection::{reveal, hide, unexplore}` |

## 4. Storage needed in the actor/runtime world

* Actor zone vector actor+0x148 (9 slots; `aa_zones::zone::Zone` values),
  actor detection word +0x54, detection memory slots +0x58/+0x5C (records
  0x4EB9F0, 0x38 bytes), detection component (+0xB8: reveal mask +0x10, ZOD
  radius +0x14).
* GMap tile flags +0x20 and blocker GTIDs +0x1C for `flood::FloodWorld`.
* The tile actor list (+8) in head-first order for `tile::ZoneWorld::tile_actors`.

## 5. Services other owners should expose (no change requested now)

* aa-spatial: the shape/circle tile iteration 554DD0/554330 for the
  descriptors 4EA0F0 (actor shape), 4CF210 (occupancy) and 46E080+432730
  (circle) - `detection::ActorDetectionWorld::{shape_tiles, occupancy_tiles}`,
  `zone::EnterWorld::discovery_tiles`; also for the trigger fog pass 542950.
* combat/AI: 0x56EDC0 engagement sources (actor+0x118/+0x130 GTID lists,
  component +0xB0 +0x24 list, denizen threat list 0x569300, +0x70 object when
  SVar [B39AB0]+0x1C8) - `component::DetectionComponentWorld::reveal_sources`.
* property/modifier owner: 0x501C30 for `modified_detection_range`.
