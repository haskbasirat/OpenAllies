//! What the Guard state (regiment Mind +17C, vtable 0xA32BC8, hosted on
//! `aa_mind::cai_guard` in `crate::movement`) reads here.
//!
//! Compare against the original: a denizen's threat list is read as the hostile
//! organizations inside its guard range (`intruders`), as `crate::denizen` has no
//! TickGuard 0x56A7E0; the home (Mind +28 / +30) of sallying militia has no host;
//! the settled check 0x585160 reads as false.
use crate::combat::{CombatState, TargetRef};
use crate::world::{OrgId, Unit, World};
use glam::Vec2;

/// 0x51DC80 / 0x51DD30; false when refused.
pub(super) fn begin(world: &mut World, o: OrgId, target: Option<Unit>, at: Vec2, aux: Option<Vec2>) -> bool {
    if let Some(t) = target
        && position(world, t).is_none()
    {
        return false;
    }
    crate::movement::guard_command(world, o, target, at, aux.unwrap_or(Vec2::ZERO))
}

pub(super) fn position(world: &World, t: Unit) -> Option<Vec2> {
    match t {
        Unit::Actor(a) => world.actor(a).filter(|x| x.alive).map(|x| x.pos.truncate()),
        Unit::Org(o) => {
            let org = world.orgs.get(o as usize).filter(|x| x.alive)?;
            Some(world.combat.org(o).filter(|c| c.initialized).map_or(org.anchor, |c| c.position))
        }
    }
}

/// The shape's +4: a circle's radius, half a box's shorter side.
pub(crate) fn shape_radius(d: &crate::data::Def) -> f32 {
    let Some(s) = d.blocking_size else { return 0.0 };
    match d.blocking_shape.as_deref().map(str::to_ascii_uppercase).as_deref() {
        Some("CIRCLE") => s[0].max(0.0),
        Some(_) => 0.5 * s[0].min(s[1]).max(0.0),
        None => 0.0,
    }
}

/// Who has `guarded` on an engage list (Actor +118).
pub(crate) fn attackers(world: &World, st: &CombatState, guarded: Unit) -> Vec<OrgId> {
    let org = match guarded {
        Unit::Org(o) => Some(o),
        Unit::Actor(a) => world.actors.get(a as usize).and_then(|x| x.org),
    };
    let actor = match guarded {
        Unit::Actor(a) => Some(a),
        Unit::Org(_) => None,
    };
    let mut out = Vec::new();
    for (i, c) in st.orgs.iter().enumerate() {
        if !world.orgs.get(i).is_some_and(|x| x.alive) {
            continue;
        }
        if c.engage.iter().any(|t| match t.target {
            TargetRef::Org(x) => Some(x) == org,
            TargetRef::Actor(x) => Some(x) == actor,
        }) {
            out.push(i as OrgId);
        }
    }
    out
}

/// 0x569300 of a guarded denizen: its intruders. OnIntruder 0x568D80 adds a
/// hostile thing's organization as it enters the guard zone, TickGuard 0x56A7E0
/// prunes it as it leaves; both are read here as the organizations inside the
/// guard range now. `None` for a thing without a DenizenComponent.
pub(crate) fn intruders(world: &World, st: &CombatState, guarded: Unit) -> Option<Vec<OrgId>> {
    let Unit::Actor(a) = guarded else { return None };
    let x = world.actors.get(a as usize)?;
    let dd = world.data.def(x.def).denizen.as_ref()?;
    let at = x.pos.truncate();
    let r2 = dd.guard_range * dd.guard_range;
    let mut out = Vec::new();
    for (i, c) in st.orgs.iter().enumerate() {
        let Some(org) = world.orgs.get(i).filter(|o| o.alive) else { continue };
        if crate::setup::relation(world, x.kingdom, Some(org.kingdom)) == crate::books::ENEMY && c.position.distance_squared(at) < r2 {
            out.push(i as OrgId);
        }
    }
    Some(out)
}
