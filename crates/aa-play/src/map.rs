//! The battle map: heights, passability and speed limits per cell, start
//! locations, the original placed things and the map's player slots, all from
//! the original RMP (aa-mapworld) and its extracted JSON (aa-assets).
//! Owner: core.
//!
//! Sources, in the original's terms:
//! - corner heights: the same conditional base + mountain + noise heights the
//!   renderer draws (`aa_assets::TerrainMap::native_height_at_grid` over the
//!   extracted map JSON), so units stand on the drawn ground;
//! - cell flags: TERRAIN tile records bound to their Variation/Terrain
//!   definitions like aa-runtime `battle_terrain.rs` (flags20 = saved flags,
//!   type << 4, `recompute_passability` land/water bits, 557D4B missing-ID
//!   fallback), plus the blocked outer ring the native map construction sets;
//! - speed caps: TerrainData+38 (`native_terrain_variation_speeds`);
//! - placed things, owners and kingdom/team slots: DYNAMIC_THINGS, KINGDOMS,
//!   TEAMS and MAP_HEADER via `aa_mapworld::WorldInventory::decode`.
use glam::Vec2;
use serde::{Deserialize, Serialize};

/// Cell flags, same bits as the original flags20 word.
pub const BLOCKED: u32 = 0x1;
pub const NOT_LAND_PASSABLE: u32 = 0x100;
pub const NOT_WATER_PASSABLE: u32 = 0x200;
/// Native terrain type in flags20 bits 4..6: 0 LAND, 1 MOUNTAIN, 2 WATER.
pub const TERRAIN_TYPE_SHIFT: u32 = 4;
pub const TERRAIN_TYPE_MASK: u32 = 0x70;
/// `speed_limit` value of a cell without a terrain speed cap.
pub const NO_SPEED_LIMIT: f32 = f32::MAX;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PlacedThing {
    pub thing_id: String,
    pub position: [f32; 3],
    pub yaw: f32,
    /// Map kingdom slot that owns it, if any.
    pub owner_slot: Option<u32>,
    // ---- appended by the core owner ----
    /// Serialized facing vector (native X/Y/Z). `yaw` is its angle about Z
    /// from native +X (the `Actor::facing` convention).
    #[serde(default)]
    pub facing: [f32; 3],
    /// GObject scale multiplier (1 when absent).
    #[serde(default)]
    pub scale: f32,
    /// Authored model variant (-1 = default aspect).
    #[serde(default)]
    pub variant: i32,
    /// Serialized GTID of the record (map-local identity).
    #[serde(default)]
    pub serialized_id: u32,
    /// Extra properties the map gives the actor (the random map generator's
    /// RMCActorGroup `properties`, e.g. tech_noupkeep on the start bunkers).
    #[serde(default)]
    pub properties: Vec<String>,
    /// The record's class in the map (class 2 Object, 3 Actor, 9 DynamicDoodad...),
    /// 0 for a generated thing.
    #[serde(default)]
    pub class_code: u32,
    /// A Breakable's words +18 (state), +1C, +20, +24, +28, +2C after its
    /// saved Load 0x58FD60 (which zeroes them on a map with GWorld+48 = 0).
    #[serde(default)]
    pub breakable: Option<[u32; 6]>,
    /// The record's Load fails and the thing never exists: its saved actor holds a
    /// component its definition no longer has (0x4D8290 "Component %d removed from
    /// actor" -> 0x858A00; slots 0, 4 and 17 are excused), the catch of 0x4FAE80
    /// destroys it. On (2v2)Comrades the village blocks saved with a Supply component.
    #[serde(default)]
    pub load_fails: bool,
    /// A placed organization's saved state (57D130): its members are records of their
    /// own inside it, with saved ids (a herd of ambient animals on (2v2)Kursk).
    #[serde(default)]
    pub organization: Option<PlacedOrganization>,
    /// This record is a member of the organization at this index, not a top-level
    /// record of the map: it appends after them in `GameMap::placed`.
    #[serde(default)]
    pub member_of: Option<usize>,
    /// The saved Mind's bytes (slot 4), for `aa_mind::saved_load`.
    #[serde(default)]
    pub mind: Option<Vec<u8>>,
    /// The saved components a placed organization or its member holds beyond the
    /// Mind: the Mover, the Character and the rest, in saved order (0x4D6F10's
    /// restore of an actor that was alive when the map was saved).
    #[serde(default)]
    pub components: Vec<(u8, aa_mapworld::saved_components::SavedComponent)>,
    /// Actor +0xE4 as saved: the slot of the active component (0x4D8FA0), -1 for none.
    #[serde(default)]
    pub current_component: Option<i32>,
}

