//! The collision results of the element's Move on the exact ports: `0x534A40`,
//! `0x536AD0`, the low-path continuation `0x531520` and solver `0x53C170`
//! (`aa_navigation::uai_collision`, `low_path`), over the tile lists of [`space`] and
//! the element Mind's [`Host`]. Census cause M5.
//!
//! **Renditions:** `0x4F3C40`'s region and structure-crossing steps (no SAI regions
//! here, no walls) answer clear; `0x53CA30` (a destination moved around a blocking
//! object) answers no.
use super::element::Host;
use super::*;
use aa_navigation::low_path::{self, ActorPose, ActorShape, ContinuationWorld, LowPathSettings, LowPathWorld, Options, SpatialHit};
use aa_navigation::result_plan::{CollisionContext, Object};
use aa_navigation::uai_collision::{self as uc, UaiCollisionWorld};
use aa_navigation::MoveState;
use aa_numeric::Binary;
use aa_spatial::collision::{self, Descriptor, Shape};
use aa_spatial::geometry::Placement;
use aa_spatial::index::SpatialIndex;
use aa_spatial::index_world::CollisionView;
use aa_spatial::query::{self, PreparedQuery, QueryGeometry};

const PC: aa_numeric::Precision = PC53;
/// Objects of this bit are organization things; the others are index keys.
const ORG_OBJECT: u32 = 0x8000_0000;

fn low(what: impl std::fmt::Display) -> low_path::Error {
    low_path::Error::World(what.to_string())
}
fn key(word: u32) -> Option<u32> {
    (word != 0).then_some(word)
}

/// Descriptor words to the portable query (words 16 and 21 carry nothing portable).
pub(super) fn query_from_context(context: &CollisionContext) -> PreparedQuery {
    let w = &context.words;
    PreparedQuery {
        placement: Placement {
            shape: Shape { kind: w[0], radius_bits: w[1], bounds: [w[2], w[3], w[4], w[5]] },
            facing: [w[17], w[18]],
            position: [w[19], w[20]],
        },
        descriptor: Descriptor {
            source_definition: key(w[6]),
            source_actor: key(w[7]),
            ignore_actor: key(w[8]),
            ignore_definition: key(w[9]),
            collision_owner: key(w[10]),
            hostility_context: key(w[11]),
            ignore_all: w[12] & 0xff != 0,
            ignore_moving: (w[12] >> 8) & 0xff != 0,
            custom_filter: w[13] != 0,
            mask: w[15],
        },
        required_actor_flags: w[14],
    }
}

/// The portable query as descriptor words.
pub(super) fn context_from_query(query: &PreparedQuery) -> CollisionContext {
    let d = &query.descriptor;
    let s = &query.placement.shape;
    let mut w = [0u32; 22];
    w[0] = s.kind;
    w[1] = s.radius_bits;
    w[2..6].copy_from_slice(&s.bounds);
    w[6] = d.source_definition.unwrap_or(0);
    w[7] = d.source_actor.unwrap_or(0);
    w[8] = d.ignore_actor.unwrap_or(0);
    w[9] = d.ignore_definition.unwrap_or(0);
    w[10] = d.collision_owner.unwrap_or(0);
    w[11] = d.hostility_context.unwrap_or(0);
    w[12] = u32::from(d.ignore_all) | (u32::from(d.ignore_moving) << 8);
    w[13] = u32::from(d.custom_filter);
    w[14] = query.required_actor_flags;
    w[15] = d.mask;
    w[17] = query.placement.facing[0];
    w[18] = query.placement.facing[1];
    w[19] = query.placement.position[0];
    w[20] = query.placement.position[1];
    CollisionContext { words: w }
}

