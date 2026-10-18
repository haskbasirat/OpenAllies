//! Native5788B0 over the actual in-progress saved Organization and owned
//! member components. This observes the restored prefix; it does not supply
//! missing per-record Actor Init or recompute an uninitialized Performer value.
use crate::catalog::Catalog;
use aa_actors::{ActorId, components::ComponentSlot, loading::ActorShell};
use aa_body::F32;
use aa_combat::{Error, Result};
use aa_mapworld::restore::{OrganizationCombatContext, RestoredComponent};
#[cfg(test)]
#[path = "saved_organization_combat_tests.rs"]
mod tests;

struct Borrowed<'a, 'world, 'source> {
    catalog: &'a Catalog<'source>,
    context: OrganizationCombatContext<'world>,
}
pub fn recompute(
    catalog: &Catalog<'_>,
    context: OrganizationCombatContext<'_>,
) -> std::result::Result<(), String> {
    //Structural host identity preflight is a portable refusal boundary, not
    //a claim about malformed native pointer fault/store timing.
    if context.organization.header.owner != context.owner {
        return Err("5788B0 active Organization owner identity mismatch".into());
    }
    let pc = context.precision;
    aa_combat::organization_combat_value::recompute(&mut Borrowed { catalog, context }, pc)
        .map_err(|e| e.to_string())
}
/// Immediate ActorInit5788B0: the read-only layout/mask source PC is the
/// completed Bank's actual source authority, separately from live arithmetic.
/// The older public diagnostic entry above keeps its original explicit-PC API.
pub(crate) fn recompute_at_source(
    catalog: &Catalog<'_>,
    mut context: OrganizationCombatContext<'_>,
    source_pc: aa_numeric::Precision,
) -> std::result::Result<(), String> {
    {
        let bank=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?;
        let source=bank.combat_source()?;
        if source.source_precision()!=source_pc || source.source_identities()!=catalog.actors.sources.as_slice() {
            return Err("5788B0 actual source Bank PC/identity differs".into());
        }
    }
    if context.organization.header.owner!=context.owner {
        return Err("5788B0 active Organization owner identity mismatch".into());
    }
    let live_pc=context.precision;
    //Borrowed uses this scalar only for immutable source layout/mask reads;
    //the exact arithmetic coordinator receives the separately retained live PC.
    context.precision=source_pc;
    aa_combat::organization_combat_value::recompute(&mut Borrowed{catalog,context},live_pc)
        .map_err(|e|e.to_string())
}
impl Borrowed<'_, '_, '_> {
    fn shell(&self, actor: ActorId) -> Result<&ActorShell<RestoredComponent>> {
        if actor == self.context.owner {
            return Ok(self.context.owner_actor);
        }
        self.context
            .registry
            .get(actor)
            .and_then(|entry| entry.value.loaded())
            .and_then(|entity| entity.shell.actor.as_ref())
            .ok_or_else(|| {
                Error::World(format!(
                    "5788B0 member {:#x} is not a current restored Actor",
                    actor.raw()
                ))
            })
    }
    fn performer(
        &self,
        actor: ActorId,
    ) -> Result<Option<&aa_combat::constructors::OwnedPerformer>> {
        match self.shell(actor)?.components.get(ComponentSlot::Performer) {
            None => Ok(None),
            Some(RestoredComponent::Performer(p)) if p.state.owner == actor.raw() => Ok(Some(p)),
            Some(RestoredComponent::Performer(_)) => Err(Error::World(
                "5788B0 Performer belongs to a different Actor".into(),
            )),
            Some(_) => Err(Error::World(
                "5788B0 Performer slot payload mismatch".into(),
            )),
        }
    }
    fn health(&self, performer_owner: ActorId) -> Result<Option<[F32; 2]>> {
        match self
            .shell(performer_owner)?
            .components
            .get(ComponentSlot::Body)
        {
            None => Ok(None),
            Some(RestoredComponent::Body(body)) if body.header.owner == performer_owner => {
                Ok(Some([body.body.health, body.body.maximum]))
            }
            Some(RestoredComponent::Body(_)) => Err(Error::World(
                "57DC40 Body belongs to a different Performer owner".into(),
            )),
            Some(_) => Err(Error::World("57DC40 Body slot payload mismatch".into())),
        }
    }
    fn parameter_mask(&self, canonical: &str) -> Result<u32> {
        self.catalog
            .canonical_component_mask(canonical, self.context.precision)
            .map_err(Error::World)
    }
}