/// What 57D130 saves of a placed organization, as far as a world needs it.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct PlacedOrganization {
    /// (saved layout slot, `GameMap::placed` index of the member's record).
    pub members: Vec<(i32, usize)>,
    /// The leader's slot; negative for none.
    pub leader: i32,
    /// 59E300: morale as a fraction of the maximum.
    pub morale_bits: u32,
    /// The formation's index in the layout; negative for none.
    pub formation: i32,
    /// +BC, +C0: where the banner goes.
    pub position_bits: Option<[u32; 2]>,
    /// +78: each slot's formation offset, as saved. The Load's state change 0x57BEF0
    /// keeps a member's and scatters the rest.
    #[serde(default)]
    pub offsets: Vec<[u32; 2]>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct MapSlot {
    pub nation: Option<String>,
    pub team: u32,
    pub color: Option<u32>,
    // ---- appended by the core owner ----
    /// Header kingdom id, e.g. "kingdom_0", "independent_2".
    #[serde(default)]
    pub id: String,
    /// Independent (neutral) kingdom: owns cities, farms and resources.
    #[serde(default)]
    pub independent: bool,
    /// Header colour id ("blue", "red"...).
    #[serde(default)]
    pub color_id: String,
    /// Header team id ("team_0"); empty for independents.
    #[serde(default)]
    pub team_id: String,
    /// Header faction (general) id, when the map fixes one.
    #[serde(default)]
    pub faction: Option<String>,
    /// KINGDOMS payload +0x188 kingdom index (relation table column).
    #[serde(default)]
    pub kingdom_index: Option<u32>,
    /// Saved relation of this kingdom to every kingdom index (+0x1D0) and to
    /// every team (+0x1DC): native 0 self, 1 friend, 2 neutral, 3 enemy
    /// (4E52D0 relation, 4E5250/4E5280/4E52A0 friend/neutral/enemy tests).
    #[serde(default)]
    pub relations_to_kingdoms: Vec<u32>,
    #[serde(default)]
    pub relations_to_teams: Vec<u32>,
    /// Saved GTID of the KINGDOMS record.
    #[serde(default)]
    pub serialized_id: Option<u32>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GameMap {
    pub id: String,
    pub name: String,
    pub width: u32,
    pub height: u32,
    pub cell_size: f32,
    /// (width+1)*(height+1) corner heights, row-major.
    pub heights: Vec<f32>,
    /// width*height cell flags (BLOCKED, NOT_LAND_PASSABLE, ...).
    pub flags: Vec<u32>,
    /// width*height land speed limits (m/s) from the terrain variations.
    pub speed_limit: Vec<f32>,
    pub start_locations: Vec<Vec2>,
    pub placed: Vec<PlacedThing>,
    pub slots: Vec<MapSlot>,
    // ---- appended by the core owner ----
    /// `slots` index of each start location (`start_locations[i]` belongs to
    /// `slots[player_slots[i]]`).
    #[serde(default)]
    pub player_slots: Vec<usize>,
    /// Header team ids in slot order ("team_0", "team_1").
    #[serde(default)]
    pub teams: Vec<String>,
    /// Native world extents in metres (MAP_HEADER creator m_width/m_height).
    #[serde(default)]
    pub world_extents: [f32; 2],
    #[serde(default)]
    pub biome: String,
    #[serde(default)]
    pub weather: String,
    /// Header name/description string keys (MAPSTRINGS).
    #[serde(default)]
    pub name_key: String,
    #[serde(default)]
    pub description_key: String,
    /// Effective TerrainVariation id of every cell (index into `variations`).
    #[serde(default)]
    pub cell_variation: Vec<u16>,
    #[serde(default)]
    pub variations: Vec<String>,
    /// Problems met while loading (unresolved terrain ids, missing starts...).
    #[serde(default)]
    pub load_notes: Vec<String>,
    /// The world random stream (GWorld+0xD4) as map creation leaves it, and
    /// the state every GetRNG 0x4F02B0 call logged on the way (the
    /// CreateGWorld getter first). A generated map: as the random map
    /// generator left it. A loaded map: the GWORLD chunk's saved state, which
    /// the map load 0x4FC600 reads over the seeded one (with the saved clock,
    /// tick 0 and step 1/16 on every stock map). The world continues from it.
    #[serde(default)]
    pub rng_state: Option<u32>,
    #[serde(default)]
    pub rng_log: Vec<u32>,
    /// The GTID manager state the map was saved with (IDMGR chunk, 0x4F0230);
    /// `None` for a generated map. See `crate::ids`.
    #[serde(default)]
    pub id_manager: Option<IdManager>,
    /// Saved GTID of each TEAMS record, in team slot order.
    #[serde(default)]
    pub team_ids: Vec<u32>,
    /// The saved periodic queue 0 (GWorld+0x1AC, UPDATABLES chunk, read by
    /// the queue Load 0x50B8B0): the `slots` index of every kingdom whose
    /// Updatable is on it, in list order (head first = delivered first).
    /// `None` when the map saves no periodic queues (world version < 22, or
    /// a generated map). See `crate::scheduler`.
    #[serde(default)]
    pub queue0_kingdoms: Option<Vec<usize>>,
    /// A world the random map generator made for a world creator: what its
    /// draws left on the queues, for `crate::scheduler::replay_generated`. Such a
    /// map's things are already made: no InitExisting, queue Load or PostLoad.
    #[serde(default)]
    pub generated: Option<GeneratedWorld>,
    /// The saved periodic queues 0, 1, 2 (GWorld+1AC/+1C0/+1D4), owners
    /// resolved; `None` as for `queue0_kingdoms`. See `crate::scheduler`.
    #[serde(default)]
    pub saved_queues: Option<Vec<SavedPeriodic>>,
    /// The saved deferred queues 3 and 4 (GWorld+1E8 / +208, UPDATABLES, read
    /// by 0x50BFF0): their rings replace the world's at the queue Load, so
    /// the events InitExisting registered do not survive. `None` when the
    /// chunk is missing or unreadable. See `crate::scheduler`.
    #[serde(default)]
    pub saved_deferred: Option<Vec<SavedPeriodic>>,
    /// The map's height layers as the simulation reads them, exact binary32
    /// words of HEIGHT_MAP and MOUNTAIN_HEIGHT_MAP. `None` (a generated or test
    /// map) makes `height_at` read `heights`.
    #[serde(default)]
    pub native_terrain: Option<Box<NativeTerrain>>,
    /// The SAI regions the map saved in its TERRAIN suffix (`0x7AA9B0`), which the
    /// route solver reads. `None` for a generated or test map and below world
    /// version 17: those run the static analysis `0x7ACB00`.
    #[serde(skip)]
    pub saved_topology: Option<std::sync::Arc<aa_navigation::topology::Topology>>,
}

/// GWorld+34 / +38 and the tile grid, what the ground query 0x4F0E50 reads
/// (`aa_spatial::query_actor_height` with `aa_heightfield` samplers). Kept in
/// plain words so `height_at` can run for every actor and every drawn thing:
/// `ground` is that port at PC53 in binary64, which gives the same bits because
/// every x87 operation of these leaves rounds to 53 bits and no value reaches
/// the binary64 subnormal range. `tests/heights_native.rs` holds them equal on
/// the shipped maps.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct NativeTerrain {
    /// Sample grid (0x5519A0): width, height, coordinate shift.
    pub width: i32,
    pub height: i32,
    pub shift: u32,
    /// Sample spacing, `1 << shift` metres.
    pub cell: f32,
    pub base: Vec<f32>,
    pub overlay: Vec<f32>,
    /// Tile grid: width, height, coordinate shift.
    pub tile_width: i32,
    pub tile_height: i32,
    pub tile_shift: u32,
    /// Tile terrain +1C: 1 mountain, 2 water, else 0.
    pub overlay_kind: Vec<u8>,
    /// Tile terrain +68 / +6C noise of a mountain tile.
    pub noise: Vec<[f32; 2]>,
}

impl NativeTerrain {
    /// 0x550190 nearest.
    fn nearest(&self, field: &[f32], x: f32, y: f32) -> f32 {
        let bias = 1i32 << (self.shift - 1);
        let ix = ((x as i32).wrapping_add(bias) >> self.shift).clamp(0, self.width - 1);
        let iy = ((y as i32).wrapping_add(bias) >> self.shift).clamp(0, self.height - 1);
        field[(iy * self.width + ix) as usize]
    }

