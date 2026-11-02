//! Placement stages of KKC_RandomMapGenerator::_GenerateMap (0x5925E0):
//! - StagePlaceKingdoms (RMGPlaceKingdomsStage.cpp 0x5963B0): team markers,
//!   then kingdom markers, spread by the marker balancer
//!   (KKC_RMGBalancer::BalanceMarkers 0x590B10, pair penalty 0x590E10, the
//!   kingdom balancer callbacks 0x5954B0..0x5956A0);
//! - StagePlaceRequiredObjects (0x593F30 / PlaceRequiredObject 0x591E60);
//! - StagePlaceNonRequiredObjects (0x595180: PlaceGroup 0x594590 and
//!   BalancePlaceGroup 0x594E60), with the group context callbacks
//!   (search circle 0x593EB0, score 0x595340, validity 0x595430);
//! - FindRandomPosition (0x591A30, 90 tries, lowest score) and
//!   CheckPositionValid (0x591350: terrain, main land region, overlap);
//! - the later stages live in `doodads.rs` and `session.rs`, the water doodads
//!   in `shore.rs`.
use super::data::{GroupDef, RmcDef, RmgData};
use super::grid::World;
use super::rng::Rng;

#[derive(Clone, Debug)]
pub(super) struct Object {
    pub thing: usize,
    pub pos: [f32; 2],
    /// GThing +0x28.
    pub z: f32,
    pub facing: [f32; 2],
    pub scale: f32,
    pub variant: i32,
    /// Index into `Placement::kingdoms`.
    pub owner: Option<usize>,
    pub group: Option<usize>,
    /// GThing +0x14, the slot of its GTID.
    pub id: u32,
    /// Its entry in `Space`.
    pub member: u32,
    pub alive: bool,
    /// The city whose layout created it (ActorParams +0x38 mode 6, +0x4C).
    pub parent: Option<usize>,
    /// The placeholder or random thing it replaced.
    pub placeholder_of: Option<usize>,
    /// The trailer its InitCreate made (0x4D70F2).
    pub trailer: Option<usize>,
    /// Flag bit 3 (0x4EE930): an automatic doodad of its tile.
    pub auto: bool,
}

/// The parameter block of CreateGThing 0x4F9C10 (0x4C8C20 / 0x4C8C70): +4 the
/// definition, +8 position, +0x14 facing, +0x20 variant, +0x28 the owner, +0x34
/// a requested GTID, +0x38 the mode (6 a city layout's object), +0x4C its city.
#[derive(Clone, Debug)]
pub(super) struct Create {
    pub thing: usize,
    pub pos: [f32; 2],
    pub z: f32,
    pub facing: [f32; 2],
    pub variant: i32,
    pub owner: Option<usize>,
    pub group: Option<usize>,
    pub id: Option<u32>,
    pub parent: Option<usize>,
    pub replaces: Option<usize>,
    /// +0x3C set (an actor's replacement, 0x4FD8C0): 0x4F3640 is strict.
    pub strict: bool,
}

impl Default for Create {
    fn default() -> Self {
        Self { thing: 0, pos: [0.0, 0.0], z: 0.0, facing: DEFAULT_FACING, variant: -1, owner: None, group: None, id: None, parent: None, replaces: None, strict: false }
    }
}

#[derive(Clone, Debug)]
pub(super) struct Kingdom {
    /// Index into `RmcDef::kingdoms`.
    pub def: usize,
    pub major: bool,
    pub team: Option<u32>,
    /// The start location object: the balanced marker until the Corps HQ
    /// placeholder (starting_location_priority 1000) moves it onto the HQ.
    pub start: [f32; 2],
    pub start_priority: i32,
    /// The balanced marker (where start_loc stands).
    pub marker: [f32; 2],
    /// Position of the highest-priority owned actor (the Corps HQ).
    pub hq: Option<[f32; 2]>,
    /// The nation it was given or drew (ConfigureGWorld 0x50E8B0).
    pub nation: Option<String>,
}

/// One draw of a thing that leaves something in the world (the stream state
/// before it; `crate::map::GeneratedEvent` once the things have their places).
#[derive(Clone, Copy, Debug)]
pub(super) struct ThingEvent {
    pub state: u32,
    pub what: ThingWhat,
}

#[derive(Clone, Copy, Debug)]
pub(super) enum ThingWhat {
    /// A component's registration on queue 1 (0x563FD0).
    Component { thing: usize, slot: u8 },
    /// The InitCreate or InitExisting of a DynamicDoodad's components.
    Doodad { thing: usize },
    /// 0x567840.
    Defender { defender: usize, intruder: usize },
}