/// `aa_navigation::endpoint::World` of `0x53D340`: `0x53CE40` over the query.
pub(super) struct Endpoint<'a, 'p, 'q> {
    pub space: &'a mut space::Space,
    pub policy: &'p mut space::Policy<'q>,
    pub query: PreparedQuery,
}
impl aa_navigation::endpoint::World for Endpoint<'_, '_, '_> {
    fn acceptable(&mut self, _: [u32; 2], candidate: [u32; 2]) -> aa_navigation::endpoint::Result<bool> {
        let fail = |e: aa_spatial::Error| aa_navigation::endpoint::Error::World(e.to_string());
        let (x, y) = (f32::from_bits(candidate[0]), f32::from_bits(candidate[1]));
        let extent = self.space.geometry.extent_bits;
        if !(x >= 0.0 && y >= 0.0 && x < f32::from_bits(extent[0]) && y < f32::from_bits(extent[1])) {
            return Ok(false);
        }
        self.query.set_position(&self.space.index, candidate, &*self.policy).map_err(fail)?;
        let grid = self.space.index.grid().clone();
        let cell = [(x as i32) >> (grid.coordinate_shift & 31), (y as i32) >> (grid.coordinate_shift & 31)];
        if self.space.index.tile(cell).map_err(fail)?.flags20 & 1 != 0 {
            return Ok(false);
        }
        let blocked = CollisionView { index: &mut self.space.index, policy: &mut *self.policy }
            .blocked(cell, &mut self.query.descriptor)
            .map_err(|e| aa_navigation::endpoint::Error::World(e.to_string()))?;
        if blocked {
            return Ok(false);
        }
        // 0x4F3C40: past its bounds, 0x553BA0 when the mask has 0x300.
        if self.query.descriptor.mask & 0x300 != 0 {
            let hit = space::first_hit(self.space, self.policy, &self.query);
            return Ok(!hit.blocked());
        }
        Ok(true)
    }
    /// Rendition of `0x53CA30`.
    fn displace_blocked(&mut self, _: [u32; 2], _: &mut [u32; 2]) -> aa_navigation::endpoint::Result<bool> {
        Ok(false)
    }
}

/// `UaiCollisionWorld` for one result. The port calling this holds the Move's fields,
/// so what needs the Mind's whole payload is left in `after`, for when it returned.
pub(super) struct Uai<'u, 'w> {
    pub host: &'u mut Host<'w>,
    pub index: &'u mut SpatialIndex,
    pub geometry: &'u QueryGeometry,
    pub after: element::After,
}

impl Uai<'_, '_> {
    fn policy_parts(&mut self) -> (&mut SpatialIndex, space::Policy<'_>) {
        let (ms, world) = self.host.parts();
        (&mut *self.index, space::Policy { ms, world })
    }
    fn id(&self, object: Object) -> ActorId {
        space::key_actor(object.0)
    }
    fn radius(&self, id: ActorId, soft: bool) -> Binary {
        let (ms, world) = self.host.parts_ref();
        let _ = ms;
        let def = world.data.def(world.actors[id as usize].def);
        let bits = if soft { def.soft_shape[1] } else { def.blocking_words[1] };
        Binary::from_f32_bits(bits).unwrap_or(Binary::from_u32(0))
    }
    fn placement(d: &uc::Descriptor) -> low_path::Result<Placement> {
        let s = d.shape.ok_or_else(|| low("null shape"))?;
        Ok(Placement { shape: Shape { kind: s.kind, radius_bits: s.radius_bits, bounds: s.bounds_bits }, facing: d.facing_bits, position: d.position_bits })
    }
}