    /// 0x550210, its unstored result.
    fn interpolated(&self, field: &[f32], x: f32, y: f32) -> f64 {
        let (xd, yd, cell) = (f64::from(x), f64::from(y), f64::from(self.cell));
        // FPREM: exact, the sign of the dividend. Only X is stored binary32.
        let rx = (xd % cell) as f32;
        let ry = yd % cell;
        let epsilon = f64::from(f32::from_bits(0x38d1_b717));
        let half = f64::from((cell * 0.5) as f32);
        if f64::from(rx) < epsilon {
            if ry < epsilon {
                return f64::from(self.nearest(field, x, y));
            }
            let first = self.nearest(field, x, (yd - half) as f32);
            let second = self.nearest(field, x, (half + yd) as f32);
            return (f64::from(second) + f64::from(first)) * 0.5;
        }
        if ry < epsilon {
            let first = self.nearest(field, (half + xd) as f32, y);
            let second = self.nearest(field, (xd - half) as f32, y);
            return (f64::from(second) + f64::from(first)) * 0.5;
        }
        let reciprocal = 1.0 / cell;
        let tx = f64::from((f64::from(rx) * reciprocal) as f32);
        let ty = f64::from((reciprocal * ry) as f32);
        // This branch does not add the nearest sampler's index bias.
        let ix = ((x as i32) >> self.shift).clamp(0, self.width - 1);
        let iy = ((y as i32) >> self.shift).clamp(0, self.height - 1);
        let at = |i: i32, j: i32| f64::from(field[(j * self.width + i) as usize]);
        let h00 = at(ix, iy);
        let h10 = if ix + 1 < self.width { at(ix + 1, iy) } else { h00 };
        let h01 = if iy + 1 < self.height { at(ix, iy + 1) } else { h00 };
        // The original repeats h00 even when one other neighbour is valid.
        let h11 = if ix + 1 < self.width && iy + 1 < self.height { at(ix + 1, iy + 1) } else { h00 };
        let w11 = ty * tx;
        let minus_x = 1.0 - tx;
        let w01 = f64::from((ty * minus_x) as f32);
        let minus_y = 1.0 - ty;
        let w10 = f64::from((tx * minus_y) as f32);
        let sum = minus_y * minus_x * h00 + w10 * h10 + w01 * h01;
        sum + w11 * h11
    }

    /// 0x550210 on the base layer alone: what the camera's target stands on (0x6FF288
    /// loads the world's HEIGHT_MAP and calls it, no overlay, no terrain).
    pub fn base_height(&self, x: f32, y: f32) -> f32 {
        self.interpolated(&self.base, x, y) as f32
    }

    /// 0x4F0E50 at `(x, y)` with the flags of a ground thing, 0x122 (0x4EEB40):
    /// the base layer, stored; the overlay when the tile or one of its eight
    /// neighbours is mountain or water; a mountain tile's noise offset.
    pub fn ground(&self, x: f32, y: f32) -> f32 {
        const NEIGHBOURS: [[i32; 2]; 8] = [[-1, -1], [-1, 0], [-1, 1], [0, 1], [1, 1], [1, 0], [1, -1], [0, -1]];
        let base = self.interpolated(&self.base, x, y) as f32;
        let tx = ((x as i32) >> self.tile_shift).clamp(0, self.tile_width - 1);
        let ty = ((y as i32) >> self.tile_shift).clamp(0, self.tile_height - 1);
        let tile = (ty * self.tile_width + tx) as usize;
        let own = self.overlay_kind[tile];
        let selected = own == 1
            || own == 2
            || NEIGHBOURS.iter().any(|[dx, dy]| {
                let (nx, ny) = (tx + dx, ty + dy);
                nx >= 0 && ny >= 0 && nx < self.tile_width && ny < self.tile_height && matches!(self.overlay_kind[(ny * self.tile_width + nx) as usize], 1 | 2)
            });
        let mut overlay = 0.0f64;
        if selected {
            overlay = self.interpolated(&self.overlay, x, y);
            if own == 1 {
                let remainder = (ty.wrapping_mul(self.tile_width).wrapping_add(tx).wrapping_mul(0x1273).wrapping_add(0x18b3) % 0x1b55) % 1000;
                let [min, max] = self.noise[tile].map(f64::from);
                let offset = (max - min) * f64::from(remainder) * f64::from(f32::from_bits(0x3a83_126f)) + min;
                overlay += offset;
            }
        }
        (overlay + f64::from(base)) as f32
    }
}

/// aa-rules COMPONENTS slot of the Properties component.
pub const PROPERTIES_SLOT: u8 = 6;

/// The owner of a saved queue entry: the UPDATABLES table id resolved
/// through the saved Updatable ids of the kingdoms and the placed things'
/// components.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum SavedOwner {
    /// Component `slot` (aa-rules COMPONENTS) of `placed[index]`.
    Placed { index: usize, slot: u8 },
    /// DynamicDoodad component of `placed[index]`: slot 0 Disturbable, slot 1
    /// Breakable (their own factory, not COMPONENTS slots; 0x4DCD10).
    Doodad { index: usize, slot: u8 },
    /// The economy Updatable (+180) of the kingdom of `slots[slot]`.
    Kingdom { slot: usize },
    /// Component `component` of the kingdom of `slots[slot]`.
    KingdomComponent { slot: usize, component: u8 },
    /// A table id aa-play does not resolve (negative: the native null id).
    Unknown(i32),
}

/// What the random map generator's draws left in its world, in draw order.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct GeneratedWorld {
    pub events: Vec<GeneratedEvent>,
    /// `placed` indices (city, block): 0x5642A0 in the city's layout (the City's
    /// PostCreate 0x564BE0 -> 0x5644F0 with its list), in creation order. City+10
    /// holds them head first and they change hands with the city.
    pub blocks: Vec<(usize, usize)>,
    /// `placed` indices (truck, trailer): the trailer the truck's InitCreate made
    /// (0x4D70F2), a GActor of its own natively and `Thing::Trailer` in aa-play.
    pub trailers: Vec<(usize, usize)>,
}

/// One such draw: the stream state before it and what it did.
#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub struct GeneratedEvent {
    pub state: u32,
    pub what: GeneratedWhat,
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum GeneratedWhat {
    /// 0x50AC60 / 0x50AC00 -> 0x50B2C0: a registration on periodic queue `queue`.
    Periodic { queue: u8, owner: SavedOwner },
    /// The InitCreate or the InitExisting of `placed[index]`'s DynamicDoodad
    /// components (0x5904A0, 0x58F960).
    Doodad { index: usize },
    /// 0x567840: `placed[intruder]` entered the zone of control of `placed[defender]`.
    Defender { defender: usize, intruder: usize },
}

/// One saved periodic queue: its current bucket and every bucket's entries
/// (owner, data) in list order.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct SavedPeriodic {
    pub current: usize,
    pub buckets: Vec<Vec<(SavedOwner, u32)>>,
}

/// IDMGR: the next-fit cursor, the highest used slot and the generation of
/// every slot below it (exactly `highest_slot` of them, as 0x4F0230 reads).
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct IdManager {
    pub next_slot: u32,
    pub highest_slot: u32,
    pub generations: Vec<u16>,
}