struct GroupInst {
    def: GroupDef,
    ids: String,
    kingdom: Option<usize>,
    objects: Vec<usize>,
    /// The group instance (0x593690) bounding box (min x, min y, max x,
    /// max y), its centre and radius (0x593700).
    bbox: [f32; 4],
    centre: [f32; 2],
    radius: f32,
    /// The parsed restriction list (0x4A2150): (target group, proximity_min,
    /// proximity_desired). A wildcard lists the groups parsed before this one
    /// whose ids start with its prefix.
    restr: Vec<(usize, f32, f32)>,
    /// Group +0x50 resolved by 0x4F48E0 (search_relative_kingdom_ids) and
    /// +0x4C (search_relative_group_ids).
    search_kingdom: Option<usize>,
    search_group: Option<usize>,
}

#[path = "place_exact.rs"]
mod exact;
#[path = "shore.rs"]
mod shore;
#[path = "doodads.rs"]
mod doodads;
#[path = "session.rs"]
mod session;

pub(super) struct Placement<'a> {
    data: &'a RmgData,
    rmc: &'a RmcDef,
    pub objects: Vec<Object>,
    pub kingdoms: Vec<Kingdom>,
    groups: Vec<GroupInst>,
    passable: Vec<bool>,
    region: Vec<u32>,
    main_region: u32,
    pub notes: Vec<String>,
    /// The marker balancer's (iteration, score) log of the kingdom stage
    /// (the original's "balance iteration %d score %f" lines).
    pub balance_log: Vec<(i32, f64)>,
    /// The GMap occupancy the original's placement queries read.
    space: super::space::Space,
    /// Balanced groups: the marker log ("marker %d placed / created at")
    /// and the balancer's (iteration, score) log.
    pub marker_log: Vec<(&'static str, i32, [f32; 2])>,
    pub group_balance_log: Vec<(i32, f64)>,
    /// GTID slots: the next-fit cursor 0x4EFDB0 and the slots in use.
    id_cursor: u32,
    id_used: Vec<bool>,
    /// Things by the tile of their position (GMap tile +8, 0x54FDC0).
    tile_things: Vec<Vec<u32>>,
    /// Things that failed their placement (definition, position).
    pub dead: Vec<(usize, [f32; 2])>,
    /// What the draws left in the world, for `GameMap::generated`.
    pub(super) events: Vec<ThingEvent>,
    /// The generation of every slot (0x4EFEC0 adds one when a thing goes).
    id_generation: Vec<u16>,
    /// Whether the session has started (GWorld +0x48 is still 0 until then:
    /// 0x4EB500 paints base terrain).
    pub started: bool,
    /// The biome (GMap+0x24) for a thing's base terrain.
    biome: &'a super::data::BiomeDef,
    /// The object of a `Space` member.
    by_member: std::collections::HashMap<u32, usize>,
    /// The rules, for what a Defender may target (a film's world has them).
    game: Option<&'a crate::data::GameData>,
    /// The Defenders' zones of control so far.
    zocs: Vec<session::Zoc>,
}

const DEFAULT_FACING: [f32; 2] = [0.0, -1.0];
/// The Structure component's slot: actor +0x94.
const STRUCTURE_SLOT: u32 = 13;
impl<'a> Placement<'a> {
    pub(super) fn new(data: &'a RmgData, rmc: &'a RmcDef, biome: &'a super::data::BiomeDef, world: &World, first_id: u32, game: Option<&'a crate::data::GameData>) -> Self {
        let mut p = Self {
            data,
            rmc,
            objects: Vec::new(),
            kingdoms: Vec::new(),
            groups: Vec::new(),
            passable: Vec::new(),
            region: Vec::new(),
            main_region: 0,
            notes: Vec::new(),
            balance_log: Vec::new(),
            space: super::space::Space::new(world, data),
            marker_log: Vec::new(),
            group_balance_log: Vec::new(),
            id_cursor: first_id,
            id_used: vec![true; first_id as usize + 1],
            tile_things: vec![Vec::new(); world.tw * world.th],
            dead: Vec::new(),
            events: Vec::new(),
            id_generation: Vec::new(),
            started: false,
            biome,
            by_member: std::collections::HashMap::new(),
            game,
            zocs: Vec::new(),
        };
        p.update_regions(world);
        p
    }

