//! Order feedback the UI asks the effects system to play.
//!
//! The original's order dispatch (`0x6229F0`) sends a location order and,
//! when it is accepted (`0x621C70`) and the order is a point order (actor
//! command flag 0x20 clear) or was given on the minimap, calls VWorld
//! `0x43DB10`: the AVar `VFXMoveCommand` VFX (`move_vfx`, Effects/
//! game_vfx.tgi: VFXModelTemporary, model MoveEffect/MoveEffect.NIF) is
//! created at the order point, at the ground height (`0x43D050`, flags
//! 0x222) plus 0.1 (`[0x9FE0C0]`), unattached. A temporary model VFX plays
//! its NIF once and goes away.
//!
//! The UI pushes [`PointVfx`] requests here; the effects system (owner:
//! effects) consumes them every frame. [`Waypoints`] are the selected
//! units' order flags, which the effects system keeps on screen.
use crate::session::Battle;
use aa_play::{Order, Unit};
use bevy::prelude::*;

/// One point VFX request.
#[derive(Clone, Debug, PartialEq)]
pub struct PointVfxRequest {
    /// VFX IDS (Effects/*.tgi), e.g. `move_vfx`.
    pub vfx_ids: String,
    /// Native ground point (x, y); z = ground height + `z_offset`.
    pub at: Vec2,
    pub z_offset: f32,
}

/// Requests of this frame (drained by the effects system).
#[derive(Resource, Default, Debug)]
pub struct PointVfx {
    pub requests: Vec<PointVfxRequest>,
}

/// `0x43DB10`: the move command VFX (AVar VFXMoveCommand) at a ground point.
pub fn move_command(marks: &mut PointVfx, at: Vec2) {
    marks.requests.push(PointVfxRequest { vfx_ids: "move_vfx".into(), at, z_offset: 0.1 });
}

/// One waypoint marker of a selected unit's order (`0x626F50` slot).
#[derive(Clone, Debug, PartialEq)]
pub struct WaypointMark {
    /// VFXModelPermanent IDS (Effects/waypoints.tgi), e.g.
    /// `waypoint_move_actor`.
    pub vfx_ids: String,
    /// Native position (x, y, z); z is already the ground height (or the
    /// target actor's height).
    pub at: Vec3,
}

/// The waypoint markers to show this frame, in slot order (`0x627D90`).
///
/// The original's unit panels (SimpleActorDisplay, SimpleActorMini,
/// CompanyMini: panel flag +0x92) keep a list of waypoint slots for their
/// actor. Every frame `0x627D90` walks the actor's current order, then its
/// queued orders, and `0x626F50` fills one slot per order.
/// - A slot whose order is unchanged only moves its VFX (vtable +0x68, the
///   new position from `0x624F80`).
/// - A new order recreates it (`0x6252A0`: the order's ActorCommand
///   `command_vfx_ids_actor`, resolved at +0x30, created at the point with
///   `0x43CCA0`, a permanent model VFX).
/// - Slots past the last order are emptied, so the marker lives exactly as
///   long as the order (and the selection).
///
/// The effects system keeps one VFX per index: it recreates the VFX when
/// `vfx_ids` changes, moves it to `at`, and removes the VFX of indices past
/// the end of the list.
#[derive(Resource, Default, Debug)]
pub struct Waypoints {
    pub marks: Vec<WaypointMark>,
}

/// Which actor command the UI gave a unit, so its waypoint shows that
/// command's VFX. The simulation folds move / run / guard / explore /
/// attack-move into one `Order::Move`.
#[derive(Resource, Default, Debug)]
pub struct Issued {
    orders: Vec<(Unit, Vec2, &'static str)>,
}

impl Issued {
    /// `vfx_ids` is the command's `command_vfx_ids_actor`
    /// (Game/actor_commands*.tgi).
    pub fn give(&mut self, units: &[Unit], to: Vec2, vfx_ids: &'static str) {
        self.orders.retain(|(u, _, _)| !units.contains(u));
        self.orders.extend(units.iter().map(|u| (*u, to, vfx_ids)));
    }

    fn of(&self, u: Unit, to: Vec2) -> Option<&'static str> {
        self.orders.iter().find(|(x, p, _)| *x == u && p.distance_squared(to) < 1e-4).map(|(_, _, v)| *v)
    }
}

/// `0x627D90` for every selected own unit: its current order's waypoint.
/// - Location orders (order flag 4) stand at the order point on the ground
///   (`0x624F80`: `0x4F0E50`, flags 0x122).
/// - Actor orders stand at the target's position.
/// - Our orders have no queue, so there are no numbered queued waypoints
///   (VFXText, `UIWaypointNumberZOffset`).
pub fn fill_waypoints(battle: Option<Res<Battle>>, issued: Res<Issued>, mut marks: ResMut<Waypoints>) {
    let Some(b) = battle else {
        if !marks.marks.is_empty() {
            marks.marks.clear();
        }
        return;
    };
    let w = &b.world;
    let mut out = Vec::new();
    for u in &b.selection {
        let order = match u {
            Unit::Org(o) => match w.orgs.get(*o as usize) {
                Some(g) if g.alive && g.kingdom == b.local => g.order.clone(),
                _ => continue,
            },
            Unit::Actor(a) => match w.actor(*a) {
                Some(x) if x.alive && x.kingdom == Some(b.local) => x.order.clone(),
                _ => continue,
            },
        };
        let ground = |p: Vec2| Vec3::new(p.x, p.y, w.map.height_at(p));
        match order {
            Order::Idle => {}
            Order::Move { to, attack_move } => {
                // the command the UI gave; kill (attack-move) / move otherwise
                let vfx = issued.of(*u, to).unwrap_or(if attack_move { "waypoint_attack_actor" } else { "waypoint_move_actor" });
                out.push(WaypointMark { vfx_ids: vfx.into(), at: ground(to) });
            }
            Order::Attack { target } => {
                if let Some(t) = w.actor(target).filter(|t| t.alive && crate::ui::fog::drawn(&b, t.id)) {
                    out.push(WaypointMark { vfx_ids: "waypoint_attack_actor".into(), at: t.pos });
                }
            }
            Order::Deploy { at, .. } => out.push(WaypointMark { vfx_ids: "waypoint_unpack".into(), at: ground(at) }),
        }
    }
    if marks.marks != out {
        marks.marks = out;
    }
}

/// Requests nobody drained this frame (no battle view) are dropped so they
/// do not play late.
pub fn drop_stale(mut marks: ResMut<PointVfx>) {
    if !marks.requests.is_empty() {
        marks.requests.clear();
    }
}