impl LowPathWorld for Uai<'_, '_> {
    /// `0x54F370`.
    fn tile_blocked(&mut self, tile: [i32; 2], context: &CollisionContext) -> low_path::Result<bool> {
        let mut descriptor = query_from_context(context).descriptor;
        let (index, mut policy) = self.policy_parts();
        CollisionView { index, policy: &mut policy }.blocked(tile, &mut descriptor).map_err(low)
    }
    /// `0x552250`.
    fn set_context_position(&mut self, context: &mut CollisionContext, position: [u32; 2]) -> low_path::Result<()> {
        let mut query = query_from_context(context);
        let (index, policy) = self.policy_parts();
        query.set_position(index, position, &policy).map_err(low)?;
        context.words[19] = query.placement.position[0];
        context.words[20] = query.placement.position[1];
        Ok(())
    }
    /// `0x553BA0`.
    fn first_hit(&mut self, context: &CollisionContext) -> low_path::Result<SpatialHit> {
        let query = query_from_context(context);
        let geometry = self.geometry;
        let (index, mut policy) = self.policy_parts();
        let hit = query::first_hit(index, geometry, &query, &mut policy).map_err(low)?;
        Ok(SpatialHit { kind: hit.kind, reference: hit.reference })
    }
    /// `0x553CE0`.
    fn sweep(&mut self, context: &CollisionContext, goal: [u32; 2]) -> low_path::Result<SpatialHit> {
        let mut query = query_from_context(context);
        let geometry = self.geometry;
        let (index, mut policy) = self.policy_parts();
        let hit = aa_spatial::sweep::sweep(index, geometry, &mut query, goal, &mut policy).map_err(low)?;
        Ok(SpatialHit { kind: hit.kind, reference: hit.reference })
    }
    /// `0x4F3C40`: 3 outside; 0 when the mask has no 0x300; 4 blocked by the first hit.
    /// Rendition: its region (1) and structure-crossing (2) steps.
    fn placement_result(&mut self, _: Option<Object>, context: &CollisionContext) -> low_path::Result<u32> {
        let query = query_from_context(context);
        let [x, y] = query.placement.position.map(f32::from_bits);
        let extent = self.geometry.extent_bits;
        if !(x >= 0.0 && y >= 0.0 && x < f32::from_bits(extent[0]) && y < f32::from_bits(extent[1])) {
            return Ok(3);
        }
        if query.descriptor.mask & 0x300 == 0 {
            return Ok(0);
        }
        Ok(if LowPathWorld::first_hit(self, context)?.kind != 0 { 4 } else { 0 })
    }
    fn resolve_actor(&mut self, reference: u32) -> Option<Object> {
        self.index.resolve_reference(reference).map(Object)
    }
    fn actor_virtual_b4(&mut self, _: Object) -> low_path::Result<bool> {
        Ok(true)
    }
    fn actor_word_90_present(&mut self, _: Object) -> low_path::Result<bool> {
        Ok(false)
    }
    /// Actor virtual +0x100: a thing inside a transport (not hosted).
    fn actor_virtual_100(&mut self, _: Object) -> low_path::Result<u32> {
        Ok(0)
    }
    /// `0x552340`.
    fn context_excludes(&mut self, context: &CollisionContext, actor: Object) -> low_path::Result<bool> {
        let mut descriptor = query_from_context(context).descriptor;
        let (index, mut policy) = self.policy_parts();
        collision::excluded(actor.0, &mut descriptor, &mut CollisionView { index, policy: &mut policy }).map_err(low)
    }
    fn actor_pose(&mut self, actor: Object) -> low_path::Result<ActorPose> {
        let m = self.index.member(actor.0).map_err(low)?;
        Ok(ActorPose { position_bits: m.actor.position, facing_bits: m.actor.facing })
    }
    /// Thing virtual +0: the soft shape's radius.
    fn source_virtual_0(&mut self, source: Object) -> low_path::Result<u32> {
        Ok(self.index.member(source.0).map_err(low)?.soft.radius_bits)
    }
    fn actor_shape(&mut self, actor: Object) -> low_path::Result<ActorShape> {
        let s = &self.index.member(actor.0).map_err(low)?.blocking;
        Ok(ActorShape { kind: s.kind, word1_bits: s.radius_bits, word5_bits: s.bounds[3] })
    }
    /// `0x4D67F0`.
    fn refresh_overlaps(&mut self, source: Object) -> low_path::Result<()> {
        let id = self.id(source);
        let (ms, world) = self.host.parts_mut();
        let mut entries = std::mem::take(&mut ms.actors[id as usize].overlaps);
        let now = world.clock.begin.0;
        let geometry = self.geometry;
        {
            let mut policy = space::Policy { ms: &*ms, world: &*world };
            aa_spatial::overlaps::refresh_overlaps(self.index, geometry, source.0, &mut entries, now, &mut |_, _| Ok(true), &mut policy).map_err(low)?;
        }
        ms.actors[id as usize].overlaps = entries;
        Ok(())
    }
}

impl ContinuationWorld for Uai<'_, '_> {
    /// `0x554350(actor, 0)`.
    fn collision_context(&mut self, _: &mut MoveState) -> low_path::Result<CollisionContext> {
        self.host.set_go_site_low();
        let id = self.host.actor_id();
        let (index, policy) = self.policy_parts();
        let query = aa_spatial::actor_context::prepare(index, space::actor_key(id), false, &policy).map_err(low)?;
        Ok(context_from_query(&query))
    }
    /// `0x4CE6E0`: an actor stands for itself here (organizations are no targets).
    fn replacement_actor(&mut self, object: Object, _: &mut MoveState) -> low_path::Result<Object> {
        Ok(object)
    }
    /// Mind +0x54: the Deploy's status.
    fn owner_word_54(&mut self, _: &mut MoveState) -> low_path::Result<u32> {
        Ok(self.host.deploy_status())
    }
    fn actor_position(&mut self, _: &mut MoveState) -> low_path::Result<[u32; 2]> {
        Ok(self.host.position_bits())
    }
    /// `0x52D660`.
    fn stop_action(&mut self, _: &mut MoveState) -> low_path::Result<()> {
        self.host.stop_move();
        Ok(())
    }
    /// `0x4CFC80`.
    fn actor_stop_animation(&mut self, _: &mut MoveState) -> low_path::Result<()> {
        self.host.deactivate_now();
        Ok(())
    }
    fn schedule(&mut self, delay: u32, _: &mut MoveState) -> low_path::Result<()> {
        self.host.schedule_now(delay);
        Ok(())
    }
    /// Mind +0x15C `0x52F260(1)`, for when the port returned.
    fn path_failed(&mut self, _: &mut MoveState) -> low_path::Result<()> {
        self.after = element::After::PathFailed;
        Ok(())
    }
    /// `0x5300A0`.
    fn action_go(&mut self, speed: u32, state: &mut MoveState) -> low_path::Result<()> {
        let after = self.host.action_go_on(state, speed).map_err(low)?;
        if after != element::After::None {
            self.after = after;
        }
        Ok(())
    }
}

