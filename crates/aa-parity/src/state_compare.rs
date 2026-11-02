//! Field-by-field comparison of aa-play's world with the native sampler's
//! snapshots ([`crate::native_trace`]) of the same tick, lane by lane, over the
//! whole film.
//!
//! Call [`StateComparison::observe`] after every replay step (the hook of
//! `replay::run_observed`); ticks the native trace did not sample are
//! skipped. Each lane keeps its first differing tick and field, and every
//! difference to the end of the film, collapsed into [`Cause`]s:
//! * `rng`: GWorld+D4 against `World::native_rng`, and [`getter_census`] for
//!   the getter stream (every native getter has a site and a call chain);
//! * `existence`: the live class-3 GTIDs against aa-play's live actors,
//!   organizations and trailers (`World::ids`), the definition each GTID
//!   names, and the organization member counts;
//! * `economy`: Kingdom +1C4 stockpiled, +1A0 provided, +1AC effective
//!   provided, +1B8 consumed, +1F0 shortage against `Books` `stockpiled`,
//!   `raw_provided`, `provided`, `consumed`, `shortage_ratio` (kingdoms matched
//!   by the +188 index);
//! * `production`: Structure queue count, deadline, paused time (-1.0 while
//!   running) and duration against `ActorBooks::queue`;
//! * `movement`: actor position (x, y), height, facing (direction angle) and
//!   Mover speed `+20` against `Actor::pos`, `Actor::facing` and
//!   `Motion::speed`; an organization's own position against
//!   `Organization::anchor`;
//! * `combat`: Body health `+10` and maximum `+14` against `Actor::health` and
//!   `production::max_health`, and the dead flag `+8` bit 0;
//! * `supply`: Supply `+18` stock against `ActorBooks::supply_stock`, and zone
//!   slot 0 (ZOS) against the aa-zones engine's;
//! * `fog`: zone slots 1 (RZOD), 4 (ZOC), 6 (ZOD), 8 and the Detection ZOD
//!   radius against the aa-zones engine's.
//!
//! Positions, health, speeds, stocks and zone fields are compared as exact
//! binary32 bits; the facing as an angle within 1e-4 rad (the native keeps a
//! direction vector). A trailer (a GActor natively, `Thing::Trailer` here) is
//! compared for existence only: aa-play draws and moves no trailer.
//!
//! A [`Cause`] is a signature: the lane's field, the kind of thing and how the
//! two sides differ (who moves, who has the zone). It counts the things it
//! touches and how many of them it is the first difference of: a cause that is
//! never first on any thing is a knock-on of an earlier one. [`Forcing`] goes
//! further: a replay can be put back on the native state after every sampled
//! tick, so only what one tick (or eight, after tick 1200) gets wrong remains.
use crate::native_trace::{NativeActor, NativeTick, NativeTrace, NativeZone, getter_group};
use crate::replay::{Pairing, thing_definition};
use aa_play::World;
use aa_play::ids::Thing;
use serde::Serialize;
use std::collections::{BTreeMap, BTreeSet};

pub const LANES: [&str; 8] = [
    "rng",
    "existence",
    "economy",
    "production",
    "movement",
    "combat",
    "supply",
    "fog",
];

/// One differing field.
#[derive(Clone, Debug, Serialize)]
pub struct Divergence {
    pub tick: u32,
    pub gtid: Option<u32>,
    pub name: String,
    pub field: String,
    pub native: String,
    pub ours: String,
    /// The collapsed signature ([`Cause`]).
    pub cause: String,
}

/// All differences of one signature, to the end of the film.
#[derive(Clone, Debug, Serialize)]
pub struct Cause {
    pub lane: &'static str,
    pub signature: String,
    pub first_tick: u32,
    pub last_tick: u32,
    /// Sampled ticks it shows in.
    pub ticks: u32,
    /// Differing fields in total.
    pub fields: u64,
    /// The things it touches.
    pub things: BTreeSet<u32>,
    /// The things whose first difference (in any per-thing lane) it is.
    pub first_on: BTreeSet<u32>,
    pub sample: Divergence,
    /// Runs of consecutive sampled ticks it shows in (the first 64).
    pub spans: Vec<(u32, u32)>,
}

