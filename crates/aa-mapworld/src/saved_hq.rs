//! Native574040 on the actual inserted HQ component, with real load owners.
//! Slot30 is loaded directly; a truncated stream retains prior native stores.
use crate::{
    Error, Result,
    inventory::Reader,
    restore::{RestoreDefinitions, RestoredRecord},
    restore_components::{UpdatableRegistration, UpdatableTarget},
};
use aa_actors::{
    ActorId, IdentityRegistry,
    hq_state::{HqLog, HqState, HqWorld},
    organization_state::LoadError,
};
use aa_numeric::Precision;
/// Wire framing only; live restoration uses load() directly for error prefixes.
pub fn decode_prefix(bytes: &[u8]) -> Result<usize> {
    let mut r = Reader::new(bytes);
    let updatable = r.version(1)?;
    if updatable >= 1 {
        r.u32()?;
    }
    let version = r.version(3)?;
    r.u32()?; // signed badge index
    let count = r.u32()? as i32;
    if version == 1 {
        for _ in 0..count.max(0) {
            r.u32()?;
        }
    } else {
        for _ in 0..count.max(0) {
            if r.take(1)?[0] == 0 {
                continue;
            }
            if version >= 3 {
                r.u32()?;
                r.u32()?;
                r.u32()?;
            } else {
                r.take(1)?;
                r.u32()?;
            }
        }
    }
    Ok(r.at)
}
fn failure(e: impl std::fmt::Display) -> LoadError {
    LoadError::World(e.to_string())
}
/// Current actor+E8 is supplied from its constructed/load shell, never a player slot.
pub struct Owner<'a> {
    pub actor: ActorId,
    pub kingdom: Option<ActorId>,
    pub registry: &'a IdentityRegistry<RestoredRecord>,
    pub registrations: &'a mut Vec<UpdatableRegistration>,
    pub precision: Precision,
}
struct World<'a, 'b, D> {
    owner: Owner<'a>,
    definitions: &'b D,
}
impl<D: RestoreDefinitions> World<'_, '_, D> {
    fn nation(
        &self,
        kingdom: u32,
    ) -> std::result::Result<Option<&crate::restore::NamedDefinition>, LoadError> {
        if self.owner.kingdom.map(ActorId::raw) != Some(kingdom) {
            return Err(failure("HQ owner Kingdom identity differs"));
        }
        let Some(RestoredRecord::Kingdom(record)) = self
            .owner
            .registry
            .get(ActorId::from_raw(kingdom))
            .map(|entry| &entry.value)
        else {
            return Err(failure("HQ owner Kingdom is not currently registered"));
        };
        let shell = record
            .runtime
            .as_ref()
            .and_then(|r| r.derived.as_ref())
            .ok_or_else(|| failure("HQ owner Kingdom has no current constructor/load state"))?;
        match shell.references.get(&0x234) {
            Some(nation) => Ok(nation.as_ref()),
            None if shell.words.get(&0x234) == Some(&0) => Ok(None),
            None => Err(failure("HQ owner Kingdom+234 Nation field is unavailable")),
        }
    }
}
impl<D: RestoreDefinitions> HqWorld for World<'_, '_, D> {
    fn updatable_table_word0(&self) -> aa_actors::hq_state::Result<i32> {
        self.definitions.updatable_table_bound().map_err(failure)
    }
    fn updatable_slot_occupied(&self, id: i32) -> aa_actors::hq_state::Result<bool> {
        self.definitions
            .updatable_slot_occupied(id)
            .map_err(failure)
    }
    fn register_updatable(&mut self, id: i32) -> aa_actors::hq_state::Result<()> {
        let target = UpdatableTarget::Component {
            owner: self.owner.actor.raw(),
            slot: 30,
        };
        self.definitions
            .register_updatable(id, &target)
            .map_err(failure)?;
        self.owner.registrations.push(UpdatableRegistration {
            serialized_id: id,
            target,
        });
        Ok(())
    }
    fn owner_kingdom(&self) -> aa_actors::hq_state::Result<Option<u32>> {
        Ok(self.owner.kingdom.map(ActorId::raw))
    }
    fn nation_badge_count(&self, kingdom: u32) -> aa_actors::hq_state::Result<Option<i32>> {
        self.nation(kingdom)?
            .map(|nation| {
                self.definitions
                    .nation_badge_count(nation, self.owner.precision)
                    .map_err(failure)
                    .and_then(|n| i32::try_from(n).map_err(failure))
            })
            .transpose()
    }
    fn nation_name(&self, _kingdom: u32) -> aa_actors::hq_state::Result<Vec<u16>> {
        Err(failure(
            "HQ invalid-badge diagnostic requires actual NationData+10 text",
        ))
    }
    fn owner_name(&mut self) -> aa_actors::hq_state::Result<Vec<u16>> {
        Err(failure(
            "HQ invalid-badge diagnostic requires actual GActor4D99D0 display-name owner",
        ))
    }
    fn log(&mut self, _log: HqLog) -> aa_actors::hq_state::Result<()> {
        Err(failure("HQ invalid-badge diagnostic sink is not bound"))
    }
}
pub fn load(
    state: &mut HqState,
    bytes: &[u8],
    owner: Owner<'_>,
    definitions: &impl RestoreDefinitions,
) -> Result<usize> {
    state
        .load(bytes, &mut World { owner, definitions })
        .map_err(|e| {
            let text = e.to_string();
            match e {
                LoadError::Warning(_) | LoadError::Fatal(_) | LoadError::Exception(_) => {
                    Error::Format(crate::restore::load_warning_exception(text))
                }
                _ => Error::Format(text),
            }
        })
}