impl UaiCollisionWorld for Uai<'_, '_> {
    fn state_actor(&mut self, _: &mut MoveState) -> low_path::Result<Object> {
        Ok(Object(space::actor_key(self.host.actor_id())))
    }
    fn actor_flags(&mut self, actor: Object) -> low_path::Result<u32> {
        Ok(self.index.member(actor.0).map_err(low)?.actor.flags)
    }
    fn actor_gtid(&mut self, actor: Object) -> low_path::Result<u32> {
        Ok(actor.0)
    }
    fn organization_thing(&mut self, actor: Object) -> low_path::Result<Option<Object>> {
        let id = self.id(actor);
        let (_, world) = self.host.parts_ref();
        Ok(world.actors[id as usize].org.filter(|&o| world.orgs.get(o as usize).is_some_and(|x| x.alive)).map(|o| Object(ORG_OBJECT | o)))
    }
    fn organization_component(&mut self, thing: Object) -> low_path::Result<Option<Object>> {
        Ok((thing.0 & ORG_OBJECT != 0).then_some(thing))
    }
    fn organization_leader(&mut self, component: Object) -> low_path::Result<Option<Object>> {
        let (ms, _) = self.host.parts_ref();
        Ok(ms.orgs[(component.0 & !ORG_OBJECT) as usize].leader.map(|l| Object(space::actor_key(l))))
    }
    fn thing_mind(&mut self, thing: Object) -> low_path::Result<Option<Object>> {
        Ok(Some(thing))
    }
    /// Mind +0x44, the Move's path, and its head's point: regiment `0x5247A0`, element
    /// `0x535E00` (the leader's, for its members).
    fn mind_virtual_44(&mut self, mind: Object) -> low_path::Result<uc::MindTarget> {
        let (ms, _) = self.host.parts_ref();
        if mind.0 & ORG_OBJECT == 0 {
            let head = ms.actors[space::key_actor(mind.0) as usize].payload.as_ref().and_then(|p| p.plan.movement.path.nodes.first().copied());
            return Ok(head.map_or(uc::MindTarget::NoPoint, |n| uc::MindTarget::Point([n.words[0], n.words[1]])));
        }
        let o = (mind.0 & !ORG_OBJECT) as usize;
        Ok(match ms.orgs[o].order.path.first() {
            Some(w) => uc::MindTarget::Point([w.pos.x.to_bits(), w.pos.y.to_bits()]),
            None => uc::MindTarget::NoPoint,
        })
    }
    /// Regiment Mind +0x68 `0x5247C0`: its Engage is current (`crate::combat`'s).
    fn mind_virtual_68(&mut self, _: Object) -> low_path::Result<bool> {
        Ok(false)
    }
    /// Element Mind +0x68 `0x535E50`: its Bombard is current (`crate::combat`'s).
    fn owner_virtual_68(&mut self, _: &mut MoveState) -> low_path::Result<bool> {
        Ok(false)
    }
    /// `0x4CEDE0`.
    fn actor_moving(&mut self, actor: Object) -> low_path::Result<bool> {
        let id = self.id(actor);
        let (ms, _) = self.host.parts_ref();
        Ok(ms.actors[id as usize].mover.active)
    }
    fn actor_virtual_0(&mut self, actor: Object) -> low_path::Result<Binary> {
        Ok(self.radius(self.id(actor), true))
    }
    fn actor_virtual_4(&mut self, actor: Object) -> low_path::Result<Binary> {
        Ok(self.radius(self.id(actor), false))
    }
    fn actor_collision_shape(&mut self, actor: Object) -> low_path::Result<Option<uc::Shape>> {
        let s = &self.index.member(actor.0).map_err(low)?.blocking;
        Ok(Some(uc::Shape { kind: s.kind, radius_bits: s.radius_bits, bounds_bits: s.bounds }))
    }
    fn actor_soft_shape(&mut self, actor: Object) -> low_path::Result<Option<uc::Shape>> {
        let s = &self.index.member(actor.0).map_err(low)?.soft;
        Ok(Some(uc::Shape { kind: s.kind, radius_bits: s.radius_bits, bounds_bits: s.bounds }))
    }
    /// Actor virtual +0x18: the Mover's speed.
    fn actor_virtual_18(&mut self, actor: Object) -> low_path::Result<Binary> {
        let id = self.id(actor);
        let (ms, _) = self.host.parts_ref();
        Binary::from_f32_bits(ms.actors[id as usize].mover.speed.to_bits()).map_err(low)
    }
    /// Actor virtual +0x20(1): a thing that takes others along (none is hosted).
    fn actor_virtual_20(&mut self, _: Object, _: u32) -> low_path::Result<bool> {
        Ok(false)
    }
    /// Actor virtual +0x16C(other): `other` is its enemy.
    fn actor_virtual_16c(&mut self, actor: Object, other: Object) -> low_path::Result<bool> {
        let (a, b) = (self.id(actor), self.id(other));
        let (_, world) = self.host.parts_ref();
        Ok(kingdom_team(world, world.actors[a as usize].kingdom) != kingdom_team(world, world.actors[b as usize].kingdom))
    }
    /// `0x4D6940`: at the tail of the list.
    fn ignore_thing(&mut self, actor: Object, entry: uc::IgnoredThing) -> low_path::Result<()> {
        let id = self.id(actor);
        let (ms, _) = self.host.parts_mut();
        ms.actors[id as usize].overlaps.push(aa_spatial::overlaps::OverlapEntry { reference: entry.gtid, expiry_bits: entry.until_bits });
        Ok(())
    }
    /// `0x531170`.
    fn forward_probe(&mut self, state: &mut MoveState, direction: [u32; 2], distance: u32) -> low_path::Result<[u32; 4]> {
        self.probe(state, None, direction, distance)
    }
    /// `0x5312C0`.
    fn origin_probe(&mut self, state: &mut MoveState, origin: [u32; 2], direction: [u32; 2], distance: u32) -> low_path::Result<[u32; 4]> {
        self.probe(state, Some(origin), direction, distance)
    }
    /// `0x46EF30`.
    fn shapes_overlap(&mut self, a: &uc::Descriptor, b: &uc::Descriptor) -> low_path::Result<bool> {
        aa_spatial::shape_overlap::overlaps(&Self::placement(a)?, &Self::placement(b)?, PC).map_err(low)
    }
    /// `0x46F100`.
    fn shapes_in_range(&mut self, a: &uc::Descriptor, range: u32, b: &uc::Descriptor, inverted: bool) -> low_path::Result<bool> {
        aa_spatial::range::shapes_in_range(&Self::placement(a)?, &Self::placement(b)?, range, inverted, PC).map_err(low)
    }
}