impl Cause {
    /// Only ever on things that already differed: a knock-on of an earlier
    /// cause. A kingdom or world field has no thing, so it is never one.
    pub fn knock_on(&self) -> bool {
        !self.things.is_empty() && self.first_on.is_empty()
    }
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Lane {
    /// Ticks compared before (and including) the first divergence.
    pub ticks_compared: u32,
    pub first: Option<Divergence>,
    /// Every differing field at the first divergent tick (at most 12).
    pub at_first_tick: Vec<Divergence>,
    pub differing_at_first_tick: usize,
    /// Sampled ticks with any difference in this lane.
    pub ticks_differing: u32,
    /// Every difference to the end of the film, by signature.
    pub causes: BTreeMap<String, Cause>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StateComparison {
    pub lanes: BTreeMap<&'static str, Lane>,
    pub ticks_observed: u32,
    /// The sampled tick before this one.
    pub previous_tick: u32,
    /// Sampled tick each thing first differed at, in a per-thing lane.
    pub thing_first: BTreeMap<u32, u32>,
    /// Native things never compared because aa-play models them apart.
    pub trailers_seen: BTreeSet<u32>,
    /// Last sampled positions, to tell a mover from a stander.
    #[serde(skip)]
    previous_native: BTreeMap<u32, [u32; 2]>,
    #[serde(skip)]
    previous_ours: BTreeMap<u32, [u32; 2]>,
}

/// Put aa-play back on the native state after a sampled tick. The replay then
/// shows what each sampled interval alone gets wrong; what still differs is
/// independent of these lanes' earlier differences. A diagnostic only: it
/// writes the public fields, so internal state (Mover speed and path, Mind
/// state, queues) stays aa-play's own.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Forcing {
    /// GWorld+D4 into `World::native_rng`.
    pub rng: bool,
    /// Kingdom +1C4 into `Books::stockpiled`.
    pub economy: bool,
    /// Position and direction into `Actor` and `Organization::anchor`.
    pub movement: bool,
    /// Body +10 into `Actor::health`.
    pub health: bool,
}

/// aa-play's thing for a native GTID, through the pairing when there is one.
fn ours_of(world: &World, pairing: Option<&Pairing>, gtid: u32) -> Option<Thing> {
    match pairing {
        Some(p) => world.ids.thing_at_slot(p.ours(gtid)? as u16),
        None => world.ids.thing(gtid),
    }
}

/// Apply `forcing` with the native snapshot of the tick just compared.
pub fn force(world: &mut World, native: &NativeTick, forcing: Forcing, pairing: Option<&Pairing>) {
    if forcing.rng {
        world.native_rng.state = native.rng;
    }
    if forcing.economy {
        for k in &native.kingdoms {
            let Some(ours) = aa_play::ids::kingdom_by_index(world, k.index) else {
                continue;
            };
            let n = world.data.resources.len();
            let b = world.books.kingdom_mut(ours, n);
            for (i, &bits) in k.stockpiled.iter().enumerate() {
                if let Some(v) = b.stockpiled.get_mut(i) {
                    *v = f32::from_bits(bits);
                }
            }
        }
    }
    if !forcing.movement && !forcing.health {
        return;
    }
    for (gtid, n) in &native.actors {
        match ours_of(world, pairing, *gtid) {
            Some(Thing::Actor(a)) => {
                let Some(actor) = world.actors.get_mut(a as usize) else {
                    continue;
                };
                if actor.removed {
                    continue;
                }
                if forcing.movement {
                    actor.pos.x = f32::from_bits(n.pos[0]);
                    actor.pos.y = f32::from_bits(n.pos[1]);
                    actor.pos.z = f32::from_bits(n.pos[2]);
                    actor.facing = f32::from_bits(n.dir[1]).atan2(f32::from_bits(n.dir[0]));
                }
                if forcing.health
                    && let Some(h) = n.health
                {
                    actor.health = f32::from_bits(h[0]);
                }
            }
            Some(Thing::Org(o)) if forcing.movement => {
                if let Some(org) = world.orgs.get_mut(o as usize) {
                    org.anchor.x = f32::from_bits(n.pos[0]);
                    org.anchor.y = f32::from_bits(n.pos[1]);
                }
            }
            _ => {}
        }
    }
}

fn f(bits: u32) -> String {
    format!("{} ({bits:#010x})", f32::from_bits(bits))
}

/// The kind of thing, for a signature.
fn family(world: &World, thing: Thing) -> &'static str {
    let def = match thing {
        Thing::Actor(a) => world.actors.get(a as usize).map(|x| x.def),
        Thing::Org(o) => world.orgs.get(o as usize).map(|x| x.def),
        Thing::Trailer(_) => return "trailer",
        _ => return "other",
    };
    use aa_play::data::DefClass::*;
    match def.and_then(|d| world.data.def(d).class) {
        Some(Building) => "building",
        Some(Truck) => "truck",
        Some(Organization) => "organization",
        Some(Member) => "member",
        Some(Other) | None => "other",
    }
}

/// How far apart two points are, as a coarse class.
fn distance_class(a: [u32; 2], b: [f32; 2]) -> &'static str {
    let (ax, ay) = (f32::from_bits(a[0]), f32::from_bits(a[1]));
    let d = ((ax - b[0]).powi(2) + (ay - b[1]).powi(2)).sqrt();
    if d < 0.001 {
        "< 1 mm"
    } else if d < 0.1 {
        "< 10 cm"
    } else if d < 1.0 {
        "< 1 m"
    } else if d < 10.0 {
        "< 10 m"
    } else {
        ">= 10 m"
    }
}