    /// Land-passable tiles (terrain `land`, not the blocked outer ring) and
    /// their connected regions; the largest is the main land region the
    /// original requires every placed object to stand in (0x591350).
    pub(super) fn update_regions(&mut self, world: &World) {
        let (w, h) = (world.tw, world.th);
        self.passable = (0..w * h)
            .map(|t| {
                let (x, y) = (t % w, t / w);
                let edge = x == 0 || y == 0 || x + 1 == w || y + 1 == h;
                !edge && self.data.terrains[self.data.variations[world.var[t] as usize].terrain].land
            })
            .collect();
        self.region = vec![0; w * h];
        let mut sizes = vec![0usize];
        let mut stack = Vec::new();
        for s in 0..w * h {
            if !self.passable[s] || self.region[s] != 0 {
                continue;
            }
            let id = sizes.len() as u32;
            sizes.push(0);
            self.region[s] = id;
            stack.push(s);
            while let Some(t) = stack.pop() {
                sizes[id as usize] += 1;
                let (x, y) = ((t % w) as i32, (t / w) as i32);
                for d in [[1, 0], [-1, 0], [0, 1], [0, -1]] {
                    if let Some(n) = world.tile_at(x + d[0], y + d[1]) {
                        if self.passable[n] && self.region[n] == 0 {
                            self.region[n] = id;
                            stack.push(n);
                        }
                    }
                }
            }
        }
        self.main_region = (1..sizes.len()).max_by_key(|&i| (sizes[i], std::cmp::Reverse(i))).unwrap_or(0) as u32;
    }

    pub(super) fn in_main_region(&self, world: &World, p: [f32; 2]) -> bool {
        world.tile_of(p).is_some_and(|t| self.passable[t] && self.region[t] == self.main_region)
    }

    /// The `properties` of a generated group (RMCActorGroup), given to
    /// every actor it places (e.g. tech_noupkeep on the start bunkers).
    pub(super) fn group_properties(&self, group: Option<usize>) -> Vec<String> {
        group
            .and_then(|g| self.groups.get(g))
            .and_then(|g| g.def.properties.as_deref())
            .map(|s| s.split([',', ' ', '\t']).map(str::trim).filter(|p| !p.is_empty() && !p.eq_ignore_ascii_case("null")).map(str::to_string).collect())
            .unwrap_or_default()
    }

    // ---------------- kingdoms ----------------

    /// Active kingdoms: the first `sides` playable majors plus every minor.
    pub(super) fn setup_kingdoms(&mut self, sides: u32, teams: &[u32], spec: Option<&super::WorldSpec>) -> anyhow::Result<()> {
        let mut majors = 0u32;
        // The session's sides are named (films with `side05` and no `side03`); +0x188
        // keeps the RMC's position.
        let listed = |ids: &str| spec.is_none_or(|s| s.present.is_empty() || s.present.iter().any(|p| p.eq_ignore_ascii_case(ids)));
        for (i, k) in self.rmc.kingdoms.iter().enumerate() {
            if k.independent {
                self.kingdoms.push(Kingdom { def: i, major: false, team: None, start: [0.0, 0.0], start_priority: 0, marker: [0.0, 0.0], hq: None, nation: None });
            } else if majors < sides && listed(&k.ids) {
                let team = teams.get(majors as usize).copied();
                self.kingdoms.push(Kingdom { def: i, major: true, team, start: [0.0, 0.0], start_priority: 0, marker: [0.0, 0.0], hq: None, nation: None });
                majors += 1;
            }
        }
        anyhow::ensure!(majors == sides, "{} has {majors} playable sides, {sides} requested", self.rmc.ids);
        Ok(())
    }

