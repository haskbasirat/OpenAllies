//! Native combat reactions triggered by Body damage (`5625C0`).
//!
//! - `501770` Modifier OnAttacked: every defender ability granted by the
//!   victim's modifier (`+B0` array, `+B4` count cached before the loop) is
//!   tried against the attacker through `4C9EB0`, sharing ONE context built by
//!   `4C8300(2)` with source = victim and target = attacker. Context writes by
//!   one ability's admission/use are seen by the next ability.
//! - `4C9EB0`: `4C8F20(context, check_range=true)`; on success `4C9550` at the
//!   current world time.
//! - `4D6190` attacked-by bookkeeping (event kind 3 damage): the victim (or the
//!   organization it belongs to) refreshes or prepends `{attacker GTID, now,
//!   timeout}` in its `+130` list, where timeout is BombardTargetTimeout plus,
//!   for a Mind that accepts it (`+6C`), the weapon definition `+20`. A new
//!   entry also runs `4D3A70` on the attacker: prepend the victim GTID to its
//!   `+128` engagement list and emit event `0x24|!bit29` unless the attacker
//!   is a detached organization member.
use crate::{Error, Result, performer::Context};
use aa_body::F32;
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};

pub trait DefenderWorld {
    /// Modifier `+B4`, read once.
    fn ability_count(&self) -> Result<i32>;
    /// Modifier `+B0[index]`, reread per iteration.
    fn ability(&self, index: i32) -> Result<u32>;
    /// `4C8F20` admission (aa_combat::admission::can_use in production).
    fn can_use(&mut self, ability: u32, context: &mut Context, check_range: bool) -> Result<u32>;
    /// `4C9550` (aa_combat::execution::use_ability in production).
    fn use_ability(&mut self, ability: u32, context: &mut Context, time: F32) -> Result<u32>;
    /// World `+E0`.
    fn now(&self) -> Result<F32>;
}

/// Native `4C9EB0`.
pub fn try_use(ability: u32, context: &mut Context, world: &mut impl DefenderWorld) -> Result<u32> {
    if world.can_use(ability, context, true)? != 0 {
        return Ok(0);
    }
    let time = world.now()?;
    world.use_ability(ability, context, time)
}

/// Native `501770(victim, attacker)`.
pub fn defender_abilities(
    victim: u32,
    attacker: u32,
    world: &mut impl DefenderWorld,
) -> Result<()> {
    let mut context = Context::new(2);
    context.source = Some(victim);
    context.target = Some(attacker);
    let count = world.ability_count()?;
    let mut index = 0;
    while index < count {
        let ability = world.ability(index)?;
        let _ = try_use(ability, &mut context, world)?;
        index += 1;
    }
    Ok(())
}

/// One `+130` attacked-by node.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AttackedBy {
    pub attacker_id: u32,
    pub time: F32,
    pub timeout: F32,
}

/// Membership component `+80`: `+10` organization actor and `+1C` attached.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Membership {
    pub organization: Option<u32>,
    pub attached: bool,
}

pub trait AttackWorld {
    /// Actor virtual `+A8`; true stops bookkeeping on that actor.
    fn virtual_a8(&mut self, actor: u32) -> Result<bool>;
    fn membership(&self, actor: u32) -> Result<Option<Membership>>;
    /// Actor virtual `+100` (outer actor).
    fn outer(&mut self, actor: u32) -> Result<Option<u32>>;
    /// Actor `+14` GTID.
    fn actor_id(&self, actor: u32) -> Result<u32>;
    /// Actor `+8`.
    fn flags8(&self, actor: u32) -> Result<u32>;
    /// Actor `+130` list (front is the native head).
    fn attacked_by(&mut self, actor: u32) -> Result<&mut Vec<AttackedBy>>;
    /// Actor `+128` engagement list (front is the native head).
    fn engaged(&mut self, actor: u32) -> Result<&mut Vec<u32>>;
    /// `actor+70` Mind present, then its virtual `+6C`.
    fn has_mind(&self, actor: u32) -> Result<bool>;
    fn mind_accepts_weapon_bonus(&mut self, actor: u32) -> Result<bool>;
    /// World `+E0`.
    fn now(&self) -> Result<F32>;
    /// `4D3A70` message: name virtual `+24`, allegiance `+108`, `42D490`.
    fn engagement_message(&mut self, attacker: u32, event: u32) -> Result<()>;
}

fn n(value: F32) -> Result<Binary> {
    Ok(Binary::from_f32_bits(value.0)?)
}

/// Native `4D3A70(victim)` on `attacker`.
pub fn record_engagement(
    attacker: u32,
    victim: Option<u32>,
    world: &mut impl AttackWorld,
) -> Result<()> {
    let victim_id = match victim {
        Some(v) => world.actor_id(v)?,
        None => 0,
    };
    let list = world.engaged(attacker)?;
    list.try_reserve(1)
        .map_err(|e| Error::World(format!("engagement list allocation failed: {e}")))?;
    list.insert(0, victim_id);
    let detached_member = matches!(
        world.membership(attacker)?,
        Some(Membership {
            organization: Some(_),
            attached: false
        })
    );
    if !detached_member {
        let event = 0x24 | (!(world.flags8(attacker)? >> 29) & 1);
        world.engagement_message(attacker, event)?;
    }
    Ok(())
}

/// Native `4D6190(source, weapon)` on `victim`. `weapon` is `None` for a null
/// weapon object, `Some(None)` for a weapon without a definition and
/// `Some(Some(bonus))` for definition `+20`. `bombard_timeout` is the
/// settings-cache `+1A0` value.
pub fn attacked_by(
    victim: u32,
    source: u32,
    weapon: Option<Option<F32>>,
    bombard_timeout: F32,
    world: &mut impl AttackWorld,
    p: Precision,
) -> Result<()> {
    if world.virtual_a8(victim)? {
        return Ok(());
    }
    let mut victim = victim;
    while let Some(Membership {
        organization: Some(organization),
        ..
    }) = world.membership(victim)?
    {
        if world.virtual_a8(organization)? {
            return Ok(());
        }
        victim = organization;
    }
    let attacker = match world.membership(source)? {
        Some(Membership {
            organization: Some(organization),
            attached: true,
        }) => organization,
        _ => match world.outer(source)? {
            Some(_) => world
                .outer(source)?
                .ok_or(Error::Invalid("outer actor vanished between native calls"))?,
            None => source,
        },
    };
    let attacker_id = world.actor_id(attacker)?;
    if let Some(index) = world
        .attacked_by(victim)?
        .iter()
        .position(|entry| entry.attacker_id == attacker_id)
    {
        let now = world.now()?;
        world.attacked_by(victim)?[index].time = now;
        return Ok(());
    }
    let mut timeout = bombard_timeout;
    if world.has_mind(victim)?
        && world.mind_accepts_weapon_bonus(victim)?
        && let Some(Some(bonus)) = weapon
    {
        timeout = F32(n(timeout)?.add(&n(bonus)?, p)?.store_f32()?);
    }
    let now = world.now()?;
    let list = world.attacked_by(victim)?;
    list.try_reserve(1)
        .map_err(|e| Error::World(format!("attacked-by list allocation failed: {e}")))?;
    list.insert(
        0,
        AttackedBy {
            attacker_id,
            time: now,
            timeout,
        },
    );
    record_engagement(attacker, Some(victim), world)
}