struct Collector<'a> {
    tick: u32,
    names: &'a BTreeMap<u32, String>,
    pairing: Option<&'a Pairing>,
    previous_native: &'a BTreeMap<u32, [u32; 2]>,
    previous_ours: &'a BTreeMap<u32, [u32; 2]>,
    found: BTreeMap<&'static str, Vec<Divergence>>,
}
impl Collector<'_> {
    fn push(
        &mut self,
        lane: &'static str,
        gtid: Option<u32>,
        field: &str,
        cause: String,
        native: String,
        ours: String,
    ) {
        let name = gtid
            .and_then(|g| self.names.get(&g).cloned())
            .unwrap_or_default();
        self.found.entry(lane).or_default().push(Divergence {
            tick: self.tick,
            gtid,
            name,
            field: field.to_owned(),
            native,
            ours,
            cause,
        });
    }
    fn bits(
        &mut self,
        lane: &'static str,
        gtid: Option<u32>,
        field: &str,
        cause: impl FnOnce(u32, u32) -> String,
        native: u32,
        ours: u32,
    ) {
        if native != ours {
            let c = cause(native, ours);
            self.push(lane, gtid, field, c, f(native), f(ours));
        }
    }
}

/// (radius bits, tile x, tile y, flags) of zone `slot` in aa-play's aa-zones engine.
fn zone_of(world: &World, thing: Thing, slot: usize) -> Option<(u32, i32, i32, u16)> {
    let engine = world.fog.engine.as_ref()?;
    let s = match thing {
        Thing::Actor(a) => engine.parts.actors.get(a as usize)?,
        Thing::Org(o) => engine.parts.orgs.get(o as usize)?,
        _ => return None,
    };
    s.zones
        .slots
        .get(slot)
        .copied()
        .flatten()
        .map(|z| (z.radius_bits, z.x, z.y, z.flags))
}

fn detection_radius(world: &World, thing: Thing) -> Option<u32> {
    let engine = world.fog.engine.as_ref()?;
    let s = match thing {
        Thing::Actor(a) => engine.parts.actors.get(a as usize)?,
        Thing::Org(o) => engine.parts.orgs.get(o as usize)?,
        _ => return None,
    };
    s.comp.map(|c| c.zod_radius_bits)
}

/// A native zone with the radius -1.0 is an inactive one.
fn zone_active(radius_bits: u32) -> bool {
    f32::from_bits(radius_bits) >= 0.0
}

#[allow(clippy::too_many_arguments)]
fn compare_zone(
    c: &mut Collector,
    lane: &'static str,
    gtid: u32,
    slot: u32,
    who: &str,
    position_differs: bool,
    native: Option<&NativeZone>,
    ours: Option<(u32, i32, i32, u16)>,
) {
    let field = format!("zone {slot}");
    let show_native = |n: &NativeZone| {
        format!(
            "radius {} tile ({}, {}) flags {:#x}",
            f(n.radius),
            n.x,
            n.y,
            n.flags
        )
    };
    let show_ours = |o: (u32, i32, i32, u16)| {
        format!("radius {} tile ({}, {}) flags {:#x}", f(o.0), o.1, o.2, o.3)
    };
    match (native, ours) {
        (None, None) => {}
        (Some(n), None) => c.push(
            lane,
            Some(gtid),
            &field,
            format!(
                "{who} zone {slot}: native has it ({}), aa-play none",
                if zone_active(n.radius) {
                    "active"
                } else {
                    "inactive"
                }
            ),
            show_native(n),
            "none".into(),
        ),
        (None, Some(o)) => c.push(
            lane,
            Some(gtid),
            &field,
            format!("{who} zone {slot}: aa-play has it, native none"),
            "none".into(),
            show_ours(o),
        ),
        (Some(n), Some(o)) => {
            if (n.radius, n.x, n.y, n.flags) != o {
                let mut what = Vec::new();
                if n.radius != o.0 {
                    what.push(if zone_active(n.radius) != zone_active(o.0) {
                        "active differs"
                    } else {
                        "radius"
                    });
                }
                if (n.x, n.y) != (o.1, o.2) {
                    what.push(if position_differs {
                        "tile (the thing's position differs too)"
                    } else {
                        "tile (same position)"
                    });
                }
                if n.flags != o.3 {
                    what.push("flags");
                }
                c.push(
                    lane,
                    Some(gtid),
                    &field,
                    format!("{who} zone {slot}: {}", what.join(", ")),
                    show_native(n),
                    show_ours(o),
                );
            }
        }
    }
}

