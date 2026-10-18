//! Native Actor/Kingdom/Team relation lookup4CF040/4E52D0/4E10F0.
//! Runtime references are generation-bearing handles, never header names.
use aa_actors::ActorId;
use aa_mapworld::WorldInventory;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("missing or invalid relation source: {0}")]
    Source(String),
    #[error("missing live relation identity {0:#x}")]
    Identity(u32),
    #[error("relation vector is missing native slot {0}")]
    Slot(u32),
}
pub type Result<T> = std::result::Result<T, Error>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Team {
    pub slot: u32,
    pub kingdoms: Vec<u32>,
    pub teams: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Kingdom {
    pub slot: u32,
    pub team: Option<ActorId>,
    pub kingdoms: Vec<u32>,
    pub teams: Vec<u32>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Relations {
    /// Native mutable globals B28158/B2815C, supplied from verified startup.
    pub orphan_relation: u32,
    pub team_null_relation: u32,
    pub teams: BTreeMap<ActorId, Team>,
    pub kingdoms: BTreeMap<ActorId, Kingdom>,
}
impl Relations {
    /// Bind exact saved relation vectors. This is not Kingdom construction or
    /// world publication and it does not infer diplomacy from displayed teams.
    pub fn from_inventory(
        input: &WorldInventory,
        orphan_relation: u32,
        team_null_relation: u32,
    ) -> Result<Self> {
        let teams = input
            .teams
            .as_ref()
            .ok_or_else(|| Error::Source("TEAMS not decoded".into()))?;
        let kingdoms = input
            .kingdoms
            .as_ref()
            .ok_or_else(|| Error::Source("KINGDOMS not decoded".into()))?;
        if !input.team_generation_mismatches.is_empty()
            || !input.kingdom_generation_mismatches.is_empty()
        {
            return Err(Error::Source(
                "saved relation identity generation mismatch".into(),
            ));
        }
        let mut out = Self {
            orphan_relation,
            team_null_relation,
            teams: BTreeMap::new(),
            kingdoms: BTreeMap::new(),
        };
        for slot in &teams.slots {
            if let Some(t) = &slot.record {
                let id = ActorId::from_raw(t.serialized_id);
                if out
                    .teams
                    .insert(
                        id,
                        Team {
                            slot: t.slot_index,
                            kingdoms: t.words_4c.clone(),
                            teams: t.words_58.clone(),
                        },
                    )
                    .is_some()
                {
                    return Err(Error::Source("duplicate saved Team identity".into()));
                }
            }
        }
        for slot in &kingdoms.slots {
            if let Some(k) = &slot.record {
                let p = &k.payload;
                let raw = p.team_handle.ok_or_else(|| {
                    Error::Source(
                        "legacy Kingdom team state requires its constructor default".into(),
                    )
                })?;
                let team = (raw != 0).then(|| ActorId::from_raw(raw));
                if let Some(team) = team
                    && !out.teams.contains_key(&team)
                {
                    return Err(Error::Identity(team.raw()));
                }
                let values = p.words_1dc.as_ref().ok_or_else(|| {
                    Error::Source("legacy Kingdom team relations require initialization".into())
                })?;
                let id = ActorId::from_raw(k.serialized_id);
                if out
                    .kingdoms
                    .insert(
                        id,
                        Kingdom {
                            slot: p.word_188,
                            team,
                            kingdoms: p.words_1d0.values.clone(),
                            teams: values.values.clone(),
                        },
                    )
                    .is_some()
                {
                    return Err(Error::Source("duplicate saved Kingdom identity".into()));
                }
            }
        }
        Ok(out)
    }
    fn kingdom(&self, id: ActorId) -> Result<&Kingdom> {
        self.kingdoms.get(&id).ok_or(Error::Identity(id.raw()))
    }
    fn team(&self, id: ActorId) -> Result<&Team> {
        self.teams.get(&id).ok_or(Error::Identity(id.raw()))
    }
    fn value(values: &[u32], slot: u32) -> Result<u32> {
        values.get(slot as usize).copied().ok_or(Error::Slot(slot))
    }
    /// Native4CF040 compares the owner pointer to the passed allegiance first,
    /// including the null/null case, before calling Kingdom virtual+144.
    pub fn actor_relation(&self, owner: Option<ActorId>, other: Option<ActorId>) -> Result<u32> {
        if owner == other {
            return Ok(0);
        }
        match owner {
            Some(id) => self.kingdom_relation(id, other),
            None => Ok(self.orphan_relation),
        }
    }
    pub fn kingdom_relation(&self, id: ActorId, other: Option<ActorId>) -> Result<u32> {
        if Some(id) == other {
            return Ok(0);
        }
        let Some(other) = other else {
            return Ok(self.orphan_relation);
        };
        let current = self.kingdom(id)?;
        if let Some(team) = current.team {
            return self.team_relation(team, Some(other));
        }
        let target = self.kingdom(other)?;
        if let Some(team) = target.team {
            Self::value(&current.teams, self.team(team)?.slot)
        } else {
            Self::value(&current.kingdoms, target.slot)
        }
    }
    pub fn team_relation(&self, id: ActorId, other: Option<ActorId>) -> Result<u32> {
        let Some(other) = other else {
            return Ok(self.team_null_relation);
        };
        let target = self.kingdom(other)?;
        let current = self.team(id)?;
        if let Some(team) = target.team {
            Self::value(&current.teams, self.team(team)?.slot)
        } else {
            Self::value(&current.kingdoms, target.slot)
        }
    }
    /// Native4E52A0/4E5280 compare the returned relation word exactly.
    pub fn hostile(code: u32) -> bool {
        code == 3
    }
    pub fn allied(code: u32) -> bool {
        code == 2
    }
}
