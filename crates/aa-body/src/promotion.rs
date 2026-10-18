//! Owned ExperienceComponent state and native rank transitions571690/5716B0/
//! 571A60/571C50. Definitions retain their original list order and explicit rank
//! index; no thresholds, current rank, modifiers or actors are invented.
use crate::{
    Error, F32, Result,
    credit::{self, Object},
};
use aa_numeric::Precision;
use serde::{Deserialize, Serialize};
use std::{cmp::Ordering, collections::BTreeMap};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExperienceState {
    pub owner: Object,
    pub experience: F32,
    pub current_rank: Option<Object>,
}
impl ExperienceState {
    /// Original constructor571690 initializes no rank. Apply requires the real
    /// factory's subsequent rank assignment; null is not silently rank zero.
    pub fn new(owner: Object) -> Self {
        Self {
            owner,
            experience: F32::ZERO,
            current_rank: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rank {
    pub threshold: F32,
    /// Rank+4, used to select definition's next entry, not inferred from identity.
    pub index: i32,
    pub modifier: Option<Object>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RankTable {
    pub entries: Vec<Option<Object>>,
    pub definitions: BTreeMap<Object, Rank>,
}
impl RankTable {
    fn get(&self, id: Object) -> Result<&Rank> {
        self.definitions
            .get(&id)
            .ok_or_else(|| Error::Callback(format!("missing rank definition {}", id.0)))
    }
    /// Exact45DB60 next-index policy, with explicit invalid storage errors.
    pub fn next(&self, current: Option<Object>) -> Result<Option<Object>> {
        let current = current.ok_or_else(|| {
            Error::Callback("ExperienceComponent current rank was not initialized".into())
        })?;
        let next = self.get(current)?.index.wrapping_add(1);
        let count = i32::try_from(self.entries.len())
            .map_err(|_| Error::Callback("rank table exceeds native signed index domain".into()))?;
        if next >= count {
            return Ok(None);
        }
        let index = usize::try_from(next)
            .map_err(|_| Error::Callback("negative next rank index".into()))?;
        let result = *self
            .entries
            .get(index)
            .ok_or_else(|| Error::Callback("invalid rank table index".into()))?;
        if let Some(id) = result {
            self.get(id)?;
        }
        Ok(result)
    }
}

/// Live actor fields used by promotion. +7C is Organization/regiment, +80 is
/// Element, and+D8 suppresses ordinary classifications. The cap is the optional
/// Element+14 definition's+18 scalar, not a fabricated universal level limit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActorFields {
    pub regiment: bool,
    pub element: bool,
    pub experience_component: bool,
    pub componentd8: bool,
    pub element_experience_cap: Option<F32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct State {
    pub component: ExperienceState,
    pub ranks: RankTable,
    pub actor: ActorFields,
}

/// Effects act synchronously on concrete state. Runtime adapters apply the
/// actual modifier/actor/organization effects, and can change rank or XP during
/// a callback. Earlier state writes are not rolled back after an error.
pub trait Effects {
    fn remove_modifier(&mut self, modifier: Option<Object>, state: &mut State) -> Result<()>;
    fn add_modifier(
        &mut self,
        modifier: Option<Object>,
        kind: u32,
        state: &mut State,
    ) -> Result<bool>;
    fn rank_changed(
        &mut self,
        old: Option<Object>,
        new: Option<Object>,
        state: &mut State,
    ) -> Result<()>;
    fn actor_label(&mut self, state: &mut State) -> Result<String>;
    fn allegiance(&mut self, state: &mut State) -> Result<Option<Object>>;
    fn notification(
        &mut self,
        kind: u32,
        allegiance: Option<Object>,
        label: String,
        state: &mut State,
    ) -> Result<()>;
    /// Native4CEB70; notify regiment members and root actor's allegiance using
    /// the original award amount, even when the local XP total was capped.
    fn propagate_experience(&mut self, amount: F32, state: &mut State) -> Result<()>;
    /// Direct score storage of allegiance+32C, with no callbacks or defaults.
    /// The actual component may be absent; invalid identities must be errors.
    fn statistics(&mut self, allegiance: Object) -> Result<Option<&mut [F32]>>;
}

pub fn set_rank<E: Effects>(
    state: &mut State,
    requested: Option<Object>,
    effects: &mut E,
) -> Result<()> {
    let old = state.component.current_rank;
    if let Some(old) = old {
        effects.remove_modifier(state.ranks.get(old)?.modifier, state)?;
    }
    state.component.current_rank = requested;
    if let Some(new) = requested {
        let modifier = state.ranks.get(new)?.modifier;
        effects.add_modifier(modifier, 7, state)?;
    }
    effects.rank_changed(old, state.component.current_rank, state)
}

pub fn reset_and_apply<E: Effects>(
    state: &mut State,
    amount: F32,
    notify: bool,
    precision: Precision,
    effects: &mut E,
) -> Result<()> {
    state.component.experience = F32::ZERO;
    apply(state, amount, notify, precision, effects)
}

/// Native571A60: increment, optional cap, every crossed rank in list order,
/// propagation and positive-award statistics. Negative awards do not demote.
pub fn apply<E: Effects>(
    state: &mut State,
    amount: F32,
    notify: bool,
    precision: Precision,
    effects: &mut E,
) -> Result<()> {
    state.component.experience = F32(amount
        .number()?
        .add(&state.component.experience.number()?, precision)?
        .store_f32()?);
    if state.actor.element
        && let Some(cap) = state.actor.element_experience_cap
        && cap.compare(F32::ZERO)? == Ordering::Greater
        && cap.compare(state.component.experience)? == Ordering::Less
    {
        state.component.experience = cap;
    }
    let mut next = state.ranks.next(state.component.current_rank)?;
    while let Some(rank) = next {
        if state
            .ranks
            .get(rank)?
            .threshold
            .compare(state.component.experience)?
            == Ordering::Greater
        {
            break;
        }
        let old = state.component.current_rank;
        if let Some(old) = old {
            effects.remove_modifier(state.ranks.get(old)?.modifier, state)?;
        }
        state.component.current_rank = Some(rank);
        let modifier = state.ranks.get(rank)?.modifier;
        effects.add_modifier(modifier, 7, state)?;
        effects.rank_changed(old, state.component.current_rank, state)?;
        if notify {
            let kind = if state.actor.regiment { 2 } else { 0 };
            let label = effects.actor_label(state)?;
            let allegiance = effects.allegiance(state)?;
            effects.notification(kind, allegiance, label, state)?;
        }
        next = state.ranks.next(state.component.current_rank)?;
    }
    effects.propagate_experience(amount, state)?;
    let allegiance = effects.allegiance(state)?;
    if let Some(allegiance) = allegiance
        && let Some(stats) = effects.statistics(allegiance)?
        && amount.compare(F32::ZERO)? == Ordering::Greater
    {
        let indices = if state.actor.regiment && !state.actor.componentd8 {
            Some((6, 7))
        } else if state.actor.element
            && !state.actor.componentd8
            && state.actor.experience_component
        {
            Some((17, 18))
        } else {
            None
        };
        if let Some((total, maximum)) = indices {
            let total = stats
                .get_mut(total)
                .ok_or_else(|| Error::Callback("missing total-experience statistic".into()))?;
            credit::increment_stat(total, amount, precision)?;
            let maximum = stats
                .get_mut(maximum)
                .ok_or_else(|| Error::Callback("missing maximum-experience statistic".into()))?;
            if maximum.compare(state.component.experience)? == Ordering::Less {
                *maximum = state.component.experience;
            }
        }
    }
    Ok(())
}