impl StateComparison {
    pub fn new() -> Self {
        let mut lanes = BTreeMap::new();
        for l in LANES {
            lanes.insert(l, Lane::default());
        }
        Self {
            lanes,
            ..Default::default()
        }
    }

    /// All lanes have diverged (the first-divergence report is complete).
    pub fn done(&self) -> bool {
        self.lanes.values().all(|l| l.first.is_some())
    }

    pub fn observe(
        &mut self,
        tick: u32,
        world: &World,
        trace: &NativeTrace,
        pairing: Option<&Pairing>,
    ) {
        let Some(native) = trace.ticks.get(&tick) else {
            return;
        };
        self.ticks_observed += 1;
        let mut c = Collector {
            tick,
            names: &trace.names,
            pairing,
            previous_native: &self.previous_native,
            previous_ours: &self.previous_ours,
            found: BTreeMap::new(),
        };
        compare_tick(&mut c, native, world);
        let found = c.found;
        // The things that differ for the first time now, in a per-thing lane.
        let mut newly: BTreeSet<u32> = BTreeSet::new();
        for (lane, list) in &found {
            if *lane == "rng" || *lane == "economy" {
                continue;
            }
            for d in list {
                if let Some(g) = d.gtid
                    && !self.thing_first.contains_key(&g)
                {
                    newly.insert(g);
                }
            }
        }
        for g in &newly {
            self.thing_first.insert(*g, tick);
        }
        for (name, lane) in self.lanes.iter_mut() {
            let list = found.get(name).map_or(&[][..], Vec::as_slice);
            if lane.first.is_none() {
                lane.ticks_compared += 1;
                if !list.is_empty() {
                    lane.first = Some(list[0].clone());
                    lane.differing_at_first_tick = list.len();
                    lane.at_first_tick = list.iter().take(12).cloned().collect();
                }
            }
            if !list.is_empty() {
                lane.ticks_differing += 1;
            }
            let mut seen_now: BTreeSet<&str> = BTreeSet::new();
            for d in list {
                let cause = lane.causes.entry(d.cause.clone()).or_insert_with(|| Cause {
                    lane: name,
                    signature: d.cause.clone(),
                    first_tick: tick,
                    last_tick: tick,
                    ticks: 0,
                    fields: 0,
                    things: BTreeSet::new(),
                    first_on: BTreeSet::new(),
                    sample: d.clone(),
                    spans: Vec::new(),
                });
                cause.last_tick = tick;
                cause.fields += 1;
                if seen_now.insert(d.cause.as_str()) {
                    cause.ticks += 1;
                    let continues = cause
                        .spans
                        .last()
                        .is_some_and(|s| s.1 == self.previous_tick);
                    if continues {
                        if let Some(s) = cause.spans.last_mut() {
                            s.1 = tick;
                        }
                    } else if cause.spans.len() < 64 {
                        cause.spans.push((tick, tick));
                    }
                }
                if let Some(g) = d.gtid {
                    cause.things.insert(g);
                    if newly.contains(&g) {
                        cause.first_on.insert(g);
                    }
                }
            }
        }
    }

    /// Remember the sampled positions for the next tick's motion classes
    /// (call after any forcing, so "moved" means moved since the forcing).
    /// It closes the tick: call once per step, after [`Self::observe`].
    pub fn remember(
        &mut self,
        tick: u32,
        world: &World,
        trace: &NativeTrace,
        pairing: Option<&Pairing>,
    ) {
        let Some(native) = trace.ticks.get(&tick) else {
            return;
        };
        self.previous_tick = tick;
        self.previous_native = native
            .actors
            .iter()
            .map(|(g, a)| (*g, [a.pos[0], a.pos[1]]))
            .collect();
        self.previous_ours = native
            .actors
            .keys()
            .filter_map(|g| match ours_of(world, pairing, *g) {
                Some(Thing::Actor(a)) => world
                    .actors
                    .get(a as usize)
                    .map(|x| (*g, [x.pos.x.to_bits(), x.pos.y.to_bits()])),
                Some(Thing::Org(o)) => world
                    .orgs
                    .get(o as usize)
                    .map(|x| (*g, [x.anchor.x.to_bits(), x.anchor.y.to_bits()])),
                _ => None,
            })
            .collect();
        for g in native.actors.keys() {
            if matches!(ours_of(world, pairing, *g), Some(Thing::Trailer(_))) {
                self.trailers_seen.insert(*g);
            }
        }
    }

    /// Every cause of every lane, oldest first.
    pub fn causes(&self) -> Vec<&Cause> {
        let mut all: Vec<&Cause> = self
            .lanes
            .values()
            .flat_map(|l| l.causes.values())
            .collect();
        all.sort_by_key(|c| (c.first_tick, c.lane, c.signature.clone()));
        all
    }
}