impl aa_combat::organization_combat_value::World for Borrowed<'_, '_, '_> {
    //The exclusive active Organization borrow pins this exact member vector
    //throughout the call. No source lookup or scalar store can replace it.
    type Members = ();
    type Member = ActorId;
    type Performer = ActorId;
    type Slot = usize;
    fn member_count(&mut self) -> Result<i32> {
        i32::try_from(self.context.organization.members.len())
            .map_err(|_| Error::Invalid("5788B0 member count exceeds native signed range"))
    }
    fn member_array(&mut self) -> Result<()> {
        Ok(())
    }
    fn member(&mut self, _: (), index: i32) -> Result<Option<ActorId>> {
        let member = *self
            .context
            .organization
            .members
            .get(
                usize::try_from(index)
                    .map_err(|_| Error::Invalid("5788B0 negative member index"))?,
            )
            .ok_or(Error::Invalid("5788B0 member index outside actual vector"))?;
        Ok((!member.is_null()).then_some(member))
    }
    fn performer(&mut self, member: ActorId) -> Result<Option<ActorId>> {
        Ok(Borrowed::performer(self, member)?.map(|_| member))
    }
    fn performer_health(&mut self, performer: ActorId) -> Result<Option<[F32; 2]>> {
        let owner = Borrowed::performer(self, performer)?
            .ok_or(Error::Invalid("57DC40 Performer vanished"))?
            .state
            .owner;
        self.health(ActorId::from_raw(owner))
    }
    fn performer_value(&mut self, performer: ActorId) -> Result<F32> {
        Ok(F32(Borrowed::performer(self, performer)?
            .ok_or(Error::Invalid("5788B0 Performer value owner vanished"))?
            .word60))
    }
    fn slot(&mut self, index: i32) -> Result<Option<usize>> {
        let index =
            usize::try_from(index).map_err(|_| Error::Invalid("5788B0 negative slot index"))?;
        let info = aa_mapworld::restore::RestoreDefinitions::organization_layout_info(
            self.catalog,
            &self.context.organization.layout.definition,
            self.context.precision,
        )
        .map_err(Error::World)?;
        let group = *info.slot_groups.get(index).ok_or(Error::Invalid(
            "5788B0 slot group index outside current layout",
        ))? as usize;
        let value = self
            .context
            .organization
            .layout
            .parameters
            .get(group)
            .ok_or(Error::Invalid(
                "5788B0 parameter index outside saved layout",
            ))?;
        Ok(value.as_ref().map(|_| group))
    }
    fn slot_flags(&mut self, slot: usize) -> Result<u32> {
        let id = self
            .context
            .organization
            .layout
            .parameters
            .get(slot)
            .and_then(Option::as_deref)
            .ok_or(Error::Invalid("5788B0 selected parameter vanished"))?;
        self.parameter_mask(id)
    }
    fn slot_value(&mut self, slot: usize) -> Result<F32> {
        let id = self
            .context
            .organization
            .layout
            .parameters
            .get(slot)
            .and_then(Option::as_deref)
            .ok_or(Error::Invalid("5788B0 selected value parameter vanished"))?;
        self.catalog.completed_actor_combat_values(id)
            .map(|values| values.value_1e4)
            .map_err(|error| Error::World(format!(
                "4618B0 completed canonical ActorData+1E4 source value is unavailable for {id}: {error}"
            )))
    }
    fn current(&mut self) -> Result<F32> {
        self.context
            .organization
            .words
            .get(&0x68)
            .copied()
            .map(F32)
            .ok_or(Error::Invalid("Organization+68 uninitialized"))
    }
    fn maximum(&mut self) -> Result<F32> {
        self.context
            .organization
            .words
            .get(&0x6c)
            .copied()
            .map(F32)
            .ok_or(Error::Invalid("Organization+6C uninitialized"))
    }
    fn store_current(&mut self, value: F32) -> Result<()> {
        self.context.organization.words.insert(0x68, value.0);
        Ok(())
    }
    fn store_maximum(&mut self, value: F32) -> Result<()> {
        self.context.organization.words.insert(0x6c, value.0);
        Ok(())
    }
}
