//! Shapes the generated world like the extracted original maps: an
//! aa_assets::TerrainMap (palette of original Variation ids and textures,
//! base + overlay corner grids), the placed things with their original
//! models, and the aa_play GameMap the battle runs on (flags20-style cell
//! flags, Variation speed caps, native corner heights, start locations and
//! kingdom slots), built like the core's RMP loader does for real maps.
use super::data::RmgData;
use super::grid::World;
use super::place::{Kingdom, Object, Placement};
use super::{GeneratedPlacement, RandomMapParams};
use crate::map::{GameMap, PlacedThing};
use serde_json::{Value, json};
use std::collections::BTreeMap;

/// Native terrain type in flags20 bits 4..6 (as the core's RMP loader stores it).
const TERRAIN_TYPE_SHIFT: u32 = 4;
/// `GameMap::speed_limit` of an uncapped cell (the core's NO_SPEED_LIMIT).
const NO_SPEED_LIMIT: f32 = f32::MAX;

fn normalize(path: &str) -> Option<String> {
    let mut parts = Vec::new();
    for part in path.replace('\\', "/").split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop()?;
            }
            p => parts.push(p.to_owned()),
        }
    }
    (!parts.is_empty()).then(|| parts.join("/"))
}

/// resource_prefix + aspect_prefix into the aspect's model format (0x405D60,
/// source-relative like 0x4763C0); the same rule as the core's data loader.
pub(super) fn model_path(folder: &str, resource: &str, aspect: &str, format: &str) -> Option<String> {
    let prefix = if aspect.starts_with('/') { aspect.to_owned() } else { format!("{resource}{aspect}") };
    let expanded = if format.contains("%s") { format.replacen("%s", &prefix, 1) } else { format.to_owned() };
    if expanded.is_empty() {
        return None;
    }
    let path = if let Some(p) = expanded.strip_prefix('/') {
        p.to_owned()
    } else if folder.is_empty() {
        expanded
    } else {
        format!("{folder}/{expanded}")
    };
    normalize(&path)
}

/// A Variation texture as the extracted maps name it
/// ("/assets/converted/textures/Terrain/.../X.png").
pub(super) fn texture_path(folder: &str, texture: &str) -> String {
    let joined = if let Some(t) = texture.strip_prefix('/') { t.to_owned() } else { format!("{folder}/{texture}") };
    let joined = normalize(&joined).unwrap_or(joined);
    let stem = joined.rsplit_once('.').map_or(joined.as_str(), |(s, _)| s);
    format!("/assets/converted/textures/{stem}.png")
}

