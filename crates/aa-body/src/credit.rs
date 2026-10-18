//! Native actor kill-credit `4D57C0`, classification leaves and score arithmetic.
//!
//! Actor/component handles are host identities, never original process pointers.
//! Callbacks are required and synchronous. Errors preserve prior writes, including
//! the credited bit, so a failed operation must not be automatically retried.
use crate::{F32, Result};
use aa_numeric::{Binary, Precision};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Object(pub u32);

/// Direct native victim/definition fields, reread at their original boundaries.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Victim {
    pub identity: Object,
    /// Actor+8, distinct from the actor property word+100.
    pub flags: u32,
    pub owner: Option<Object>,
    pub body_damage: Option<[u32; 7]>,
    pub alternate_damage: Option<[u32; 7]>,
    pub component7c: bool,
    pub organization: bool,
    pub organization_leader: Option<Object>,
    pub component88: bool,
    pub component90: bool,
    pub component94: bool,
    pub component98: bool,
    pub component9c: bool,
    pub componenta4: bool,
    pub componentc0: bool,
    pub componentc4: bool,
    pub componentd8: bool,
    pub definition2c8_nonempty: bool,
    pub definition43c_nonempty: bool,
    pub definition4e0_nonzero: bool,
    pub definition4f4: F32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Settings {
    pub session_mode: u32,
    pub world_time: F32,
    pub kill_credit_timeout: F32,
    /// Cache+168, +160, +164 and +178 respectively; resolved native settings.
    pub experience_factor: F32,
    pub experience_minimum: F32,
    pub experience_maximum: F32,
    pub zero_value_fallback: F32,
    pub campaign_observer: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub victim: Victim,
    pub settings: Settings,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct StatsHandles {
    pub ordinary: Option<Object>,
    pub relations: Option<Object>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct SourceFields {
    pub component7c: bool,
    pub organization_leader: Option<Object>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Event {
    PlayerNotification {
        player: Object,
        owner_label: String,
        victim_label: String,
    },
    SpecialNotification {
        allegiance: Option<Object>,
        victim_label: String,
    },
    CampaignDeath,
    AwardExperience {
        source: Object,
        amount: F32,
        argument1: bool,
        argument2: bool,
    },
    OrganizationReward {
        source: Object,
    },
    Component7cReward {
        source: Object,
    },
    Componentc4Reward {
        source: Object,
    },
    DeathTrigger,
}

pub trait World {
    /// Pure generation-checked registry lookup (`432E40`).
    fn resolve(&self, id: u32) -> Result<Option<Object>>;
    fn source_accepts_victim(&mut self, source: Object, state: &mut State) -> Result<bool>;
    fn hostile(&mut self, other: Object, state: &mut State) -> Result<bool>;
    fn outer_actor(&mut self, state: &mut State) -> Result<Option<Object>>;
    fn actor_flags(&self, actor: Object) -> Result<u32>;
    fn virtual20(&mut self, argument: bool, state: &mut State) -> Result<bool>;
    fn begin_players(&mut self, state: &mut State) -> Result<Option<Object>>;
    fn next_player(&mut self, state: &mut State) -> Result<Option<Object>>;
    fn visible_to(&mut self, player: Object, argument: bool, state: &mut State) -> Result<bool>;
    fn victim_label(&mut self, state: &mut State) -> Result<String>;
    fn owner_label(&mut self, owner: Object, state: &mut State) -> Result<String>;
    fn allegiance(&mut self, state: &mut State) -> Result<Option<Object>>;
    /// Direct field views, without virtual calls or synthesized missing owners.
    fn stats_handles(&self, allegiance: Object) -> Result<StatsHandles>;
    fn source_fields(&self, source: Object) -> Result<SourceFields>;
    /// Native `58E870(index, amount)` on the kingdom statistics object
    /// (kingdom+0x32C). aa-body owns no statistics type: the runtime binds this
    /// to `aa_economy::tick::add_score` on the single aa-economy `Statistics`
    /// (same arithmetic as [`increment_stat`]). Adapters must return errors for
    /// absent components or invalid indices rather than inventing statistics.
    fn add_statistic(
        &mut self,
        statistics: Object,
        index: u32,
        amount: F32,
        precision: Precision,
    ) -> Result<()>;
    /// Native `58D950(index, other, amount)` on the kingdom relation statistics
    /// (kingdom+0x340). Bind to `aa_economy::statistics::RelationStatistics::add`
    /// on the single aa-economy store; aa-body keeps no relation-statistics type.
    fn add_relation(
        &mut self,
        statistics: Object,
        index: u32,
        other: Option<Object>,
        amount: F32,
        precision: Precision,
    ) -> Result<()>;
    /// Native `4D01E0(actor,0)` extended result. Component-derived value is a
    /// required world dependency; rounding here would change experience.
    fn actor_value(
        &mut self,
        actor: Object,
        state: &mut State,
        precision: Precision,
    ) -> Result<Binary>;
    fn event(&mut self, event: Event, state: &mut State) -> Result<()>;
}

/// Native `58E870` arithmetic: one add at the active precision, then a
/// binary32 store. Identical to `aa_economy::tick::add_score`, which owns the
/// live statistics storage; kept here only for bindings without that crate.
pub fn increment_stat(value: &mut F32, amount: F32, precision: Precision) -> Result<()> {
    *value = F32(amount
        .number()?
        .add(&value.number()?, precision)?
        .store_f32()?);
    Ok(())
}

/// Native `571820` followed by `571960`. Victim value has a binary32 store;
/// source value and ratio remain extended. Factor is cached before callbacks,
/// whereas minimum/maximum are loaded after them. This returns the caller's
/// final binary32 store at `4D5D5C`.
pub fn experience_amount<W: World>(
    source: Object,
    state: &mut State,
    world: &mut W,
    precision: Precision,
) -> Result<F32> {
    let factor = state.settings.experience_factor.number()?;
    let mut value = F32(world
        .actor_value(state.victim.identity, state, precision)?
        .store_f32()?);
    if value.compare(F32::ZERO)? == Ordering::Equal {
        value = if state.victim.definition4f4.0 == 0xbf80_0000 {
            F32(0xbf80_0000)
        } else {
            state.settings.zero_value_fallback
        };
    }
    let source_value = world.actor_value(source, state, precision)?;
    let ratio = if source_value.compare(&Binary::from_u32(0)) == Ordering::Greater {
        let half = Binary::from_f32_bits(0x3f00_0000)?;
        value
            .number()?
            .div(&source_value, precision)?
            .mul(&half, precision)?
            .add(&half, precision)?
    } else {
        Binary::from_u32(0)
    };
    let amount = F32(ratio.mul(&factor, precision)?.store_f32()?);
    if amount.compare(state.settings.experience_minimum)? == Ordering::Less {
        Ok(state.settings.experience_minimum)
    } else if amount.compare(state.settings.experience_maximum)? == Ordering::Greater {
        Ok(state.settings.experience_maximum)
    } else {
        Ok(amount)
    }
}

fn missing(what: &str) -> crate::Error {
    crate::Error::Callback(format!("kill-credit {what}"))
}

fn pair<W: World>(
    world: &mut W,
    victim: Option<Object>,
    attacker: Option<Object>,
    lost: u32,
    won: u32,
    precision: Precision,
) -> Result<()> {
    if let Some(handle) = victim {
        world.add_statistic(handle, lost, F32::ONE, precision)?;
    }
    if let Some(handle) = attacker {
        world.add_statistic(handle, won, F32::ONE, precision)?;
    }
    Ok(())
}

/// Execute native credit once. Returns the cached credited source, which can
/// differ from the source-or-leader used in the experience ratio.
pub fn credit_kill<W: World>(
    state: &mut State,
    world: &mut W,
    precision: Precision,
) -> Result<Option<Object>> {
    if state.victim.flags & 0x10 != 0 || state.settings.session_mode == 3 {
        return Ok(None);
    }
    state.victim.flags |= 0x10;
    let Some(record) = state.victim.body_damage.or(state.victim.alternate_damage) else {
        return Ok(None);
    };
    let expired = state
        .settings
        .world_time
        .number()?
        .sub(&F32(record[4]).number()?, precision)?
        .compare(&state.settings.kill_credit_timeout.number()?)
        == Ordering::Greater;
    let (mut source, mut allegiance) = if expired {
        (None, None)
    } else {
        (world.resolve(record[0])?, world.resolve(record[1])?)
    };
    if let Some(actor) = source
        && !world.source_accepts_victim(actor, state)?
    {
        source = None;
    }
    if let Some(owner) = allegiance
        && !world.hostile(owner, state)?
    {
        allegiance = None;
    }

    if (!state.victim.organization || state.victim.organization_leader.is_none())
        && !state.victim.component90
    {
        let outer_dead = if world.outer_actor(state)?.is_some() {
            let outer = world
                .outer_actor(state)?
                .ok_or_else(|| missing("second outer-actor result is null"))?;
            world.actor_flags(outer)? & 1 != 0
        } else {
            false
        };
        if ((!state.victim.component94 && !state.victim.component88) || !outer_dead)
            && state.victim.owner.is_some()
            && (world.virtual20(true, state)? || state.victim.componentc0)
        {
            let mut player = world.begin_players(state)?;
            while let Some(current) = player {
                if world.hostile(current, state)? && world.visible_to(current, true, state)? {
                    let victim_label = world.victim_label(state)?;
                    let owner = state
                        .victim
                        .owner
                        .ok_or_else(|| missing("notification owner became null"))?;
                    let owner_label = world.owner_label(owner, state)?;
                    world.event(
                        Event::PlayerNotification {
                            player: current,
                            owner_label,
                            victim_label,
                        },
                        state,
                    )?;
                }
                player = world.next_player(state)?;
            }
        }
    }

    // Original snapshots all four component pointers before classification and
    // later reward callbacks. Related-owner arguments are still reread below.
    let absent = StatsHandles {
        ordinary: None,
        relations: None,
    };
    let victim_stats = match state.victim.owner {
        Some(o) => world.stats_handles(o)?,
        None => absent,
    };
    let attacker_stats = match allegiance {
        Some(o) => world.stats_handles(o)?,
        None => absent,
    };
    if state.victim.component7c && !state.victim.componentd8 && world.virtual20(false, state)? {
        pair(
            world,
            victim_stats.ordinary,
            attacker_stats.ordinary,
            5,
            4,
            precision,
        )?;
    }
    if state.victim.organization && !state.victim.componentd8 && world.virtual20(false, state)? {
        pair(
            world,
            victim_stats.ordinary,
            attacker_stats.ordinary,
            10,
            9,
            precision,
        )?;
        if let (Some(stats), Some(other)) = (victim_stats.relations, allegiance) {
            world.add_relation(stats, 0x3c, Some(other), F32::ONE, precision)?;
        }
        if let (Some(stats), Some(other)) = (attacker_stats.relations, state.victim.owner) {
            world.add_relation(stats, 0x3c, Some(other), F32::ONE, precision)?;
        }
    }
    if state.victim.organization && !state.victim.componentd8 && state.victim.definition2c8_nonempty
    {
        pair(
            world,
            victim_stats.ordinary,
            attacker_stats.ordinary,
            13,
            12,
            precision,
        )?;
    }
    if state.victim.organization && !state.victim.componentd8 && state.victim.componenta4 {
        let victim_label = world.victim_label(state)?;
        let allegiance = world.allegiance(state)?;
        world.event(
            Event::SpecialNotification {
                allegiance,
                victim_label,
            },
            state,
        )?;
        pair(
            world,
            victim_stats.ordinary,
            attacker_stats.ordinary,
            16,
            15,
            precision,
        )?;
    }
    if state.victim.component94 && world.outer_actor(state)?.is_some() {
        pair(
            world,
            victim_stats.ordinary,
            attacker_stats.ordinary,
            22,
            21,
            precision,
        )?;
    }
    // 58EAE0 is a pure +98 presence test whose return value is ignored.
    let classification = if !state.victim.component9c
        || (state.victim.component94 && world.outer_actor(state)?.is_some())
    {
        0
    } else if state.victim.definition43c_nonempty {
        1
    } else if state.victim.definition4e0_nonzero {
        2
    } else if state.victim.component88 {
        3
    } else {
        4
    };
    match classification {
        1 => pair(
            world,
            victim_stats.ordinary,
            attacker_stats.ordinary,
            31,
            30,
            precision,
        )?,
        2 => pair(
            world,
            victim_stats.ordinary,
            attacker_stats.ordinary,
            41,
            40,
            precision,
        )?,
        3 => pair(
            world,
            victim_stats.ordinary,
            attacker_stats.ordinary,
            37,
            36,
            precision,
        )?,
        _ => {}
    }
    if state.settings.campaign_observer {
        world.event(Event::CampaignDeath, state)?;
    }
    if let Some(source) = source {
        let value_source = if state.victim.component7c {
            let fields = world.source_fields(source)?;
            if fields.component7c {
                source
            } else {
                fields.organization_leader.unwrap_or(source)
            }
        } else {
            source
        };
        let amount = experience_amount(value_source, state, world, precision)?;
        world.event(
            Event::AwardExperience {
                source,
                amount,
                argument1: true,
                argument2: true,
            },
            state,
        )?;
        if state.victim.organization {
            world.event(Event::OrganizationReward { source }, state)?;
        }
        if state.victim.component7c {
            world.event(Event::Component7cReward { source }, state)?;
        }
        if state.victim.componentc4 {
            world.event(Event::Componentc4Reward { source }, state)?;
        }
    }
    world.event(Event::DeathTrigger, state)?;
    Ok(source)
}