fn def_name(world: &World, thing: Thing) -> String {
    match thing {
        Thing::Actor(_) | Thing::Org(_) | Thing::Trailer(_) => thing_definition(world, thing),
        other => format!("{other:?}"),
    }
}

fn compare_tick(c: &mut Collector, native: &NativeTick, world: &World) {
    c.bits(
        "rng",
        None,
        "GWorld+D4",
        |_, _| "GWorld+D4 state".into(),
        native.rng,
        world.native_rng.state,
    );
    // existence: the live ids of actors, organizations and trailers (as the
    // native GTID they pair with, or their own when unpaired).
    let mut ours: BTreeMap<u32, Thing> = BTreeMap::new();
    let mut unpaired: Vec<(u32, Thing)> = Vec::new();
    for (g, t) in world.ids.iter() {
        let live = match t {
            Thing::Actor(a) => world.actors.get(a as usize).is_some_and(|x| !x.removed),
            Thing::Org(o) => world.orgs.get(o as usize).is_some_and(|x| x.alive),
            Thing::Trailer(_) => true,
            _ => false,
        };
        if !live {
            continue;
        }
        match c.pairing {
            Some(p) => match p.native(g.raw()) {
                Some(n) => {
                    ours.insert(n, t);
                }
                None => unpaired.push((g.raw(), t)),
            },
            None => {
                ours.insert(g.raw(), t);
            }
        }
    }
    // Native GActors aa-play keeps as map scenery (roads, start_loc ...:
    // `Thing::Placed` under the same saved id) are not simulated by aa-play
    // and are left out.
    let theirs: BTreeSet<u32> = native
        .actors
        .keys()
        .copied()
        .filter(|g| !matches!(ours_of(world, c.pairing, *g), Some(Thing::Placed(_))))
        .collect();
    for g in theirs.iter().filter(|g| !ours.contains_key(g)) {
        let what = match ours_of(world, c.pairing, *g) {
            None => "absent".to_string(),
            Some(t) => format!("{t:?}"),
        };
        let name = c.names.get(g).cloned().unwrap_or_default();
        let cause = format!(
            "missing {name}{}",
            if what == "absent" {
                ""
            } else {
                " (dead or removed here)"
            }
        );
        c.push("existence", Some(*g), "live", cause, "alive".into(), what);
    }
    for (g, t) in ours.iter().filter(|(g, _)| !theirs.contains(g)) {
        let name = def_name(world, *t);
        c.push(
            "existence",
            Some(*g),
            "live",
            format!("extra {name}"),
            "absent".into(),
            format!("{t:?}"),
        );
    }
    for (g, t) in &unpaired {
        let name = def_name(world, *t);
        c.push(
            "existence",
            Some(*g),
            "live",
            format!("extra {name} (no native counterpart)"),
            "absent".into(),
            format!("{t:?}"),
        );
    }
    // economy
    for k in &native.kingdoms {
        let Some(ours) = aa_play::ids::kingdom_by_index(world, k.index) else {
            continue;
        };
        let Some(b) = world.books.kingdom(ours) else {
            continue;
        };
        let lists: [(&str, &Vec<u32>, &Vec<f32>); 4] = [
            ("stockpiled +1C4", &k.stockpiled, &b.stockpiled),
            ("provided +1A0", &k.provided, &b.raw_provided),
            ("effective provided +1AC", &k.effective, &b.provided),
            ("consumed +1B8", &k.consumed, &b.consumed),
        ];
        for (field, n, o) in lists {
            for (i, &bits) in n.iter().enumerate() {
                let ob = o.get(i).map_or(u32::MAX, |v| v.to_bits());
                if bits != ob {
                    let resource = world.data.resources.get(i).map_or("?", |r| r.id.as_str());
                    c.push(
                        "economy",
                        Some(k.gtid),
                        &format!("kingdom {} {field}[{i} {resource}]", k.index),
                        format!("{field} {resource}"),
                        f(bits),
                        f(ob),
                    );
                }
            }
        }
        c.bits(
            "economy",
            Some(k.gtid),
            "shortage +1F0",
            |_, _| "shortage +1F0".into(),
            k.shortage,
            b.shortage_ratio.to_bits(),
        );
    }
    // per thing
    for (gtid, n) in &native.actors {
        let Some(thing) = ours_of(world, c.pairing, *gtid) else {
            continue;
        };
        compare_thing(c, *gtid, n, thing, world);
    }
}

fn motion_class(moved_native: bool, moved_ours: bool) -> &'static str {
    match (moved_native, moved_ours) {
        (true, false) => "native moves, aa-play stands",
        (false, true) => "aa-play moves, native stands",
        (true, true) => "both move",
        (false, false) => "both stand",
    }
}