impl GameMap {
    /// Implemented by the core owner.
    pub fn load(game_root: &std::path::Path, map_id: &str) -> anyhow::Result<Self> {
        let data = crate::data::GameData::load(game_root)?;
        load(game_root, map_id, &data)
    }
    pub fn cell_of(&self, p: Vec2) -> Option<(u32, u32)> {
        let (x, y) = ((p.x / self.cell_size).floor(), (p.y / self.cell_size).floor());
        (x >= 0.0 && y >= 0.0 && (x as u32) < self.width && (y as u32) < self.height).then_some((x as u32, y as u32))
    }
    pub fn cell_flags(&self, p: Vec2) -> u32 {
        self.cell_of(p).map_or(BLOCKED, |(x, y)| self.flags[(y * self.width + x) as usize])
    }
    /// Ground height at a native ground point: the query 0x4F0E50 on a map
    /// with its height layers, else bilinear over the corner heights.
    pub fn height_at(&self, p: Vec2) -> f32 {
        if let Some(n) = &self.native_terrain {
            return n.ground(p.x, p.y);
        }
        if self.heights.is_empty() {
            return 0.0;
        }
        let gx = (p.x / self.cell_size).clamp(0.0, self.width as f32);
        let gy = (p.y / self.cell_size).clamp(0.0, self.height as f32);
        let (x0, y0) = (gx.floor() as u32, gy.floor() as u32);
        let (x1, y1) = ((x0 + 1).min(self.width), (y0 + 1).min(self.height));
        let (tx, ty) = (gx - x0 as f32, gy - y0 as f32);
        let h = |x: u32, y: u32| self.heights[(y * (self.width + 1) + x) as usize];
        (h(x0, y0) * (1.0 - tx) + h(x1, y0) * tx) * (1.0 - ty) + (h(x0, y1) * (1.0 - tx) + h(x1, y1) * tx) * ty
    }
    // ---- appended by the core owner ----
    /// Land speed cap of the cell under `p` (NO_SPEED_LIMIT when uncapped).
    pub fn speed_limit_at(&self, p: Vec2) -> f32 {
        self.cell_of(p).map_or(0.0, |(x, y)| self.speed_limit[(y * self.width + x) as usize])
    }
    /// Native terrain type of the cell under `p` (0 land, 1 mountain, 2 water).
    pub fn terrain_type(&self, p: Vec2) -> u32 {
        (self.cell_flags(p) & TERRAIN_TYPE_MASK) >> TERRAIN_TYPE_SHIFT
    }
    /// Map kingdom slots that start locations belong to.
    pub fn player_count(&self) -> usize {
        self.start_locations.len()
    }
    /// Build a map from already-loaded data (tests and tools).
    pub fn load_with(game_root: &std::path::Path, map_id: &str, data: &crate::data::GameData) -> anyhow::Result<Self> {
        load(game_root, map_id, data)
    }
    /// A map the extracted set does not hold: `bytes` is the RMP itself (a map of
    /// a profile's data, read the way the original reads any map, 0x4FC600), and
    /// `name` the text of its `map_name` when the caller has it. Heights come from
    /// its own height layers (0x4F0E50 at the corners).
    pub fn load_file(map_id: &str, bytes: Vec<u8>, data: &crate::data::GameData, name: Option<String>) -> anyhow::Result<Self> {
        assemble(map_id, bytes, data, None, name)
    }
}