    /// KKC_RMGPlaceKingdomsStage::Run (0x5963B0) with BalancePlaceKingdoms
    /// (0x595940): one team marker per team with members (team order), then
    /// per kingdom without a team; all start in the map centre and are
    /// spread by the marker balancer (0x590B10) to W / (trunc(sqrt(n)) + 1);
    /// each kingdom marker then starts on its team marker's best position and
    /// is balanced again (same-team pairs to min(team_desired_distance, that
    /// distance)). The kingdoms' start locations are their markers' best
    /// positions. Returns the balance log (iteration, score) of both passes.
    pub(super) fn place_kingdoms_exact(&mut self, world: &World, rng: &mut Rng) -> anyhow::Result<Vec<(i32, f64)>> {
        let mut log = Vec::new();
        let majors: Vec<usize> = (0..self.kingdoms.len()).filter(|&k| self.kingdoms[k].major).collect();
        let start_thing = self.data.thing_by_id.get("start_loc").copied();
        // 0x595530: tile + the start location's shape radius (ObjectData
        // +0x3C) + 0.0001, on the register.
        let radius = start_thing.map_or(0.0, |t| self.data.things[t].shape_radius);
        let margin = f64::from(world.tile) + f64::from(radius) + f64::from(f32::from_bits(0x38D1_B717));
        let orient = DEFAULT_FACING;
        if majors.is_empty() {
            return Ok(log);
        }
        let sv = &self.data.svars;
        let (iterations, max_worse) = (sv.balance_iterations, sv.balance_worse);
        // Team markers: teams with members in team order, then teamless kingdoms.
        let mut teams: Vec<u32> = majors.iter().filter_map(|&k| self.kingdoms[k].team).collect();
        teams.sort_unstable();
        teams.dedup();
        let mut keys: Vec<(Option<u32>, Option<usize>)> = teams.iter().map(|&t| (Some(t), None)).collect();
        keys.extend(majors.iter().filter(|&&k| self.kingdoms[k].team.is_none()).map(|&k| (None, Some(k))));
        if majors.len() == 1 {
            // A single kingdom (0x595940 first branch): up to 50 uniform
            // positions in [d, W - d] with d = tile + radius + 0.0001, the
            // first valid one (CheckPositionValid without context).
            let d = (f64::from(world.tile) + f64::from(radius) + f64::from(f32::from_bits(0x38D1_B717))) as f32;
            let d2 = (f64::from(d) + f64::from(d)) as f32;
            for _ in 0..50 {
                let x = ((f64::from(world.width) - f64::from(d2)) * rng.at(0x595A43).unit_d() + f64::from(d)) as f32;
                let y = ((f64::from(world.height) - f64::from(d2)) * rng.at(0x595A8B).unit_d() + f64::from(d)) as f32;
                if self.in_main_region(world, [x, y]) {
                    self.kingdoms[majors[0]].start = [x, y];
                    self.kingdoms[majors[0]].marker = [x, y];
                    return Ok(log);
                }
            }
            anyhow::bail!("could not place the only kingdom");
        }
        let centre = [(f64::from(world.width) * 0.5) as f32, (f64::from(world.height) * 0.5) as f32];
        let mut team_markers: Vec<Marker> = (0..keys.len()).map(|i| Marker::new(i, centre, orient, None)).collect();
        let n = team_markers.len() as i32;
        let desired = (f64::from(world.width) / f64::from(blob_ftol((f64::from(n)).sqrt()) + 1)) as f32;
        let team_cb = KingdomBalance { mode: 0, desired, same_team: desired, other: desired, margin, teams: Vec::new() };
        balance_markers(world, rng, &mut team_markers, 2.0, 2.0, &team_cb, iterations, max_worse, &mut log);
        // Kingdom markers on their team marker's best position.
        let mut kingdom_markers: Vec<Marker> = Vec::new();
        let mut kingdom_teams = Vec::new();
        for (i, &k) in majors.iter().enumerate() {
            let key = match self.kingdoms[k].team {
                Some(t) => (Some(t), None),
                None => (None, Some(k)),
            };
            let at = keys.iter().position(|&x| x == key).map_or(centre, |m| team_markers[m].best);
            kingdom_markers.push(Marker::new(i, at, orient, Some(k)));
            kingdom_teams.push(self.kingdoms[k].team);
        }
        let same_team = if desired < self.rmc.team_desired_distance { desired } else { self.rmc.team_desired_distance };
        let kingdom_cb = KingdomBalance { mode: 1, desired, same_team, other: desired, margin, teams: kingdom_teams };
        balance_markers(world, rng, &mut kingdom_markers, 2.0, 2.0, &kingdom_cb, iterations, max_worse, &mut log);
        for (m, &k) in kingdom_markers.iter().zip(&majors) {
            self.kingdoms[k].start = m.best;
            self.kingdoms[k].marker = m.best;
        }
        Ok(log)
    }

    /// 0x4F5770(0), 0x593890: the things and the groups go, teams and kingdoms
    /// stay; the GTID cursor stays where it is.
    pub(super) fn start_over(&mut self, world: &World) {
        for i in 0..self.objects.len() {
            self.delete_object(i);
        }
        self.groups.clear();
        self.kingdoms.clear();
        self.zocs.clear();
        self.balance_log.clear();
        self.marker_log.clear();
        self.group_balance_log.clear();
        self.update_regions(world);
    }