pub(super) struct Output {
    pub terrain: aa_assets::TerrainMap,
    pub placed: Vec<PlacedThing>,
    pub scenery: Vec<GeneratedPlacement>,
    pub map: GameMap,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn build(data: &RmgData, params: &RandomMapParams, world: &World, placement: &Placement<'_>, weather: Option<&str>, notes: Vec<String>) -> anyhow::Result<Output> {
    let rmc = data.rmc(&params.rmc).ok_or_else(|| anyhow::anyhow!("unknown RMC {}", params.rmc))?;
    let id = format!("random/{}_{}x{}_{:016x}", rmc.ids, params.width, params.height, params.seed);
    // Palette: every variation in use, keyed by its catalog index.
    let mut palette = BTreeMap::new();
    for &v in &world.var {
        palette.entry(v).or_insert_with(|| {
            let d = &data.variations[v as usize];
            aa_assets::TerrainPaletteEntry {
                id: d.ids.clone(),
                texture: d.texture.clone(),
                color: d.color,
                width: Some(d.texture_size[0]),
                height: Some(d.texture_size[1]),
            }
        });
    }
    let corners: Vec<f32> = (0..world.hh)
        .flat_map(|y| (0..world.hw).map(move |x| (x, y)))
        .map(|(x, y)| super::height::native_corner(world, data, x, y))
        .collect();
    let terrain = aa_assets::TerrainMap {
        id: id.clone(),
        name: rmc.name.clone(),
        description: rmc.description.clone(),
        source: format!("mapgen:{}", rmc.ids),
        width: world.tw as u32,
        height: world.th as u32,
        cell_size: world.tile,
        biomes: vec![rmc.biome.clone()],
        tiles: world.var.clone(),
        heights: world.base.clone(),
        mountain_heights: world.overlay.clone(),
        palette,
        limitations: vec![
                        "heights = base HEIGHT_MAP, mountainHeights = MOUNTAIN_HEIGHT_MAP overlay; draw with the native conditional height.".into(),
        ],
    };
    // Kingdom slots: majors in RMC order, then the independents.
    let slot_of = |k: usize| k;
    let team_ids: Vec<u32> = {
        let mut t: Vec<u32> = placement.kingdoms.iter().filter(|k| k.major).map(|k| k.team.unwrap_or(u32::MAX)).collect();
        t.sort_unstable();
        t.dedup();
        t.retain(|&v| v != u32::MAX);
        t
    };
    let mut slots = Vec::new();
    let mut team_names: Vec<String> = team_ids.iter().map(|t| format!("team_{t}")).collect();
    for k in &placement.kingdoms {
        slots.push(slot_json(data, rmc, k, &team_ids, &mut team_names));
    }
    let mut start_locations = Vec::new();
    let mut player_slots = Vec::new();
    for (i, k) in placement.kingdoms.iter().enumerate() {
        if k.major {
            // The start location object, which PrepareMapFixStartLocations
            // moved onto the Corps HQ placeholder.
            let p = k.start;
            start_locations.push(serde_json::to_value(glam::Vec2::new(p[0], p[1]))?);
            player_slots.push(slot_of(i));
        }
    }
    let mut placed = Vec::new();
    let mut scenery = Vec::new();
    let push = |o: &Object, placed: &mut Vec<Value>, scenery: &mut Vec<GeneratedPlacement>| {
        let def = &data.things[o.thing];
        let z = o.z;
        let yaw_native = o.facing[1].atan2(o.facing[0]);
        placed.push(json!({
            "thing_id": def.ids,
            "position": [o.pos[0], o.pos[1], z],
            "yaw": yaw_native,
            "owner_slot": o.owner.map(slot_of),
            "facing": [o.facing[0], o.facing[1], 0.0],
            "scale": o.scale,
            "variant": o.variant,
            "serialized_id": o.id,
            "properties": placement.group_properties(o.group),
        }));
        let model = if def.invisible {
            None
        } else if o.variant >= 0 {
            def.models.get(o.variant as usize).cloned()
        } else {
            def.models.first().cloned()
        };
        scenery.push(GeneratedPlacement {
            thing_id: def.ids.clone(),
            model,
            position: [o.pos[0], o.pos[1], z],
            yaw: o.facing[0].atan2(-o.facing[1]),
            scale: o.scale,
        });
    };
    // In GTID order, as the world walks its things.
    let mut alive: Vec<&Object> = placement.objects.iter().filter(|o| o.alive).collect();
    alive.sort_by_key(|o| o.id);
    for o in alive {
        push(o, &mut placed, &mut scenery);
    }
    // Cells: native type in bits 4..6, the blocked outer ring, land/water bits.
    let mut flags = Vec::with_capacity(world.var.len());
    let mut speed = Vec::with_capacity(world.var.len());
    let mut cell_variation = Vec::with_capacity(world.var.len());
    let mut variations: Vec<String> = Vec::new();
    let mut index_of: BTreeMap<u32, u16> = BTreeMap::new();
    for (t, &v) in world.var.iter().enumerate() {
        let vd = &data.variations[v as usize];
        let td = &data.terrains[vd.terrain];
        let (x, y) = (t % world.tw, t / world.tw);
        let mut f = u32::from(td.kind) << TERRAIN_TYPE_SHIFT;
        if x == 0 || y == 0 || x + 1 == world.tw || y + 1 == world.th {
            f |= crate::map::BLOCKED;
        }
        let f = if f & crate::map::BLOCKED != 0 {
            f | crate::map::NOT_LAND_PASSABLE | crate::map::NOT_WATER_PASSABLE
        } else {
            (f & !0x300) | if td.land { 0 } else { crate::map::NOT_LAND_PASSABLE } | if td.water { 0 } else { crate::map::NOT_WATER_PASSABLE }
        };
        flags.push(f);
        speed.push(vd.speed_limit.unwrap_or(NO_SPEED_LIMIT));
        let idx = *index_of.entry(v).or_insert_with(|| {
            variations.push(vd.ids.clone());
            (variations.len() - 1) as u16
        });
        cell_variation.push(idx);
    }
    let map_json = json!({
        "id": id,
        "name": rmc.name,
        "width": world.tw,
        "height": world.th,
        "cell_size": world.tile,
        "heights": corners,
        "flags": flags,
        "speed_limit": speed,
        "start_locations": start_locations,
        "placed": placed,
        "slots": slots,
        "player_slots": player_slots,
        "teams": team_names,
        "world_extents": [world.width, world.height],
        "biome": rmc.biome,
        "weather": weather.unwrap_or_default(),
        "name_key": rmc.name,
        "description_key": rmc.description,
        "cell_variation": cell_variation,
        "variations": variations,
        "load_notes": notes,
    });
    let placed_things: Vec<PlacedThing> = serde_json::from_value(map_json["placed"].clone())?;
    let mut map: GameMap = serde_json::from_value(map_json)?;
    // The ground the world queries (0x4F0E50): the layers and tiles as they stand.
    let terrain_of = |v: u32| &data.terrains[data.variations[v as usize].terrain];
    map.native_terrain = Some(Box::new(crate::map::NativeTerrain {
        width: world.hw as i32,
        height: world.hh as i32,
        shift: world.height_bits,
        cell: world.spacing,
        base: world.base.clone(),
        overlay: world.overlay.clone(),
        tile_width: world.tw as i32,
        tile_height: world.th as i32,
        tile_shift: world.tile_bits,
        overlay_kind: world.var.iter().map(|&v| terrain_of(v).kind).collect(),
        noise: world.var.iter().map(|&v| terrain_of(v).noise).collect(),
    }));
    Ok(Output { terrain, placed: placed_things, scenery, map })
}

fn slot_json(data: &RmgData, rmc: &super::data::RmcDef, k: &Kingdom, team_ids: &[u32], team_names: &mut Vec<String>) -> Value {
    let def = &rmc.kingdoms[k.def];
    let color = def.color.as_ref().and_then(|c| data.colors.iter().position(|x| x.eq_ignore_ascii_case(c)));
    let team = if k.major { k.team.and_then(|t| team_ids.iter().position(|&x| x == t)).map(|i| i as u32) } else { None };
    // A side without a team gets its own team (free-for-all).
    let (team, team_id) = match (k.major, team) {
        (true, Some(t)) => (t, format!("team_{}", team_ids[t as usize])),
        (true, None) => {
            team_names.push(format!("team_{}", def.ids));
            ((team_names.len() - 1) as u32, def.ids.clone())
        }
        _ => (u32::MAX, String::new()),
    };
    json!({
        "nation": def.nation,
        "team": team,
        "color": color,
        "id": def.ids,
        "independent": !k.major,
        "color_id": def.color.clone().unwrap_or_default(),
        "team_id": team_id,
        "faction": Value::Null,
        // Kingdom+0x188: the position in the RMC's (the creator's) kingdom list.
        "kingdom_index": k.def,
    })
}
