//! Original5625C0 damage sequencing. World relationships, capture, component
//! responses and death queues are required synchronous operations, not defaults.
use crate::{Body, Environment, Error, F32, HealthChange, Host, Result};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

/// Seven native words at Body+28. Only the arithmetic/event roles used here are
/// named; definitions and GTIDs still require exact runtime registry resolution.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageRecord {
    pub source_id: u32,
    pub allegiance_id: u32,
    pub damage_type: u32,
    pub amount: F32,
    pub time: F32,
    pub event_kind: u32,
    pub reason: u32,
}
impl DamageRecord {
    pub fn words(self) -> [u32; 7] {
        [
            self.source_id,
            self.allegiance_id,
            self.damage_type,
            self.amount.0,
            self.time.0,
            self.event_kind,
            self.reason,
        ]
    }
}

/// Weapon object's definition+38/+4C, tested only for initial wall propagation.
/// Presence means both the weapon and its definition are valid native objects.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct WeaponPropagation {
    pub definition_word38: F32,
    pub definition_word4c: F32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct DamageRequest {
    pub record: DamageRecord,
    pub apply_type_modifier: bool,
    pub weapon_id: Option<u32>,
    pub weapon_definition: Option<WeaponPropagation>,
    /// Incoming final bool, used only before the native hostility call overwrites
    /// its stack slot. Later events use computed hostility, not this input.
    pub propagate_to_neighbors: bool,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PropagationTarget {
    LinkedBody,
    NeighborBody(u32),
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorView {
    pub wall_link: Option<u32>,
    pub organization: Option<u32>,
    pub organization_leader: Option<u32>,
    pub mind: Option<u32>,
    pub component88: Option<u32>,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Wall {
    pub actor: u32,
    pub segments: Option<u32>,
    pub transfer_factor: F32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct OuterActor {
    pub actor: u32,
    /// Native outer actor+98, required for584CD0. Invalid pointers must error.
    pub component98: u32,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action {
    ResetRecovery,
    OrganizationElementDamaged { leader: u32 },
    OuterActorDamaged { component98: u32 },
    ModifierOnAttack { modifier: u32, source: u32 },
    MindDamaged { mind: u32 },
    PresentationDamaged,
    SourceType3 { source: u32, weapon: Option<u32> },
    OrganizationDamaged { organization: u32 },
    Component88Damaged { component: u32 },
    WallMemberKilled { component98: u32 },
    QueueDeath,
}
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Outcome {
    Ignored,
    Applied,
    Captured,
    QueuedForDeath,
    WallDeathDispatched,
}

/// All graph/actor/component leaves are required. Field views are reread at the
/// original load points; synchronous actions may mutate Body, Environment and
/// host world state. Errors can follow effects and must never trigger auto-retry.
pub trait World: Host {
    fn propagate_damage(
        &mut self,
        target: PropagationTarget,
        request: DamageRequest,
        body: &mut Body,
        env: &mut Environment,
        precision: Precision,
    ) -> std::result::Result<(), String>;
    fn actor_view(&self, actor: u32) -> std::result::Result<ActorView, String>;
    fn wall(&self, link: u32) -> std::result::Result<Option<Wall>, String>;
    fn wall_index(&self, segments: u32, actor: u32) -> std::result::Result<i32, String>;
    ///585480(index,forward,1), then array entry+Body eligibility. Both neighbor
    /// queries occur separately, so first propagation can change the second.
    fn wall_neighbor(
        &self,
        segments: u32,
        index: i32,
        forward: bool,
    ) -> std::result::Result<Option<u32>, String>;
    /// Generation-resolve allegiance_id, then actor virtual+140 with that object
    /// or null. The virtual call is allowed to mutate runtime state.
    fn hostile(
        &mut self,
        allegiance_id: u32,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<bool, String>;
    fn resolve_source(&self, source_id: u32) -> std::result::Result<Option<u32>, String>;
    /// Exact4CF110 actor+78 modifier context, not a kingdom/name guess.
    fn modifier(&self, actor: u32) -> std::result::Result<Option<u32>, String>;
    fn damage_factor(&self, modifier: u32, damage_type: u32) -> std::result::Result<F32, String>;
    /// Actor virtual+100 runs even when hostility is false.
    fn outer_actor(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<Option<OuterActor>, String>;
    ///4CEDA0 then parent+88; own+88 is used only if this resolves absent.
    fn ancestor_component88(&self, actor: u32) -> std::result::Result<Option<u32>, String>;
    /// Full4D1120 eligibility with ownership/action/leader checks; no default.
    fn capture_eligible(
        &mut self,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<bool, String>;
    fn attempt_capture(
        &mut self,
        source: Option<u32>,
        record: &mut DamageRecord,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<bool, String>;
    /// Original CaptureHealthFactor lookup returns an x87 value. Supplying the
    /// actual resolved binary32 SVar preserves this getter's exact return.
    fn capture_health_factor(&self) -> std::result::Result<F32, String>;
    fn action(
        &mut self,
        action: Action,
        record: &mut DamageRecord,
        body: &mut Body,
        env: &mut Environment,
    ) -> std::result::Result<(), String>;
}

///501650: first matching damage-type factor, otherwise the original1.0. Registry
/// identity comparisons are exact; no text-name matching or sorting is applied.
pub fn first_damage_factor(entries: &[(u32, F32)], damage_type: u32) -> F32 {
    entries
        .iter()
        .find(|(key, _)| *key == damage_type)
        .map_or(F32::ONE, |(_, value)| *value)
}

fn action(
    body: &mut Body,
    env: &mut Environment,
    world: &mut impl World,
    kind: Action,
    record: &mut DamageRecord,
) -> Result<()> {
    world
        .action(kind, record, body, env)
        .map_err(Error::Callback)?;
    body.validate_fields(env)
}
fn nonpositive(value: F32) -> Result<bool> {
    Ok(value.compare(F32::ZERO)? != Ordering::Greater)
}

/// Complete finite outer DealDamage5625C0. Armor/weapon hit computation belongs
/// upstream; this accepts the original resolved seven-word damage record.
pub fn deal_damage(
    body: &mut Body,
    request: DamageRequest,
    env: &mut Environment,
    world: &mut impl World,
    precision: Precision,
) -> Result<Outcome> {
    body.validate_fields(env)?;
    request.record.amount.number()?;
    if env.has_linked_body {
        world
            .propagate_damage(
                PropagationTarget::LinkedBody,
                DamageRequest {
                    propagate_to_neighbors: true,
                    ..request
                },
                body,
                env,
                precision,
            )
            .map_err(Error::Callback)?;
        body.validate_fields(env)?;
    }
    if nonpositive(request.record.amount)? || env.destructed {
        return Ok(Outcome::Ignored);
    }
    let mut propagate = request.propagate_to_neighbors;
    if request.weapon_id.is_some()
        && let Some(weapon) = request.weapon_definition
        && (weapon.definition_word38.compare(F32::ZERO)? == Ordering::Greater
            || weapon.definition_word4c.compare(F32::ZERO)? == Ordering::Greater)
    {
        propagate = false;
    }
    if let Some(link) = world
        .actor_view(env.actor_id)
        .map_err(Error::Callback)?
        .wall_link
        && propagate
        && let Some(wall) = world.wall(link).map_err(Error::Callback)?
        && let Some(segments) = wall.segments
        && wall.transfer_factor.compare(F32::ZERO)? != Ordering::Equal
    {
        let index = world
            .wall_index(segments, env.actor_id)
            .map_err(Error::Callback)?;
        if index >= 0 {
            // Native561BE0 copies the seven-word record before the factor store.
            let mut neighbor = request;
            neighbor.record.amount = F32(wall
                .transfer_factor
                .number()?
                .mul(&request.record.amount.number()?, precision)?
                .store_f32()?);
            neighbor.propagate_to_neighbors = false;
            for forward in [true, false] {
                if let Some(target) = world
                    .wall_neighbor(segments, index, forward)
                    .map_err(Error::Callback)?
                {
                    world
                        .propagate_damage(
                            PropagationTarget::NeighborBody(target),
                            neighbor,
                            body,
                            env,
                            precision,
                        )
                        .map_err(Error::Callback)?;
                    body.validate_fields(env)?;
                }
            }
        }
    }
    let hostile = world
        .hostile(request.record.allegiance_id, body, env)
        .map_err(Error::Callback)?;
    body.validate_fields(env)?;
    let source = world
        .resolve_source(request.record.source_id)
        .map_err(Error::Callback)?;
    let mut record = request.record;
    let modifier = world.modifier(env.actor_id).map_err(Error::Callback)?;
    if request.apply_type_modifier
        && let Some(modifier) = modifier
    {
        record.amount = F32(world
            .damage_factor(modifier, record.damage_type)
            .map_err(Error::Callback)?
            .number()?
            .mul(&record.amount.number()?, precision)?
            .store_f32()?);
    }
    if hostile {
        body.last_damage = record.words();
    }
    let requested = F32(body
        .health
        .number()?
        .sub(&record.amount.number()?, precision)?
        .store_f32()?);
    body.set_health_from_source(
        HealthChange {
            requested,
            token: 0,
            notify: true,
            source_id: Some(request.record.source_id),
        },
        env,
        world,
        precision,
    )?;
    if hostile && env.has_recovery_component && env.actor_flags & 0x100000 == 0 {
        action(body, env, world, Action::ResetRecovery, &mut record)?;
    }
    let view = world.actor_view(env.actor_id).map_err(Error::Callback)?;
    if hostile
        && view.organization.is_some()
        && let Some(leader) = view.organization_leader
    {
        action(
            body,
            env,
            world,
            Action::OrganizationElementDamaged { leader },
            &mut record,
        )?;
    }
    let outer = world.outer_actor(body, env).map_err(Error::Callback)?;
    body.validate_fields(env)?;
    if hostile {
        if let Some(outer) = outer {
            action(
                body,
                env,
                world,
                Action::OuterActorDamaged {
                    component98: outer.component98,
                },
                &mut record,
            )?;
        }
        if record.event_kind == 1 {
            if let (Some(source), Some(modifier)) = (source, modifier)
                && record.amount.compare(F32::ZERO)? == Ordering::Greater
            {
                action(
                    body,
                    env,
                    world,
                    Action::ModifierOnAttack { modifier, source },
                    &mut record,
                )?;
            }
            if let Some(mind) = world
                .actor_view(env.actor_id)
                .map_err(Error::Callback)?
                .mind
            {
                action(body, env, world, Action::MindDamaged { mind }, &mut record)?;
            }
        }
    }
    action(body, env, world, Action::PresentationDamaged, &mut record)?;
    if let Some(source) = source
        && record.event_kind == 3
    {
        action(
            body,
            env,
            world,
            Action::SourceType3 {
                source,
                weapon: request.weapon_id,
            },
            &mut record,
        )?;
    }
    if let Some(organization) = world
        .actor_view(env.actor_id)
        .map_err(Error::Callback)?
        .organization
    {
        action(
            body,
            env,
            world,
            Action::OrganizationDamaged { organization },
            &mut record,
        )?;
    }
    let ancestor = world
        .ancestor_component88(env.actor_id)
        .map_err(Error::Callback)?;
    let target = if ancestor.is_some() {
        ancestor
    } else {
        world
            .actor_view(env.actor_id)
            .map_err(Error::Callback)?
            .component88
    };
    if let Some(component) = target {
        action(
            body,
            env,
            world,
            Action::Component88Damaged { component },
            &mut record,
        )?;
    }
    let capture_eligible = if nonpositive(body.health)? {
        let eligible = world.capture_eligible(body, env).map_err(Error::Callback)?;
        body.validate_fields(env)?;
        eligible
    } else {
        false
    };
    if capture_eligible {
        let captured = if hostile {
            let accepted = world
                .attempt_capture(source, &mut record, body, env)
                .map_err(Error::Callback)?;
            body.validate_fields(env)?;
            accepted
        } else {
            false
        };
        if captured {
            let requested = F32(world
                .capture_health_factor()
                .map_err(Error::Callback)?
                .number()?
                .mul(&body.maximum.number()?, precision)?
                .store_f32()?);
            body.set_health_from_source(
                HealthChange {
                    requested,
                    token: 0,
                    notify: true,
                    source_id: None,
                },
                env,
                world,
                precision,
            )?;
            return Ok(Outcome::Captured);
        }
        if env.actor_flags & 0x4000_0000 == 0 {
            body.set_health_from_source(
                HealthChange {
                    requested: F32::ONE,
                    token: 0,
                    notify: true,
                    source_id: None,
                },
                env,
                world,
                precision,
            )?;
        }
    }
    if nonpositive(body.health)? && env.destructible {
        // Native friendly-fire logging is diagnostic only. No mutation from the
        // logger or CString lifetime is fabricated here.
        if let Some(link) = world
            .actor_view(env.actor_id)
            .map_err(Error::Callback)?
            .wall_link
            && world.wall(link).map_err(Error::Callback)?.is_some()
        {
            let wall = world.wall(link).map_err(Error::Callback)?.ok_or_else(|| {
                Error::Callback("wall owner vanished between native reads".into())
            })?;
            let component98 = wall.segments.ok_or_else(|| {
                Error::Callback("wall owner has no native segment component".into())
            })?;
            action(
                body,
                env,
                world,
                Action::WallMemberKilled { component98 },
                &mut record,
            )?;
            return Ok(Outcome::WallDeathDispatched);
        }
        action(body, env, world, Action::QueueDeath, &mut record)?;
        return Ok(Outcome::QueuedForDeath);
    }
    Ok(Outcome::Applied)
}