    /// What a world keeps of its making: the GTIDs (IDMGR, the teams' and
    /// kingdoms' first) and the draws that left something on the queues.
    pub(super) fn world_state(&self, map: &mut crate::map::GameMap, spec: &super::WorldSpec, first_id: u32, kingdoms: &[super::KingdomDraw]) {
        use crate::map::{GeneratedEvent, GeneratedWhat, GeneratedWorld, IdManager, SavedOwner};
        let slot_of_kingdom = |i: usize| self.kingdoms.iter().position(|k| self.rmc.kingdoms[k.def].ids.eq_ignore_ascii_case(&spec.kingdoms[i].ids));
        map.team_ids = (1..=spec.teams).collect();
        for i in 0..spec.kingdoms.len() {
            if let Some(s) = slot_of_kingdom(i) {
                map.slots[s].serialized_id = Some(spec.teams + 1 + i as u32);
            }
        }
        // The placed index of a thing: `placed` holds the living things in GTID order.
        let mut alive: Vec<usize> = (0..self.objects.len()).filter(|&i| self.objects[i].alive).collect();
        alive.sort_by_key(|&i| self.objects[i].id);
        let mut placed_of = vec![usize::MAX; self.objects.len()];
        for (index, &o) in alive.iter().enumerate() {
            placed_of[o] = index;
        }
        let mut events = Vec::with_capacity(kingdoms.len() + self.events.len());
        for k in kingdoms {
            let Some(slot) = slot_of_kingdom(k.kingdom) else { continue };
            let what = if k.economy { GeneratedWhat::Periodic { queue: 0, owner: SavedOwner::Kingdom { slot } } } else { GeneratedWhat::Periodic { queue: 1, owner: SavedOwner::KingdomComponent { slot, component: crate::map::PROPERTIES_SLOT } } };
            events.push(GeneratedEvent { state: k.state, what });
        }
        for e in &self.events {
            let what = match e.what {
                ThingWhat::Component { thing, slot } => {
                    let Some(&index) = placed_of.get(thing).filter(|&&p| p != usize::MAX) else { continue };
                    GeneratedWhat::Periodic { queue: 1, owner: SavedOwner::Placed { index, slot } }
                }
                ThingWhat::Doodad { thing } => {
                    let Some(&index) = placed_of.get(thing).filter(|&&p| p != usize::MAX) else { continue };
                    GeneratedWhat::Doodad { index }
                }
                ThingWhat::Defender { defender, intruder } => {
                    let (Some(&d), Some(&i)) = (placed_of.get(defender), placed_of.get(intruder)) else { continue };
                    if d == usize::MAX || i == usize::MAX {
                        continue;
                    }
                    GeneratedWhat::Defender { defender: d, intruder: i }
                }
            };
            events.push(GeneratedEvent { state: e.state, what });
        }
        let mut blocks = Vec::new();
        let mut trailers = Vec::new();
        for (index, o) in self.objects.iter().enumerate() {
            if !o.alive {
                continue;
            }
            // 0x5644F0 with a list: an actor with a Structure (+0x94), of a layout
            // run by a CityComponent (one with a zone radius).
            if let Some(city) = o.parent.filter(|&c| self.objects[c].alive && self.data.things[self.objects[c].thing].zone_radius > 0.0)
                && self.is_actor(o.thing)
                && self.data.things[o.thing].components & (1 << STRUCTURE_SLOT) != 0
            {
                blocks.push((placed_of[city], placed_of[index]));
            }
            if let Some(trailer) = o.trailer.filter(|&t| self.objects[t].alive) {
                trailers.push((placed_of[index], placed_of[trailer]));
            }
        }
        map.generated = Some(GeneratedWorld { events, blocks, trailers });
        // 0x4F0230's state: the cursor, the highest used slot, the generation
        // of every slot below it.
        let highest = alive.iter().map(|&i| self.objects[i].id).max().unwrap_or(first_id).max(first_id);
        let generations = (0..highest as usize).map(|s| self.id_generation.get(s).copied().unwrap_or(0)).collect();
        map.id_manager = Some(IdManager { next_slot: (self.id_cursor + 1).clamp(1, 65535), highest_slot: highest, generations });
    }

    /// The kingdoms' nations after ConfigureGWorld.
    pub(super) fn set_nations(&mut self, spec: &super::WorldSpec, nations: &[Option<String>]) {
        for k in &mut self.kingdoms {
            let ids = &self.rmc.kingdoms[k.def].ids;
            if let Some(i) = spec.kingdoms.iter().position(|s| s.ids.eq_ignore_ascii_case(ids)) {
                k.nation = nations.get(i).cloned().flatten();
            }
        }
    }

    /// 0x7ACB00 again: the islands of the land as it stands.
    pub(super) fn refresh_islands(&mut self) {
        self.space.analyse(self.data);
    }

    /// Every living thing in GTID order.
    pub(super) fn things(&self, data: &RmgData) -> Vec<super::GeneratedThing> {
        let mut alive: Vec<&Object> = self.objects.iter().filter(|o| o.alive).collect();
        alive.sort_by_key(|o| o.id);
        alive
            .into_iter()
            .map(|o| super::GeneratedThing { id: o.id, ids: data.things[o.thing].ids.clone(), position: [o.pos[0], o.pos[1], o.z], facing: o.facing })
            .collect()
    }