fn compare_thing(c: &mut Collector, gtid: u32, n: &NativeActor, thing: Thing, world: &World) {
    let g = Some(gtid);
    // The same GTID must name the same kind of thing (an organization has an
    // Organization component natively) and the same definition.
    let native_org = n.organization.is_some();
    let ours_org = matches!(thing, Thing::Org(_));
    if matches!(thing, Thing::Actor(_) | Thing::Org(_)) && native_org != ours_org {
        c.push(
            "existence",
            g,
            "kind",
            format!(
                "id is {} natively, {} here",
                if native_org {
                    "an organization"
                } else {
                    "an actor"
                },
                if ours_org {
                    "an organization"
                } else {
                    "an actor"
                }
            ),
            if native_org {
                "organization".into()
            } else {
                "actor".into()
            },
            format!("{thing:?}"),
        );
        return;
    }
    if !matches!(thing, Thing::Actor(_) | Thing::Org(_)) {
        return;
    }
    let who = family(world, thing);
    let native_name = c.names.get(&gtid).cloned().unwrap_or_default();
    // The sampler names a thing once, by its definition then: an in-place
    // unpack keeps the GTID and changes the definition, so only a truck (or
    // its building) may differ from the name.
    let ours_name = def_name(world, thing);
    if !native_name.is_empty() && !native_name.eq_ignore_ascii_case(&ours_name) && who != "building"
    {
        c.push(
            "existence",
            g,
            "definition",
            format!("id names {native_name} natively, {ours_name} here"),
            native_name.clone(),
            ours_name.clone(),
        );
    }
    let ours_pos = match thing {
        Thing::Actor(a) => world.actor(a).map(|x| [x.pos.x, x.pos.y]),
        Thing::Org(o) => world.orgs.get(o as usize).map(|x| [x.anchor.x, x.anchor.y]),
        _ => None,
    };
    let Some(ours_pos) = ours_pos else { return };
    let ours_bits = [ours_pos[0].to_bits(), ours_pos[1].to_bits()];
    let native_xy = [n.pos[0], n.pos[1]];
    let position_differs = native_xy != ours_bits;
    let moved_native = c
        .previous_native
        .get(&gtid)
        .is_some_and(|p| *p != native_xy);
    let moved_ours = c.previous_ours.get(&gtid).is_some_and(|p| *p != ours_bits);
    if position_differs {
        let field = if ours_org {
            "organization position / anchor"
        } else {
            "position"
        };
        c.push(
            "movement",
            g,
            field,
            format!(
                "{who} position: {}, {}",
                motion_class(moved_native, moved_ours),
                distance_class(native_xy, ours_pos)
            ),
            format!("{}, {}", f(n.pos[0]), f(n.pos[1])),
            format!("{}, {}", f(ours_bits[0]), f(ours_bits[1])),
        );
    }
    match thing {
        Thing::Actor(a) => {
            let Some(actor) = world.actor(a) else { return };
            if !position_differs {
                c.bits(
                    "movement",
                    g,
                    "height z",
                    |_, _| format!("{who} height z (same x, y)"),
                    n.pos[2],
                    actor.pos.z.to_bits(),
                );
            }
            let native_angle = f32::from_bits(n.dir[1]).atan2(f32::from_bits(n.dir[0]));
            let mut d = (native_angle - actor.facing).abs() % std::f32::consts::TAU;
            if d > std::f32::consts::PI {
                d = std::f32::consts::TAU - d;
            }
            if d > 1e-4 {
                c.push(
                    "movement",
                    g,
                    "facing (rad)",
                    format!(
                        "{who} facing{}",
                        if position_differs {
                            " (position differs too)"
                        } else {
                            " (same position)"
                        }
                    ),
                    format!("{native_angle}"),
                    format!("{}", actor.facing),
                );
            }
            if let Some(m) = n.mover {
                c.bits(
                    "movement",
                    g,
                    "mover speed +20",
                    |nb, ob| {
                        let (nz, oz) = (f32::from_bits(nb) == 0.0, f32::from_bits(ob) == 0.0);
                        format!(
                            "{who} speed +20: {}",
                            match (nz, oz) {
                                (true, false) => "native stopped, aa-play moving",
                                (false, true) => "native moving, aa-play stopped",
                                _ => "both moving, values differ",
                            }
                        )
                    },
                    m.speed,
                    actor.motion.speed.to_bits(),
                );
            }
            let dead_native = n.flags & 1 != 0;
            if dead_native == actor.alive {
                c.push(
                    "combat",
                    g,
                    "dead +8 bit 0",
                    format!(
                        "{who} {}",
                        if dead_native {
                            "dead natively, alive here"
                        } else {
                            "alive natively, dead here"
                        }
                    ),
                    n.flags.to_string(),
                    actor.alive.to_string(),
                );
            }
            if let Some(h) = n.health {
                let max = aa_play::production::max_health(world, actor.kingdom, actor.def);
                c.bits(
                    "combat",
                    g,
                    "health",
                    |nb, ob| {
                        let (nv, ov) = (f32::from_bits(nb), f32::from_bits(ob));
                        format!(
                            "{who} health: {}",
                            if nv < ov {
                                "native lost more"
                            } else {
                                "aa-play lost more"
                            }
                        )
                    },
                    h[0],
                    actor.health.to_bits(),
                );
                c.bits(
                    "combat",
                    g,
                    "health maximum +14",
                    |_, _| format!("{who} health maximum +14"),
                    h[1],
                    max.to_bits(),
                );
            }
            let books = world.books.actor(a);
            if let Some(q) = n.queue {
                let (count, deadline, paused) = books.map_or((0, 0, u32::MAX), |b| {
                    (
                        b.queue.items.len() as u32,
                        b.queue.deadline.to_bits(),
                        b.queue.paused_at.map_or((-1.0f32).to_bits(), f32::to_bits),
                    )
                });
                if q.count > 0 || count > 0 {
                    if q.count != count {
                        c.push(
                            "production",
                            g,
                            "queue length",
                            format!(
                                "queue length: {}",
                                if q.count > count {
                                    "native has more"
                                } else {
                                    "aa-play has more"
                                }
                            ),
                            q.count.to_string(),
                            count.to_string(),
                        );
                    } else {
                        c.bits(
                            "production",
                            g,
                            "queue deadline +20",
                            |_, _| "queue deadline +20 (same length)".into(),
                            q.deadline,
                            deadline,
                        );
                        c.bits(
                            "production",
                            g,
                            "queue paused +24",
                            |_, _| "queue paused +24 (same length)".into(),
                            q.paused,
                            paused,
                        );
                    }
                }
            }
            if let Some(s) = n.supply {
                c.bits(
                    "supply",
                    g,
                    "supply stock +18",
                    |_, _| format!("{who} supply stock +18"),
                    s[0],
                    books.map_or(u32::MAX, |b| b.supply_stock.to_bits()),
                );
            }
        }
        Thing::Org(o) => {
            let Some(org) = world.orgs.get(o as usize) else {
                return;
            };
            if let Some(no) = &n.organization {
                // Organization +28 holds one entry per layout slot (null when
                // empty); aa-play keeps the live members.
                let native_members = no.members.iter().filter(|&&p| p != 0).count();
                let members = org
                    .members
                    .iter()
                    .filter(|&&m| world.actor(m).is_some_and(|a| a.alive))
                    .count();
                if native_members != members {
                    c.push(
                        "existence",
                        g,
                        "organization members",
                        format!(
                            "organization members: {}",
                            if native_members > members {
                                "native has more"
                            } else {
                                "aa-play has more"
                            }
                        ),
                        native_members.to_string(),
                        members.to_string(),
                    );
                }
            }
        }
        _ => return,
    }
    // zones (actors and organizations)
    let zone = |slot: u32| n.zones.iter().find(|z| z.slot == slot);
    for (lane, slot) in [
        ("supply", 0u32),
        ("fog", 1),
        ("fog", 4),
        ("fog", 6),
        ("fog", 8),
    ] {
        compare_zone(
            c,
            lane,
            gtid,
            slot,
            who,
            position_differs,
            zone(slot),
            zone_of(world, thing, slot as usize),
        );
    }
    if let Some(d) = n.detection {
        c.bits(
            "fog",
            g,
            "detection ZOD radius +14",
            |_, _| format!("{who} detection ZOD radius +14"),
            d[1],
            detection_radius(world, thing).unwrap_or(u32::MAX),
        );
    }
}

