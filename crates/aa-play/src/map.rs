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
    /// The saved periodic queues 0, 1, 2 (GWorld+1AC/+1C0/+1D4), owners
    /// resolved; `None` as for `queue0_kingdoms`. See `crate::scheduler`.
    #[serde(default)]
    pub saved_queues: Option<Vec<SavedPeriodic>>,
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
    /// The economy Updatable (+180) of the kingdom of `slots[slot]`.
    Kingdom { slot: usize },
    /// Component `component` of the kingdom of `slots[slot]`.
    KingdomComponent { slot: usize, component: u8 },
    /// A table id aa-play does not resolve (negative: the native null id).
    Unknown(i32),
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
    /// Bilinear ground height at a native ground point.
    pub fn height_at(&self, p: Vec2) -> f32 {
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
}

pub(crate) fn terrain_tables(catalog: &aa_rules::actors::ActorCatalog, notes: &mut Vec<String>) -> TerrainTables {
    use crate::data::loader::{PC, bl, st};
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

fn read_rmp(game_root: &std::path::Path, map_id: &str) -> anyhow::Result<Vec<u8>> {
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

fn load(game_root: &std::path::Path, map_id: &str, data: &crate::data::GameData) -> anyhow::Result<GameMap> {
    let mut notes = Vec::new();
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
    // The original RMP.
    let archive = aa_mapworld::MapArchive::from_bytes(read_rmp(game_root, map_id)?)?;
    let inventory = aa_mapworld::WorldInventory::decode(&archive, 1 << 30)?;
    let saved = &inventory.world;
    if saved.word_dc != 0 || saved.begin_time_bits != 0 || saved.word_e4 != 0 || saved.delta_bits != 0.0625f32.to_bits() {
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
    for (index, p) in inventory.dynamic_things.placements.iter().enumerate() {
        // The saved components' Updatable ids (component Load order, the
        // actor tail of 0x4D6F10's restore; aa-mapworld component codecs).
        if let Some(actor) = p.actor.as_ref() {
            let walked = aa_mapworld::actor_payload::ActorTail::decode_prefix_with_stages(&p.opaque_tail, actor.version, 0, |slot, saved, _, _, bytes| {
                if !saved {
                    return Ok((0, None));
                }
                let (payload, consumed) = aa_mapworld::saved_components::SavedComponent::decode_prefix(slot, bytes)?;
                if let Some(id) = payload.updatable().and_then(|u| u.serialized_id).filter(|&id| id >= 0) {
                    updatable_owners.insert(id, SavedOwner::Placed { index, slot });
                }
                let properties = if let aa_mapworld::saved_components::SavedComponent::Properties(v) = &payload { Some(v.clone()) } else { None };
                Ok((consumed, properties))
            });
            if walked.is_err() {
                component_errors += 1;
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
        });
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
    if tw != w || th != h || (cell - terrain.cell_size).abs() > 1e-4 {
        anyhow::bail!("{map_id}: RMP terrain grid {tw}x{th}@{cell} differs from the drawn terrain {w}x{h}@{}", terrain.cell_size);
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
    }
    // The saved periodic queues (UPDATABLES, read by 0x50B8B0).
    if component_errors > 0 {
        notes.push(format!("{component_errors} placed things' saved components could not be walked for their Updatable ids"));
    }
    let saved_queues = archive.decode_chunk("UPDATABLES", 1 << 30).ok().and_then(|chunk| saved_periodic(&chunk.bytes, inventory.world.version)).map(|queues| {
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
            notes.push(format!("{unknown} saved periodic queue entries have owners aa-play does not resolve"));
        }
        out
    });
    let queue0_kingdoms = saved_queues.as_ref().map(|q| {
        q[0].buckets.iter().flatten().filter_map(|(o, _)| if let SavedOwner::Kingdom { slot } = o { Some(*slot) } else { None }).collect()
    });
    let name = std::fs::read(game_root.join("reconstruction").join("game").join("data").join("maps").join("index.json"))
        .ok()
        .and_then(|b| serde_json::from_slice::<Vec<aa_assets::MapInfo>>(&b).ok())
        .and_then(|maps| maps.into_iter().find(|m| m.id == map_id).map(|m| m.name))
        .unwrap_or_else(|| terrain.name.clone());
    Ok(GameMap {
        id: map_id.to_owned(),
        name,
        width: w,
        height: h,
        cell_size: terrain.cell_size,
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
    })
}

/// The periodic queues 0, 1, 2 of the UPDATABLES chunk: per queue its
/// current bucket and, bucket by bucket, the (owner table id, data) entries
/// in their saved (delivery) order. The chunk is the table count, then for
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
    let mut periodic = vec![(0usize, Vec::new()); 3];
    for id in [3usize, 4, 0, 1, 2] {
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
        if id < 3 {
            periodic[id] = (current as usize, buckets);
        }
    }
    Some(periodic)
}