    /// The parity probe (`AA_RMG_WORLD` names a file): the final world dump.
    pub(super) fn dump_probe(&self, world: &World) {
        let Some(path) = std::env::var_os("AA_RMG_WORLD") else { return };
        let things: Vec<(String, [f32; 2])> = {
            let mut alive: Vec<&Object> = self.objects.iter().filter(|o| o.alive).collect();
            alive.sort_by_key(|o| o.id);
            alive.into_iter().map(|o| (format!("{:x} {}", o.id, self.data.things[o.thing].ids.to_ascii_lowercase()), o.pos)).collect()
        };
        world.dump(self.data, std::path::Path::new(&path), &things);
    }

    // ---------------- groups ----------------

    /// 0x4A4600 expands apply_to_all_major_kingdoms into one group per major.
    pub(super) fn setup_groups(&mut self) {
        let lists = [&self.rmc.actor_groups, &self.rmc.doodad_groups];
        let inst = |def: &GroupDef, ids: String, kingdom: Option<usize>| GroupInst {
            def: def.clone(),
            ids,
            kingdom,
            objects: Vec::new(),
            bbox: [0.0; 4],
            centre: [0.0; 2],
            radius: 0.0,
            restr: Vec::new(),
            search_kingdom: None,
            search_group: None,
        };
        for list in lists {
            for g in list.iter() {
                if g.all_major_kingdoms {
                    for (k, kd) in self.kingdoms.iter().enumerate() {
                        if kd.major {
                            let ids = format!("{}_{}", g.ids, self.rmc.kingdoms[kd.def].ids);
                            self.groups.push(inst(g, ids, Some(k)));
                        }
                    }
                } else {
                    let kingdom = g.kingdom_ids.as_ref().and_then(|ids| self.kingdoms.iter().position(|k| self.rmc.kingdoms[k.def].ids.eq_ignore_ascii_case(ids)));
                    if g.kingdom_ids.is_some() && kingdom.is_none() {
                        continue;
                    }
                    self.groups.push(inst(g, g.ids.clone(), kingdom));
                }
            }
        }
        // 0x4A2150 restrictions in declaration order, wildcards matching the
        // groups parsed before; 0x4A31A0 / 0x4A4600 search references.
        for gi in 0..self.groups.len() {
            let mut restr = Vec::new();
            for r in &self.groups[gi].def.restrictions {
                if let Some(at) = r.group_ids.find('*') {
                    let prefix = &r.group_ids[..at];
                    for oi in 0..gi {
                        if self.groups[oi].ids.get(..prefix.len()) == Some(prefix) {
                            restr.push((oi, r.min, r.desired));
                        }
                    }
                } else if let Some(oi) = self.groups.iter().position(|o| o.ids == r.group_ids) {
                    restr.push((oi, r.min, r.desired));
                }
            }
            let g = &self.groups[gi];
            let search_kingdom = match g.def.search_relative_kingdom.as_deref() {
                Some("this") => g.kingdom,
                Some(s) => self.kingdoms.iter().position(|k| self.rmc.kingdoms[k.def].ids.eq_ignore_ascii_case(s)),
                None => None,
            };
            let search_group = match g.def.search_relative_group.as_deref() {
                Some("this") => Some(gi),
                Some(s) => self.groups.iter().position(|o| o.def.ids == s || o.ids == s),
                None => None,
            };
            let g = &mut self.groups[gi];
            g.restr = restr;
            g.search_kingdom = search_kingdom;
            g.search_group = search_group;
        }
    }


    /// CreateStartingLocation (0x595820 -> 0x5910D0 -> CreateGActor) for
    /// every placed kingdom, in kingdom order, on its marker.
    pub(super) fn create_start_locations(&mut self, world: &mut World, rng: &mut Rng) {
        if let Some(t) = self.data.thing_by_id.get("start_loc").copied() {
            for k in 0..self.kingdoms.len() {
                if self.kingdoms[k].major {
                    let pos = self.kingdoms[k].start;
                    // 0x5958D7: 0x4EE930(1), gone when its tile's terrain changes.
                    if let Some(index) = self.create_gthing(world, rng, Create { thing: t, pos, owner: Some(k), ..Default::default() }) {
                        self.objects[index].auto = true;
                    }
                }
            }
        }
    }

}