/// One group of native getters over the film.
#[derive(Clone, Debug, Default, Serialize)]
pub struct GetterGroup {
    pub native: u32,
    pub first_tick: u32,
    pub last_tick: u32,
    /// Native getters of this group in ticks whose whole getter list aa-play
    /// reproduced.
    pub in_equal_ticks: u32,
    /// Ticks whose first difference falls on a getter of this group (aa-play
    /// has another state there, or stops before it).
    pub first_difference_in: u32,
    /// The first such tick.
    pub first_difference_tick: Option<u32>,
}

/// Native getter list of one compared tick against aa-play's.
#[derive(Clone, Debug, Serialize)]
pub struct TickGetters {
    pub tick: u32,
    pub native: Vec<String>,
    pub native_states: Vec<u32>,
    pub ours_states: Vec<u32>,
}

/// Every world-RNG getter of the battle, native against aa-play.
#[derive(Clone, Debug, Default, Serialize)]
pub struct GetterCensus {
    pub native_total: u32,
    pub ours_total: u32,
    /// Every getter draws once, so without forcing both sides walk the same
    /// LCG chain and the k-th getters have the same state whatever their tick:
    /// the first state that differs only shows a forced state or an extra
    /// draw. (index, native tick, native state, our state).
    pub first_stream_difference: Option<(u32, u32, u32, u32)>,
    /// (index, native tick, our tick): the first getter that ran in another
    /// tick (our tick is the replay step, native's the GWorld counter, equal
    /// for a getter of an update).
    pub first_tick_difference: Option<(u32, u32, u32)>,
    /// Compared windows (one tick up to tick 1200, then the sampler's eight).
    pub windows: u32,
    pub windows_equal: u32,
    /// Windows where aa-play logged fewer / more getters than native.
    pub windows_short: u32,
    pub windows_long: u32,
    pub by_group: BTreeMap<String, GetterGroup>,
    /// The first windows that differ.
    pub first_windows: Vec<TickGetters>,
}