impl Uai<'_, '_> {
    fn probe(&mut self, state: &mut MoveState, origin: Option<[u32; 2]>, direction: [u32; 2], distance: u32) -> low_path::Result<[u32; 4]> {
        let id = self.host.actor_id();
        let target = self.host.ignored_thing(state.target_id);
        let position = self.host.position_bits();
        let geometry = self.geometry;
        let (index, mut policy) = self.policy_parts();
        let mut q = aa_spatial::actor_context::prepare(index, space::actor_key(id), false, &policy).map_err(low)?;
        let (from, goal) = match origin {
            Some(from) => (from, aa_spatial::actor_context::explicit_origin_goal(from, direction, distance, PC).map_err(low)?),
            None => (position, aa_spatial::actor_context::current_actor_goal(position, direction, distance, PC).map_err(low)?),
        };
        q.set_position(index, from, &policy).map_err(low)?;
        q.placement.facing = direction;
        q.descriptor.ignore_actor = target;
        let hit = aa_spatial::sweep::sweep(index, geometry, &mut q, goal, &mut policy).map_err(low)?;
        Ok(space::words(&hit))
    }
}

/// `LowPathingExaminedMax` and its fellows.
pub(super) fn settings(data: &crate::data::GameData) -> LowPathSettings {
    let d = LowPathSettings::ORIGINAL_DATA;
    let r = |name: &str, v: i32| data.rule(name).map_or(v, |x| x as i32);
    LowPathSettings {
        examined_max: r("LowPathingExaminedMax", d.examined_max),
        ignore_moving_depth: r("LowPathingIgnoreMovingDepth", d.ignore_moving_depth),
        accuracy_depth: r("LowPathingAccuracyDepth", d.accuracy_depth),
    }
}
pub(super) fn options() -> Options {
    Options { precision: PC, queue_limit: Some(low_path::NATIVE_QUEUE_LIMIT) }
}