/// One balancer marker (0x5907B0; RMGBalanceMarker): +0 its index, +4/+8
/// position, +0xC/+0x10 position at the best score, +0x14/+0x18 orientation
/// (and its best at +0x1C/+0x20), +0x24 nudge step, +0x28/+0x2C force, +0x30
/// the kingdom of a kingdom marker.
#[derive(Clone, Debug)]
pub(super) struct Marker {
    pub id: usize,
    pub pos: [f32; 2],
    pub best: [f32; 2],
    pub orient: [f32; 2],
    pub best_orient: [f32; 2],
    pub step: f32,
    pub force: [f32; 2],
    /// The marker's kingdom (kingdom markers); kept for the group balancer.
    #[allow(dead_code)]
    pub kingdom: Option<usize>,
    /// +0x30 the marker's definition (object markers).
    pub thing: usize,
}

impl Marker {
    pub(super) fn new(id: usize, pos: [f32; 2], orient: [f32; 2], kingdom: Option<usize>) -> Self {
        Self { id, pos, best: [0.0, 0.0], orient, best_orient: [0.0, 0.0], step: 0.0, force: [0.0, 0.0], kingdom, thing: usize::MAX }
    }
}

/// The balancer's callback object (vtable +0 pair penalty, +4 validity, +8
/// margin, +0xC self score).
pub(super) trait Balance {
    fn pair(&self, rng: &mut Rng, a: &mut Marker, b: &Marker) -> f32;
    /// May move the candidate (CheckPositionValid's snap); the marker moves
    /// to the candidate as left.
    fn valid(&self, m: &Marker, cand: &mut [f32; 2]) -> bool;
    fn margin(&self, m: &Marker) -> f64;
    fn self_score(&self, rng: &mut Rng, m: &mut Marker) -> f64;
}

/// The kingdom balancer (vtable 0xA4228C: 0x5954B0 / 0x595520 / 0x595530 /
/// 0x5956A0): +0x30 mode (0 team markers, 1 kingdom markers), +0x34 the
/// desired distance, +0x38 same-team and +0x3C other-team distances.
pub(super) struct KingdomBalance {
    pub mode: u32,
    pub desired: f32,
    pub same_team: f32,
    pub other: f32,
    /// tile + start location radius + 0.0001 (0x595530, on the register).
    pub margin: f64,
    /// Team of each kingdom marker (by marker id).
    pub teams: Vec<Option<u32>>,
}

impl Balance for KingdomBalance {
    fn pair(&self, rng: &mut Rng, a: &mut Marker, b: &Marker) -> f32 {
        let desired = if self.mode == 0 {
            self.desired
        } else {
            match (self.teams.get(a.id).copied().flatten(), self.teams.get(b.id).copied().flatten()) {
                (Some(x), Some(y)) if x == y => self.same_team,
                _ => self.other,
            }
        };
        pair_penalty_exact(rng, a, b.pos, desired)
    }
    fn valid(&self, _m: &Marker, _cand: &mut [f32; 2]) -> bool {
        true
    }
    fn margin(&self, _m: &Marker) -> f64 {
        self.margin
    }
    fn self_score(&self, _rng: &mut Rng, _m: &mut Marker) -> f64 {
        0.0
    }
}

/// 0x590E10: the penalty of `a` for being closer than `desired` to `q`,
/// adding the push-away force to `a` (on the register at PC53, with the
/// original's binary32 stores). Coincident markers take a random table
/// direction and a distance of 0.5 (one draw).
pub(super) fn pair_penalty_exact(rng: &mut Rng, a: &mut Marker, q: [f32; 2], desired: f32) -> f32 {
    let dx = f64::from(q[0]) - f64::from(a.pos[0]);
    let dy_d = f64::from(q[1]) - f64::from(a.pos[1]);
    let dy = dy_d as f32;
    let d = (dy_d * f64::from(dy) + dx * dx).sqrt() as f32;
    let (ux, uy, dd) = if d < f32::from_bits(0x38D1_B717) {
        let table = super::blob::direction_table();
        let t = table[(rng.at(0x590E66).next_u32() % table.len() as u32) as usize];
        (f64::from(t[0]), t[1], 0.5f64)
    } else {
        let inv = (1.0 / f64::from(d)) as f32;
        (dx * f64::from(inv), (f64::from(dy) * f64::from(inv)) as f32, f64::from(d))
    };
    let f = (dd - f64::from(desired)) as f32;
    if f > 0.0 {
        return 0.0;
    }
    let inv_des = 1.0 / f64::from(desired);
    let fx = (inv_des * (ux * f64::from(f))) as f32;
    let fy = inv_des * (f64::from(uy) * f64::from(f));
    a.force[0] = (f64::from(fx) + f64::from(a.force[0])) as f32;
    a.force[1] = (fy + f64::from(a.force[1])) as f32;
    f.abs()
}

