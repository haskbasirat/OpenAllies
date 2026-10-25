//! Field-by-field comparison of aa-play's world with the native sampler's
//! snapshots ([`crate::native_trace`]) of the same tick, lane by lane.
//!
//! Call [`StateComparison::observe`] after every replay step (the hook of
//! `replay::run_observed`); ticks the native trace did not sample are
//! skipped. Each lane keeps its first differing tick and field:
//! * `rng`: GWorld+D4 against `World::native_rng`;
//! * `existence`: the live class-3 GTIDs against aa-play's live actors and
//!   organizations (`World::ids`);
//! * `economy`: Kingdom +1C4 stockpiled, +1A0 provided, +1AC effective
//!   provided, +1B8 consumed against `Books` `stockpiled`, `raw_provided`,
//!   `provided`, `consumed` (kingdoms matched by the +188 index);
//! * `production`: Structure queue count, deadline and paused time (-1.0
//!   while running) against `ActorBooks::queue`;
//! * `movement`: actor position (x, y), facing (direction angle) and Mover
//!   speed `+20` against `Actor::pos`, `Actor::facing` and `Motion::speed`;
//!   an organization's own position against `Organization::anchor`;
//! * `combat`: Body health `+10` against `Actor::health`;
//! * `supply`: Supply `+18` stock against `ActorBooks::supply_stock`, and zone
//!   slot 0 (ZOS) against the aa-zones engine's;
//! * `fog`: zone slots 1 (RZOD) and 6 (ZOD) and the Detection ZOD radius
//!   against the aa-zones engine's.
//!
//! Positions, health, speeds, stocks and zone fields are compared as exact
//! binary32 bits; the facing as an angle within 1e-4 rad (the native keeps a
//! direction vector).
use crate::native_trace::{NativeActor, NativeTick, NativeTrace, NativeZone};
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
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Lane {
    /// Ticks compared before (and including) the first divergence.
    pub ticks_compared: u32,
    pub first: Option<Divergence>,
    /// Every differing field at the first divergent tick (at most 12).
    pub at_first_tick: Vec<Divergence>,
    pub differing_at_first_tick: usize,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct StateComparison {
    pub lanes: BTreeMap<&'static str, Lane>,
    pub ticks_observed: u32,
}

fn f(bits: u32) -> String {
    format!("{} ({bits:#010x})", f32::from_bits(bits))
}

struct Collector<'a> {
    tick: u32,
    names: &'a BTreeMap<u32, String>,
    found: BTreeMap<&'static str, Vec<Divergence>>,
}
impl Collector<'_> {
    fn push(
        &mut self,
        lane: &'static str,
        gtid: Option<u32>,
        field: &str,
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
        });
    }
    fn bits(&mut self, lane: &'static str, gtid: Option<u32>, field: &str, native: u32, ours: u32) {
        if native != ours {
            self.push(lane, gtid, field, f(native), f(ours));
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

fn compare_zone(
    c: &mut Collector,
    lane: &'static str,
    gtid: u32,
    slot: u32,
    native: Option<&NativeZone>,
    ours: Option<(u32, i32, i32, u16)>,
) {
    let field = format!("zone {slot}");
    match (native, ours) {
        (None, None) => {}
        (Some(n), None) => c.push(
            lane,
            Some(gtid),
            &field,
            format!(
                "radius {} tile ({}, {}) flags {:#x}",
                f(n.radius),
                n.x,
                n.y,
                n.flags
            ),
            "none".into(),
        ),
        (None, Some(o)) => c.push(
            lane,
            Some(gtid),
            &field,
            "none".into(),
            format!("radius {} tile ({}, {}) flags {:#x}", f(o.0), o.1, o.2, o.3),
        ),
        (Some(n), Some(o)) => {
            if (n.radius, n.x, n.y, n.flags) != o {
                c.push(
                    lane,
                    Some(gtid),
                    &field,
                    format!(
                        "radius {} tile ({}, {}) flags {:#x}",
                        f(n.radius),
                        n.x,
                        n.y,
                        n.flags
                    ),
                    format!("radius {} tile ({}, {}) flags {:#x}", f(o.0), o.1, o.2, o.3),
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
            ticks_observed: 0,
        }
    }

    /// All lanes have diverged: nothing more to learn.
    pub fn done(&self) -> bool {
        self.lanes.values().all(|l| l.first.is_some())
    }

    pub fn observe(&mut self, tick: u32, world: &World, trace: &NativeTrace) {
        let Some(native) = trace.ticks.get(&tick) else {
            return;
        };
        self.ticks_observed += 1;
        let mut c = Collector {
            tick,
            names: &trace.names,
            found: BTreeMap::new(),
        };
        compare_tick(&mut c, native, world);
        for (name, lane) in self.lanes.iter_mut() {
            if lane.first.is_some() {
                continue;
            }
            lane.ticks_compared += 1;
            if let Some(found) = c.found.get(name)
                && !found.is_empty()
            {
                lane.first = Some(found[0].clone());
                lane.differing_at_first_tick = found.len();
                lane.at_first_tick = found.iter().take(12).cloned().collect();
            }
        }
    }
}

fn compare_tick(c: &mut Collector, native: &NativeTick, world: &World) {
    c.bits("rng", None, "GWorld+D4", native.rng, world.native_rng.state);
    // existence
    let mut ours: BTreeSet<u32> = BTreeSet::new();
    for a in world.actors.iter().filter(|a| !a.removed) {
        if let Some(g) = world.ids.actor(a.id) {
            ours.insert(g.raw());
        }
    }
    for o in world.orgs.iter().filter(|o| o.alive) {
        if let Some(g) = world.ids.org(o.id) {
            ours.insert(g.raw());
        }
    }
    // Native GActors aa-play keeps as map scenery (roads, start_loc ...:
    // `Thing::Placed` under the same saved id) are not simulated by aa-play
    // and are left out.
    let theirs: BTreeSet<u32> = native
        .actors
        .keys()
        .copied()
        .filter(|g| !matches!(world.ids.thing(*g), Some(Thing::Placed(_))))
        .collect();
    let missing: Vec<u32> = theirs.difference(&ours).copied().collect();
    let extra: Vec<u32> = ours.difference(&theirs).copied().collect();
    for g in missing.iter().take(6) {
        let ours = match world.ids.thing(*g) {
            None => "absent".to_string(),
            Some(t) => format!("{t:?}"),
        };
        c.push("existence", Some(*g), "live", "alive".into(), ours);
    }
    for g in extra.iter().take(6) {
        c.push(
            "existence",
            Some(*g),
            "live",
            "absent".into(),
            format!("{:?}", world.ids.thing(*g)),
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
                        f(bits),
                        f(ob),
                    );
                }
            }
        }
    }
    // per thing
    for (gtid, n) in &native.actors {
        let Some(thing) = world.ids.thing(*gtid) else {
            continue;
        };
        compare_thing(c, *gtid, n, thing, world);
    }
}

fn compare_thing(c: &mut Collector, gtid: u32, n: &NativeActor, thing: Thing, world: &World) {
    let g = Some(gtid);
    // The same GTID must name the same kind of thing (an organization has an
    // Organization component natively).
    let native_org = n.organization.is_some();
    let ours_org = matches!(thing, Thing::Org(_));
    if matches!(thing, Thing::Actor(_) | Thing::Org(_)) && native_org != ours_org {
        c.push(
            "existence",
            g,
            "kind",
            if native_org {
                "organization".into()
            } else {
                "actor".into()
            },
            format!("{thing:?}"),
        );
        return;
    }
    match thing {
        Thing::Actor(a) => {
            let Some(actor) = world.actor(a) else { return };
            c.bits("movement", g, "position x", n.pos[0], actor.pos.x.to_bits());
            c.bits("movement", g, "position y", n.pos[1], actor.pos.y.to_bits());
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
                    format!("{native_angle}"),
                    format!("{}", actor.facing),
                );
            }
            if let Some(m) = n.mover {
                c.bits(
                    "movement",
                    g,
                    "mover speed +20",
                    m.speed,
                    actor.motion.speed.to_bits(),
                );
            }
            if let Some(h) = n.health {
                c.bits("combat", g, "health", h[0], actor.health.to_bits());
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
                            q.count.to_string(),
                            count.to_string(),
                        );
                    } else {
                        c.bits("production", g, "queue deadline +20", q.deadline, deadline);
                        c.bits("production", g, "queue paused +24", q.paused, paused);
                    }
                }
            }
            if let Some(s) = n.supply {
                c.bits(
                    "supply",
                    g,
                    "supply stock +18",
                    s[0],
                    books.map_or(u32::MAX, |b| b.supply_stock.to_bits()),
                );
            }
        }
        Thing::Org(o) => {
            let Some(org) = world.orgs.get(o as usize) else {
                return;
            };
            c.bits(
                "movement",
                g,
                "organization position x / anchor",
                n.pos[0],
                org.anchor.x.to_bits(),
            );
            c.bits(
                "movement",
                g,
                "organization position y / anchor",
                n.pos[1],
                org.anchor.y.to_bits(),
            );
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
    compare_zone(c, "supply", gtid, 0, zone(0), zone_of(world, thing, 0));
    compare_zone(c, "fog", gtid, 1, zone(1), zone_of(world, thing, 1));
    compare_zone(c, "fog", gtid, 6, zone(6), zone_of(world, thing, 6));
    if let Some(d) = n.detection {
        c.bits(
            "fog",
            g,
            "detection ZOD radius +14",
            d[1],
            detection_radius(world, thing).unwrap_or(u32::MAX),
        );
    }
}