/// Compare the whole film's getters: as one stream, and window by window
/// between the ticks the native trace sampled (a window is the replay's steps
/// after one sampled tick up to the next: the trace's order keeps the decrees
/// of a record with the update that follows them). With [`Forcing::rng`] each
/// window starts from the same state, so its first difference is exact.
pub fn getter_census(world: &World, trace: &NativeTrace, last_tick: u32) -> GetterCensus {
    use aa_play::trace::SKind;
    let mut out = GetterCensus::default();
    let sampled: Vec<u32> = trace
        .ticks
        .keys()
        .copied()
        .filter(|t| *t <= last_tick)
        .collect();
    let last_window = sampled.last().copied().unwrap_or(0);
    let native: Vec<_> = trace
        .getters
        .iter()
        .filter(|g| g.tick >= 1 && g.after < last_window)
        .collect();
    let ours: Vec<(u32, u32)> = world
        .trace
        .entries()
        .filter(|(t, k, _)| *t <= last_window && matches!(k, SKind::RngGetter))
        .map(|(t, _, v)| (t, v))
        .collect();
    out.native_total = native.len() as u32;
    out.ours_total = ours.len() as u32;
    for (i, g) in native.iter().enumerate() {
        match ours.get(i) {
            Some(&(_, s)) if s == g.state => {}
            other => {
                out.first_stream_difference =
                    Some((i as u32, g.tick, g.state, other.map_or(0, |o| o.1)));
                break;
            }
        }
    }
    for (i, g) in native.iter().enumerate() {
        let decree = g.chain.iter().any(|&a| a == 0x61F50E || a == 0x4ED687);
        let want = if decree { g.tick + 1 } else { g.tick };
        let got = ours.get(i).map_or(0, |o| o.0);
        if want != got {
            out.first_tick_difference = Some((i as u32, g.tick, got));
            break;
        }
    }
    let mut by_window: BTreeMap<u32, Vec<&crate::native_trace::NativeGetter>> = BTreeMap::new();
    for g in &native {
        let e = out
            .by_group
            .entry(getter_group(g.site, &g.chain))
            .or_default();
        if e.native == 0 {
            e.first_tick = g.tick;
        }
        e.native += 1;
        e.last_tick = g.tick;
        by_window.entry(g.after).or_default().push(g);
    }
    let mut start = 0u32;
    let mut cursor = 0usize;
    for &end in &sampled {
        let empty = Vec::new();
        let native_list = by_window.get(&start).unwrap_or(&empty);
        let mut ours_list = Vec::new();
        while let Some(&(t, s)) = ours.get(cursor) {
            if t > end {
                break;
            }
            ours_list.push(s);
            cursor += 1;
        }
        out.windows += 1;
        let equal = native_list.len() == ours_list.len()
            && native_list
                .iter()
                .zip(&ours_list)
                .all(|(g, s)| g.state == *s);
        if equal {
            out.windows_equal += 1;
            for g in native_list {
                if let Some(e) = out.by_group.get_mut(&getter_group(g.site, &g.chain)) {
                    e.in_equal_ticks += 1;
                }
            }
        } else {
            if ours_list.len() < native_list.len() {
                out.windows_short += 1;
            } else if ours_list.len() > native_list.len() {
                out.windows_long += 1;
            }
            let at = native_list
                .iter()
                .zip(&ours_list)
                .position(|(g, s)| g.state != *s)
                .unwrap_or(native_list.len().min(ours_list.len()));
            let group = native_list.get(at).map_or_else(
                || "(aa-play draws where native stops)".to_owned(),
                |g| getter_group(g.site, &g.chain),
            );
            let e = out.by_group.entry(group).or_default();
            e.first_difference_in += 1;
            e.first_difference_tick.get_or_insert(end);
            if out.first_windows.len() < 80 {
                out.first_windows.push(TickGetters {
                    tick: end,
                    native: native_list
                        .iter()
                        .map(|g| getter_group(g.site, &g.chain))
                        .collect(),
                    native_states: native_list.iter().map(|g| g.state).collect(),
                    ours_states: ours_list,
                });
            }
        }
        start = end;
    }
    out
}