/// 0x590820: a marker's score, the binary32 sum of its pair penalties
/// against every other marker (list order) plus its self score.
fn marker_score(rng: &mut Rng, markers: &mut [Marker], i: usize, cb: &dyn Balance) -> f64 {
    markers[i].force = [0.0, 0.0];
    let mut sum = 0.0f32;
    for j in 0..markers.len() {
        if markers[j].id == markers[i].id {
            continue;
        }
        let other = markers[j].clone();
        let p = cb.pair(rng, &mut markers[i], &other);
        sum = (f64::from(p) + f64::from(sum)) as f32;
    }
    cb.self_score(rng, &mut markers[i]) + f64::from(sum)
}

/// KKC_RMGBalancer::BalanceMarkers (0x590B10): every iteration scores all
/// markers (the total on the register, logged "balance iteration %d score
/// %f"), keeps the positions of the best score, stops on a score below
/// 0.0001, after more than `max_worse` non-improving iterations or after
/// `iterations`; otherwise each marker moves `step` along its unit force,
/// clamped to [margin, size - margin - 0.0001], when its new place is valid
/// (else its step grows by `delta`), and its step then grows by `delta`
/// after an improving iteration or shrinks by it (down to `step0`) after a
/// non-improving one.
#[allow(clippy::too_many_arguments)]
pub(super) fn balance_markers(world: &World, rng: &mut Rng, markers: &mut [Marker], step0: f32, delta: f32, cb: &dyn Balance, iterations: i32, max_worse: i32, log: &mut Vec<(i32, f64)>) -> bool {
    let (mut last, mut best) = (f32::MAX, f32::MAX);
    let mut worse = 0i32;
    for m in markers.iter_mut() {
        m.step = step0;
    }
    if iterations < 0 {
        return true;
    }
    let eps = f32::from_bits(0x38D1_B717);
    let mut iter = 0i32;
    loop {
        let mut total = 0.0f64;
        for i in 0..markers.len() {
            total += marker_score(rng, markers, i, cb);
        }
        let score = if markers.is_empty() { 0.0 } else { total as f32 };
        log.push((iter, total));
        let mut got_worse = false;
        if score < best {
            best = score;
            for m in markers.iter_mut() {
                m.best = m.pos;
                m.best_orient = m.orient;
            }
            if worse > 0 {
                worse -= 1;
            }
            if score < eps {
                return true;
            }
        }
        if score >= last {
            worse += 1;
            if worse > max_worse {
                return true;
            }
            got_worse = true;
        }
        last = score;
        if iter == iterations {
            return true;
        }
        for i in 0..markers.len() {
            let m = &mut markers[i];
            let len = (f64::from(m.force[0]) * f64::from(m.force[0]) + f64::from(m.force[1]) * f64::from(m.force[1])).sqrt();
            if len > 0.0 {
                let inv = 1.0 / len;
                m.force[0] = (inv * f64::from(m.force[0])) as f32;
                m.force[1] = (inv * f64::from(m.force[1])) as f32;
            }
            let oy = (f64::from(m.force[1]) * f64::from(m.step)) as f32;
            let mut cand = [(f64::from(m.step) * f64::from(m.force[0]) + f64::from(m.pos[0])) as f32, (f64::from(oy) + f64::from(m.pos[1])) as f32];
            let margin = cb.margin(m);
            let lim_x = (f64::from(world.width) - margin) - f64::from(eps);
            if f64::from(cand[0]) < margin {
                cand[0] = margin as f32;
            } else if f64::from(cand[0]) > lim_x {
                cand[0] = lim_x as f32;
            }
            let lim_y = ((f64::from(world.height) - margin) - f64::from(eps)) as f32;
            if f64::from(cand[1]) < margin {
                cand[1] = margin as f32;
            } else if cand[1] > lim_y {
                cand[1] = lim_y;
            }
            let m = &markers[i];
            if cb.valid(m, &mut cand) {
                markers[i].pos = cand;
            } else {
                markers[i].step = (f64::from(delta) + f64::from(markers[i].step)) as f32;
            }
            let m = &mut markers[i];
            if got_worse {
                if m.step > step0 {
                    m.step = (f64::from(m.step) - f64::from(delta)) as f32;
                }
            } else {
                m.step = (f64::from(delta) + f64::from(m.step)) as f32;
            }
        }
        iter += 1;
        if iter > iterations {
            return true;
        }
    }
}

fn blob_ftol(v: f64) -> i32 {
    super::blob::ftol_d(v)
}