/// Terrain definition tables (Terrain/Variation/Biome registries) the map
/// loader binds tile palettes against; built once with `GameData`.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TerrainTables {
    /// Lower-case Variation id -> definition.
    pub variations: std::collections::BTreeMap<String, TerrainVariationDef>,
    /// Lower-case Biome id -> its initial Variation (first Variation of its
    /// first Terrain, 557D4B fallback start).
    pub biome_initial: std::collections::BTreeMap<String, String>,
    /// Data/translate.tgi `node = Variation` renames (lower-case old -> new),
    /// applied first like the native 48D500 Find.
    #[serde(default)]
    pub translations: std::collections::BTreeMap<String, String>,
    /// Lower-case Biome id -> its texture_subfolder_name (Biome+98, e.g.
    /// "Arctic", "Desert", "Tropical"); biomes without one are absent.
    #[serde(default)]
    pub biome_texture_subfolder: std::collections::BTreeMap<String, String>,
}
impl TerrainTables {
    /// The texture subfolder the original selects for a map of `biome`
    /// (43F0EE copies it to the resource manager at world init; every
    /// texture lookup 5A77E0 then tries `<folder>/<subfolder>/<file>` first).
    pub fn texture_subfolder(&self, biome: &str) -> Option<&str> {
        self.biome_texture_subfolder.get(&biome.to_ascii_lowercase()).map(String::as_str)
    }
    /// 48D500: the translation first (its target only), else the id itself.
    pub fn find(&self, id: &str) -> Option<&TerrainVariationDef> {
        let key = id.to_ascii_lowercase();
        match self.translations.get(&key) {
            Some(new) => self.variations.get(new).or_else(|| self.translations.get(new).and_then(|n| self.variations.get(n))),
            None => self.variations.get(&key),
        }
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct TerrainVariationDef {
    pub id: String,
    pub terrain: String,
    pub native_type: u32,
    pub land: bool,
    pub water: bool,
    /// TerrainData+38 cap in m/s, NO_SPEED_LIMIT when the terrain has none.
    pub speed_limit: f32,
    /// TerrainData +68 / +6C noise_min, noise_max of a mountain terrain.
    #[serde(default)]
    pub noise: [f32; 2],
    /// The terrain's `[Property]` ids: what `0x4D4130` adds to a thing on the tile.
    #[serde(default)]
    pub properties: Vec<String>,
}

pub(crate) fn terrain_tables(catalog: &aa_rules::actors::ActorCatalog, notes: &mut Vec<String>) -> TerrainTables {
    use crate::data::loader::{PC, bl, fx, st};
    let mut out = TerrainTables::default();
    let speeds = catalog.native_terrain_variation_speeds(PC).unwrap_or_else(|e| {
        notes.push(format!("terrain speeds: {e}"));
        Vec::new()
    });
    let mut first_variation = std::collections::BTreeMap::new();
    if let Ok(terrains) = catalog.resolved_definitions("Terrain") {
        for terrain in terrains {
            let id = st(&terrain, "IDS").unwrap_or_default();
            let native_type = match st(&terrain, "type").unwrap_or_default().to_ascii_uppercase().as_str() {
                "MOUNTAIN" => 1,
                "WATER" => 2,
                _ => 0,
            };
            let land = bl(&terrain, "land").unwrap_or(native_type == 0);
            let water = bl(&terrain, "water").unwrap_or(native_type == 2);
            let properties: Vec<String> = terrain
                .children("Property")
                .unwrap_or_default()
                .iter()
                .filter_map(|p| st(p, "property_IDS"))
                .collect();
            for variation in terrain.children("Variation").unwrap_or_default() {
                let Some(vid) = st(&variation, "IDS") else { continue };
                first_variation.entry(id.to_ascii_lowercase()).or_insert_with(|| vid.clone());
                let speed = speeds
                    .iter()
                    .find(|s| s.variation_id.eq_ignore_ascii_case(&vid))
                    .map(|s| f32::from_bits(s.speed_limit_bits))
                    .filter(|v| *v > 0.0)
                    .unwrap_or(NO_SPEED_LIMIT);
                out.variations.entry(vid.to_ascii_lowercase()).or_insert(TerrainVariationDef {
                    id: vid,
                    terrain: id.clone(),
                    native_type,
                    land,
                    water,
                    speed_limit: speed,
                    properties: properties.clone(),
                    noise: if native_type == 1 { [fx(&terrain, "noise_min").unwrap_or(0.0), fx(&terrain, "noise_max").unwrap_or(0.0)] } else { [0.0, 0.0] },
                });
            }
        }
    }
    if let Ok(translations) = catalog.resolved_definitions("Translate") {
        for t in translations {
            if !st(&t, "node").is_some_and(|n| n.eq_ignore_ascii_case("Variation")) {
                continue;
            }
            if let (Some(old), Some(new)) = (st(&t, "ids_old"), st(&t, "ids_new")) {
                out.translations.entry(old.to_ascii_lowercase()).or_insert(new.to_ascii_lowercase());
            }
        }
    }
    if let Ok(biomes) = catalog.resolved_definitions("Biome") {
        for biome in biomes {
            let Some(id) = st(&biome, "IDS") else { continue };
            if let Some(sub) = st(&biome, "texture_subfolder_name").filter(|s| !s.is_empty() && !s.eq_ignore_ascii_case("null")) {
                out.biome_texture_subfolder.insert(id.to_ascii_lowercase(), sub);
            }
            let first = biome.children("Terrain").ok().and_then(|v| v.into_iter().next());
            if let Some(t) = first.and_then(|t| st(&t, "terrain_IDS"))
                && let Some(v) = first_variation.get(&t.to_ascii_lowercase())
            {
                out.biome_initial.insert(id.to_ascii_lowercase(), v.clone());
            }
        }
    }
    out
}

fn utf16(v: &[u16]) -> String {
    String::from_utf16_lossy(v)
}

pub fn read_rmp(game_root: &std::path::Path, map_id: &str) -> anyhow::Result<Vec<u8>> {
    // The map files are not part of the v14 overlay, so the extracted copy
    // of the Data.rwd member is the original file; fall back to the archive.
    let extracted = game_root.join("reconstruction").join("assets").join("original").join("Data").join("Maps");
    for ext in ["RMP", "rmp"] {
        let p = extracted.join(format!("{map_id}.{ext}"));
        if let Ok(bytes) = std::fs::read(&p) {
            return Ok(bytes);
        }
    }
    let mut sources = crate::data::Sources::open(game_root)?;
    for ext in ["RMP", "rmp"] {
        if let Ok(file) = sources.files.read(&format!("Maps/{map_id}.{ext}")) {
            return Ok(file.bytes);
        }
    }
    anyhow::bail!("map {map_id} not found in the original data")
}

/// What the extracted map JSON gives a stock map: the drawn grid and its corner
/// heights.
struct Drawn {
    width: u32,
    height: u32,
    cell_size: f32,
    heights: Vec<f32>,
    name: String,
}

fn load(game_root: &std::path::Path, map_id: &str, data: &crate::data::GameData) -> anyhow::Result<GameMap> {
    // Heights exactly as the renderer's terrain (aa-assets TerrainMap).
    let json = game_root.join("reconstruction").join("game").join("data").join("maps").join(format!("{map_id}.json"));
    let terrain: aa_assets::TerrainMap = serde_json::from_slice(
        &std::fs::read(&json).map_err(|e| anyhow::anyhow!("reading {}: {e}", json.display()))?,
    )
    .map_err(|e| anyhow::anyhow!("parsing {}: {e}", json.display()))?;
    terrain.validate()?;
    let metadata = aa_assets::TerrainMetadata::bundled()?;
    let (w, h) = (terrain.width, terrain.height);
    let mut heights = Vec::with_capacity(((w + 1) * (h + 1)) as usize);
    for y in 0..=h {
        for x in 0..=w {
            heights.push(terrain.native_height_at_grid(x, y, &metadata).map_or(0.0, |v| v.0));
        }
    }
    let name = std::fs::read(game_root.join("reconstruction").join("game").join("data").join("maps").join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<aa_assets::MapInfo>>(&b).ok())
        .and_then(|maps| maps.into_iter().find(|m| m.id == map_id).map(|m| m.name))
        .unwrap_or_else(|| terrain.name.clone());
    let cell_size = terrain.cell_size;
    assemble(map_id, read_rmp(game_root, map_id)?, data, Some(Drawn { width: w, height: h, cell_size, heights, name }), None)
}

/// The map from the original file; a stock map's drawn grid is held against it.
fn assemble(map_id: &str, bytes: Vec<u8>, data: &crate::data::GameData, drawn: Option<Drawn>, name: Option<String>) -> anyhow::Result<GameMap> {
    let mut notes = Vec::new();
    // The original RMP.
    let archive = aa_mapworld::MapArchive::from_bytes(bytes)?;
    let inventory = aa_mapworld::WorldInventory::decode(&archive, 1 << 30)?;
    // A saved game's things are read whole (`crate::saved_world`).
    let saved_things = inventory.is_saved_game().then(|| crate::saved_world::SavedThings::read(&inventory));
    let saved = &inventory.world;
    if saved_things.is_none() && (saved.word_dc != 0 || saved.begin_time_bits != 0 || saved.word_e4 != 0 || saved.delta_bits != 0.0625f32.to_bits()) {
        notes.push(format!(
            "saved GWORLD clock (tick {}, begin {:#x}, end {:#x}, step {:#x}) is not the stock clock and is not applied",
            saved.word_dc, saved.begin_time_bits, saved.word_e4, saved.delta_bits
        ));
    }
    let header = inventory
        .header
        .as_ref()
        .ok_or_else(|| anyhow::anyhow!("{map_id}: MAP_HEADER {}", inventory.header_error.clone().unwrap_or_default()))?;
    let creator = &header.creator.parameters;
    let teams: Vec<String> = creator.teams.iter().map(|t| utf16(&t.id)).collect();
    let mut slots = Vec::new();
    for k in &creator.kingdoms {
        let team_id = utf16(&k.string_14);
        let color_id = utf16(&k.color);
        let nation = utf16(&k.nation);
        let faction = utf16(&k.faction);
        slots.push(MapSlot {
            nation: (!nation.is_empty()).then(|| nation.trim_start_matches("nation_").to_ascii_lowercase()),
            team: teams.iter().position(|t| t.eq_ignore_ascii_case(&team_id)).map_or(u32::MAX, |i| i as u32),
            color: data.kingdom_colors.iter().position(|(c, _)| c.eq_ignore_ascii_case(&color_id)).map(|i| i as u32),
            id: utf16(&k.id),
            independent: k.byte_10 != 0,
            color_id,
            team_id,
            faction: (!faction.is_empty()).then_some(faction),
            kingdom_index: None,
            relations_to_kingdoms: Vec::new(),
            relations_to_teams: Vec::new(),
            serialized_id: None,
        });
    }
    // Saved Updatable id (UPDATABLES table index) -> its owner: each
    // kingdom's economy Updatable and its Properties component, then every
    // saved component of the placed things.
    let mut updatable_owners: std::collections::BTreeMap<i32, SavedOwner> = std::collections::BTreeMap::new();
    if let Some(k) = &inventory.kingdoms {
        for r in k.slots.iter().filter_map(|s| s.record.as_ref()) {
            let name = utf16(&r.thing.name_utf16);
            if let Some(i) = slots.iter().position(|s| s.id.eq_ignore_ascii_case(&name)) {
                if let Some(id) = r.updatable.as_ref().and_then(|u| u.serialized_id).filter(|&id| id >= 0) {
                    updatable_owners.insert(id, SavedOwner::Kingdom { slot: i });
                }
                let properties = r.actor.as_ref().and_then(|a| a.tail.properties.as_ref()).and_then(|p| p.updatable.serialized_id);
                if let Some(id) = properties.filter(|&id| id >= 0) {
                    updatable_owners.insert(id, SavedOwner::KingdomComponent { slot: i, component: PROPERTIES_SLOT });
                }
            }
            if let Some(slot) = slots.iter_mut().find(|s| s.id.eq_ignore_ascii_case(&name)) {
                slot.serialized_id = Some(r.serialized_id);
                slot.kingdom_index = Some(r.payload.word_188);
                slot.relations_to_kingdoms = r.payload.words_1d0.values.clone();
                slot.relations_to_teams = r.payload.words_1dc.as_ref().map(|v| v.values.clone()).unwrap_or_default();
            }
        }
    }
    // KINGDOMS chunk slot -> header slot by kingdom name.
    let kingdom_names: Vec<String> = inventory
        .kingdoms
        .as_ref()
        .map(|k| k.slots.iter().map(|s| s.record.as_ref().map(|r| utf16(&r.thing.name_utf16)).unwrap_or_default()).collect())
        .unwrap_or_default();
    let owner_of = |source_slot: usize| -> Option<u32> {
        let name = kingdom_names.get(source_slot)?;
        slots.iter().position(|s| s.id.eq_ignore_ascii_case(name)).map(|i| i as u32)
    };
    let owners: std::collections::BTreeMap<usize, Option<u32>> = inventory
        .ownership
        .as_ref()
        .map(|o| {
            o.references
                .iter()
                .map(|r| {
                    let slot = match &r.target {
                        aa_mapworld::ownership::OwnerTarget::Kingdom { source_slot, .. } => owner_of(*source_slot),
                        _ => None,
                    };
                    (r.dynamic_record_ordinal, slot)
                })
                .collect()
        })
        .unwrap_or_default();
    let mut placed = Vec::with_capacity(inventory.dynamic_things.placements.len());
    let mut component_errors = 0usize;
    let mut placed_organizations = Vec::new();
    for (index, p) in inventory.dynamic_things.placements.iter().enumerate() {
        // The saved components' Updatable ids (component Load order, the
        // actor tail of 0x4D6F10's restore; aa-mapworld component codecs).
        let mut load_fails = false;
        let components = data.find(&utf16(&p.thing_id_utf16)).map(|d| data.def(d).components);
        if let Some(actor) = p.actor.as_ref().filter(|_| saved_things.is_none()) {
            let walked = aa_mapworld::actor_payload::ActorTail::decode_prefix_with_stages(&p.opaque_tail, actor.version, 0, |slot, saved, _, _, bytes| {
                if !saved {
                    return Ok((0, None));
                }
                // 0x4D8290's test, slot by slot.
                if !matches!(slot, 0 | 4 | 17) && components.is_some_and(|c| c & (1 << slot) == 0) {
                    load_fails = true;
                }
                let (payload, consumed) = aa_mapworld::saved_components::SavedComponent::decode_prefix(slot, bytes)?;
                if let Some(id) = payload.updatable().and_then(|u| u.serialized_id).filter(|&id| id >= 0) {
                    updatable_owners.insert(id, SavedOwner::Placed { index, slot });
                }
                let properties = if let aa_mapworld::saved_components::SavedComponent::Properties(v) = &payload { Some(v.clone()) } else { None };
                Ok((consumed, properties))
            });
            if walked.is_err() {
                // An organization (slot 7) holds its members' whole records, which
                // the walker reads with what it needs of the definitions.
                match aa_mapworld::saved_world::walk(p.clone(), inventory.world.version, 0, &mut crate::saved_world::Minds) {
                    // Also a thing on its own with a Mind (slot 4), which the prefix
                    // walk above does not read.
                    Ok(thing) if thing.partial.is_none() => placed_organizations.push((index, thing)),
                    _ => component_errors += 1,
                }
            }
        }
        // A DynamicDoodad's two-slot Load 0x4DCD10: the components' Updatable
        // ids, and a Breakable's restored words.
        let mut breakable = None;
        if p.actor.is_none()
            && let Some(doodad) = data.find(&utf16(&p.thing_id_utf16)).map(|d| data.def(d)).filter(|d| d.runtime_class == p.class_code).and_then(|d| d.doodad.as_ref())
        {
            let definition = aa_mapworld::dynamic_doodad::Definition { owner_id: utf16(&p.thing_id_utf16), component_mask: doodad.component_mask };
            let mut registrations = Vec::new();
            let mut state = aa_mapworld::dynamic_doodad::DynamicDoodad::default();
            match state.load(&p.opaque_tail, &definition, index as u32, inventory.world.word_48, |_, _| Ok(()), &mut registrations) {
                Ok(_) => {
                    for r in registrations {
                        if let aa_mapworld::restore_components::UpdatableTarget::DynamicDoodad { slot, .. } = r.target {
                            updatable_owners.insert(r.serialized_id, SavedOwner::Doodad { index, slot });
                        }
                    }
                    breakable = state.components[1].as_ref().and_then(|c| <[u32; 6]>::try_from(c.words.as_slice()).ok());
                }
                Err(_) => component_errors += 1,
            }
        }
        let position = p.thing.position_bits.map(f32::from_bits);
        let facing = p.thing.facing_bits.map(f32::from_bits);
        let yaw = if facing[0] != 0.0 || facing[1] != 0.0 { facing[1].atan2(facing[0]) } else { -std::f32::consts::FRAC_PI_2 };
        let scale = p.object.as_ref().map(|o| f32::from_bits(o.scale_multiplier_bits)).filter(|s| s.is_finite() && *s > 0.0).unwrap_or(1.0);
        placed.push(PlacedThing {
            thing_id: utf16(&p.thing_id_utf16),
            position,
            yaw,
            owner_slot: owners.get(&p.record_ordinal).copied().flatten(),
            facing,
            scale,
            variant: p.object.as_ref().and_then(|o| o.variant_index).unwrap_or(-1),
            serialized_id: p.serialized_id,
            properties: Vec::new(),
            class_code: p.class_code,
            breakable,
            load_fails,
            organization: None,
            member_of: None,
            mind: None,
            components: Vec::new(),
            current_component: None,
        });
    }
    // A placed organization's members follow the map's own records.
    for (index, mut thing) in placed_organizations {
        for &(slot, id) in &thing.updatables {
            if id >= 0 {
                updatable_owners.insert(id, SavedOwner::Placed { index, slot });
            }
        }
        placed[index].mind = thing.mind.take();
        placed[index].components = std::mem::take(&mut thing.components);
        placed[index].current_component = thing.tail.as_ref().map(|t| t.current_component);
        let Some(mut organization) = thing.organization.take() else { continue };
        let owner_slot = placed[index].owner_slot;
        let mut members = Vec::new();
        for (slot, mut member) in std::mem::take(&mut organization.members) {
            let member_index = placed.len();
            for &(component, id) in &member.updatables {
                if id >= 0 {
                    updatable_owners.insert(id, SavedOwner::Placed { index: member_index, slot: component });
                }
            }
            let mind = member.mind.take();
            let components = std::mem::take(&mut member.components);
            let current_component = member.tail.as_ref().map(|t| t.current_component);
            let mut extra = crate::saved_world::extra_placed(&crate::saved_world::SavedEntry { thing: member, organization: None });
            extra.owner_slot = owner_slot;
            extra.member_of = Some(index);
            extra.mind = mind;
            extra.components = components;
            extra.current_component = current_component;
            placed.push(extra);
            members.push((slot, member_index));
        }
        placed[index].organization = Some(PlacedOrganization {
            members,
            leader: organization.leader,
            morale_bits: organization.morale_bits,
            formation: organization.formation,
            position_bits: organization.position_bits,
            offsets: std::mem::take(&mut organization.offsets),
        });
    }
    // A saved game: its groups and members follow the records, and each thing's
    // owner and Updatables come from its whole read.
    if let Some(things) = &saved_things {
        for entry in things.entries.iter().skip(placed.len()) {
            placed.push(crate::saved_world::extra_placed(entry));
        }
        for (index, entry) in things.entries.iter().enumerate() {
            for &(slot, id) in &entry.thing.updatables {
                if id >= 0 {
                    updatable_owners.insert(id, SavedOwner::Placed { index, slot });
                }
            }
            if let Some(handle) = entry.thing.record.actor.as_ref().map(|a| a.owner_handle).filter(|h| *h != 0) {
                placed[index].owner_slot = slots.iter().position(|s| s.serialized_id == Some(handle)).map(|s| s as u32);
            }
        }
    }
    // Start locations of the player (non-independent) slots, in slot order.
    let mut start_locations = Vec::new();
    let mut player_slots = Vec::new();
    for (i, slot) in slots.iter().enumerate() {
        if slot.independent {
            continue;
        }
        let find = |thing: &str| placed.iter().find(|p| p.owner_slot == Some(i as u32) && p.thing_id.eq_ignore_ascii_case(thing));
        match find("start_loc").or_else(|| find("placeholder_corpsunpacked")) {
            Some(p) => {
                start_locations.push(Vec2::new(p.position[0], p.position[1]));
                player_slots.push(i);
            }
            None => notes.push(format!("kingdom slot {} has no start location", slot.id)),
        }
    }
    // Simulation tiles.
    let chunk = aa_mapworld::terrain::TerrainChunk::decode(&archive.decode_chunk("TERRAIN", 1 << 30)?)?;
    let extents = header.creator.world_extents_bits.map(f32::from_bits);
    let layout = aa_mapworld::terrain::TerrainGridLayout::from_world_extents(header.creator.world_extents_bits, chunk.resolution_bits)?;
    let (tw, th) = (layout.width as u32, layout.height as u32);
    let cell = f32::from_bits(layout.cell_size_bits);
    let saved_topology = aa_navigation::topology::Topology::decode_prefix(
        &chunk.pending_auxiliary_bytes,
        aa_navigation::topology::GridLayout { width: layout.width, height: layout.height, coordinate_shift: layout.coordinate_shift, extent_bits: header.creator.world_extents_bits },
        inventory.world.version,
    )
    .ok()
    .map(|(topology, _)| std::sync::Arc::new(topology));
    if let Some(d) = &drawn
        && (tw != d.width || th != d.height || (cell - d.cell_size).abs() > 1e-4)
    {
        anyhow::bail!("{map_id}: RMP terrain grid {tw}x{th}@{cell} differs from the drawn terrain {}x{}@{}", d.width, d.height, d.cell_size);
    }
    let tables = &data.terrain;
    let palette: Vec<Option<&TerrainVariationDef>> = chunk
        .palette_utf16
        .iter()
        .map(|id| tables.find(&utf16(id)))
        .collect();
    let mut used = vec![false; palette.len()];
    for tile in &chunk.tiles {
        if let Some(u) = used.get_mut(tile.variation_index() as usize) {
            *u = true;
        }
    }
    for (i, p) in palette.iter().enumerate() {
        if p.is_none() && used[i] {
            notes.push(format!("terrain variation {} unresolved: tiles use the previous tile's terrain (native 557D4B fallback)", utf16(&chunk.palette_utf16[i])));
        }
    }
    let biome = utf16(&chunk.biome_utf16);
    let initial = tables.biome_initial.get(&biome.to_ascii_lowercase()).and_then(|v| tables.find(v));
    let mut variations: Vec<String> = Vec::new();
    let mut variation_index = |id: &str| -> u16 {
        match variations.iter().position(|v| v == id) {
            Some(i) => i as u16,
            None => {
                variations.push(id.to_owned());
                (variations.len() - 1) as u16
            }
        }
    };
    let mut flags = Vec::with_capacity(chunk.tiles.len());
    let mut speed_limit = Vec::with_capacity(chunk.tiles.len());
    let mut cell_variation = Vec::with_capacity(chunk.tiles.len());
    let mut overlay_kind = Vec::with_capacity(chunk.tiles.len());
    let mut noise = Vec::with_capacity(chunk.tiles.len());
    let mut previous = initial;
    for (i, tile) in chunk.tiles.iter().enumerate() {
        let def = palette.get(tile.variation_index() as usize).copied().flatten().or(previous);
        previous = def;
        let (x, y) = (i as u32 % tw, i as u32 / tw);
        let mut f = u32::from(tile.saved_flags()) & !TERRAIN_TYPE_MASK;
        let (kind, bits) = def.map_or((0, 1), |d| (d.native_type, u8::from(d.land) | (u8::from(d.water) << 1)));
        f |= kind << TERRAIN_TYPE_SHIFT;
        if x == 0 || y == 0 || x + 1 == tw || y + 1 == th {
            f |= BLOCKED;
        }
        flags.push(aa_mapworld::terrain::recompute_passability(f, bits));
        speed_limit.push(def.map_or(NO_SPEED_LIMIT, |d| d.speed_limit));
        cell_variation.push(def.map_or(u16::MAX, |d| variation_index(&d.id)));
        overlay_kind.push(def.map_or(0, |d| d.native_type as u8));
        noise.push(def.map_or([0.0, 0.0], |d| d.noise));
    }
    let native_terrain = native_terrain(&archive, header.creator.world_extents_bits, layout.coordinate_shift, overlay_kind, noise, (tw as i32, th as i32), &mut notes);
    // The saved periodic queues (UPDATABLES, read by 0x50B8B0).
    if component_errors > 0 {
        notes.push(format!("{component_errors} placed things' saved components could not be walked for their Updatable ids"));
    }
    let all_queues = archive.decode_chunk("UPDATABLES", 1 << 30).ok().and_then(|chunk| saved_periodic(&chunk.bytes, inventory.world.version));
    let mut resolve_queues = |queues: Vec<(usize, Vec<Vec<(i32, u32)>>)>, what: &str| -> Vec<SavedPeriodic> {
        let mut unknown = 0usize;
        let out: Vec<SavedPeriodic> = queues
            .into_iter()
            .map(|(current, buckets)| SavedPeriodic {
                current,
                buckets: buckets
                    .into_iter()
                    .map(|b| {
                        b.into_iter()
                            .map(|(source, data)| {
                                let owner = updatable_owners.get(&source).copied().unwrap_or(SavedOwner::Unknown(source));
                                unknown += usize::from(matches!(owner, SavedOwner::Unknown(_)));
                                (owner, data)
                            })
                            .collect()
                    })
                    .collect(),
            })
            .collect();
        if unknown > 0 {
            notes.push(format!("{unknown} saved {what} queue entries have owners aa-play does not resolve"));
        }
        out
    };
    let (saved_queues, saved_deferred) = match all_queues {
        Some(mut queues) => {
            let periodic = queues.split_off(2);
            (Some(resolve_queues(periodic, "periodic")), Some(resolve_queues(queues, "deferred")))
        }
        None => (None, None),
    };
    let queue0_kingdoms = saved_queues.as_ref().map(|q| {
        q[0].buckets.iter().flatten().filter_map(|(o, _)| if let SavedOwner::Kingdom { slot } = o { Some(*slot) } else { None }).collect()
    });
    // A stock map's corners are the drawn terrain's; any other map's are the
    // ground query 0x4F0E50 at the corner, the same rule on its exact words.
    let (w, h, cell_size, heights, name) = match drawn {
        Some(d) => (d.width, d.height, d.cell_size, d.heights, d.name),
        None => {
            let heights = (0..=th)
                .flat_map(|y| (0..=tw).map(move |x| (x, y)))
                .map(|(x, y)| native_terrain.as_ref().map_or(0.0, |n| n.ground(x as f32 * cell, y as f32 * cell)))
                .collect();
            (tw, th, cell, heights, name.unwrap_or_else(|| map_id.to_owned()))
        }
    };
    Ok(GameMap {
        id: map_id.to_owned(),
        name,
        width: w,
        height: h,
        cell_size,
        heights,
        flags,
        speed_limit,
        start_locations,
        placed,
        slots,
        player_slots,
        teams,
        world_extents: extents,
        biome,
        weather: utf16(&header.creator.weather),
        name_key: utf16(&header.name_key),
        description_key: utf16(&header.description_key),
        cell_variation,
        variations,
        load_notes: notes,
        rng_state: Some(inventory.world.rng_state),
        rng_log: vec![aa_simulation::NativeRng::default().state],
        id_manager: Some(IdManager {
            next_slot: inventory.id_manager.next_slot,
            highest_slot: inventory.id_manager.highest_slot,
            generations: inventory.id_manager.serialized_generations.clone(),
        }),
        team_ids: inventory.teams.iter().flat_map(|t| &t.slots).filter_map(|s| s.record.as_ref()).map(|r| r.serialized_id).collect(),
        queue0_kingdoms,
        saved_queues,
        saved_deferred,
        generated: None,
        native_terrain,
        saved_topology,
    })
}

/// HEIGHT_MAP and MOUNTAIN_HEIGHT_MAP (0x551E60 on the grid of 0x5519A0) with
/// the tile data the ground query reads. A map whose layers do not load keeps
/// the corner heights, with a note.
fn native_terrain(
    archive: &aa_mapworld::MapArchive,
    extents: [u32; 2],
    tile_shift: u32,
    overlay_kind: Vec<u8>,
    noise: Vec<[f32; 2]>,
    tiles: (i32, i32),
    notes: &mut Vec<String>,
) -> Option<Box<NativeTerrain>> {
    use aa_heightfield::binding::{GridLayout, LoadedHeightfield};
    let load = |name: &str| -> Result<LoadedHeightfield, String> {
        let chunk = archive.decode_chunk(name, 1 << 30).map_err(|e| e.to_string())?;
        // +0x10 of the chunk is the resolution 0x551E60 checks against the
        // constructed layout's (HeightmapResolutionBits).
        let shift = chunk.bytes.get(16..20).map_or(0, |b| u32::from_le_bytes(b.try_into().unwrap_or([0; 4])));
        let layout = GridLayout::from_world_extents(extents, shift).map_err(|e| e.to_string())?;
        LoadedHeightfield::decode(layout, &chunk.bytes).map_err(|e| e.to_string())
    };
    let layers = load("HEIGHT_MAP").and_then(|base| Ok((base, load("MOUNTAIN_HEIGHT_MAP")?)));
    let (base, overlay) = match layers {
        Ok(v) => v,
        Err(e) => {
            notes.push(format!("height layers not read ({e}): ground heights come from the corner heights"));
            return None;
        }
    };
    let words = |h: &LoadedHeightfield| -> Vec<f32> { h.state.field.height_bits.iter().map(|b| f32::from_bits(*b)).collect() };
    let field = &base.state.field;
    Some(Box::new(NativeTerrain {
        width: field.width,
        height: field.height,
        shift: field.coordinate_shift,
        cell: f32::from_bits(field.cell_size_bits),
        base: words(&base),
        overlay: words(&overlay),
        tile_width: tiles.0,
        tile_height: tiles.1,
        tile_shift,
        overlay_kind,
        noise,
    }))
}

/// The queues of the UPDATABLES chunk, in file order 3, 4, 0, 1, 2: per queue
/// its current bucket and, bucket by bucket, the (owner table id, data)
/// entries in their saved (delivery) order. The chunk is the table count, then for
/// world version >= 22 the queues 3, 4, 0, 1, 2 (version 0, current, bucket
/// count, then per bucket a count and that many (id, data) pairs; a
/// negative count is an empty list), as aa-runtime `saved_queues::decode`
/// reads it.
#[allow(clippy::type_complexity)]
fn saved_periodic(bytes: &[u8], world_version: u32) -> Option<Vec<(usize, Vec<Vec<(i32, u32)>>)>> {
    if world_version < 22 {
        return None;
    }
    let mut at = 0usize;
    let mut word = || -> Option<u32> {
        let w = u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?);
        at += 4;
        Some(w)
    };
    word()?;
    let mut queues = Vec::with_capacity(5);
    for _ in [3usize, 4, 0, 1, 2] {
        if word()? != 0 {
            return None;
        }
        let current = word()? as i32;
        let count = word()? as i32;
        if count <= 0 || current < 0 || current >= count {
            return None;
        }
        let mut buckets = Vec::new();
        for _ in 0..count {
            let n = word()? as i32;
            let mut bucket = Vec::new();
            for _ in 0..n.max(0) {
                let source = word()? as i32;
                let data = word()?;
                bucket.push((source, data));
            }
            buckets.push(bucket);
        }
        queues.push((current as usize, buckets));
    }
    Some(queues)
}
