//! Native Organization579A00/5797E0 over the in-progress saved component.
//!
//! The Organization and its offsets remain in their actual owned allocation.
//! Canonical layout/formation fields are read lazily at the native boundaries;
//! actual saved Element and Thing fields are read through generation checked
//! identities. Missing Mind execution remains an explicit failure after the
//! native preceding leader/offset stores.
use crate::catalog::Catalog;
use aa_actors::{ActorId, components::ComponentSlot, loading::ActorShell};
use aa_body::F32;
use aa_mapworld::restore::{OrganizationLeaderContext, RestoredComponent, RestoredRecord};
use aa_movement::{Error, Result, borrowed_formation::World};

#[cfg(test)]
#[path = "saved_organization_leader_tests.rs"]
mod tests;

struct Borrowed<'a, 'world, 'source> {
    catalog: &'a Catalog<'source>,
    context: OrganizationLeaderContext<'world>,
}

/// Execute native579A00 on this exact mutable Organization, retaining partial
/// stores on every later provider failure. A null ActorId is the native null.
pub fn set_leader(
    catalog: &Catalog<'_>,
    context: OrganizationLeaderContext<'_>,
    leader: ActorId,
) -> std::result::Result<(), String> {
    // A portable structural refusal, not a native malformed pointer claim.
    if context.organization.header.owner != context.owner {
        return Err("579A00 active Organization owner identity mismatch".into());
    }
    let pc = context.precision;
    let leader = (!leader.is_null()).then_some(leader);
    aa_movement::borrowed_formation::set_leader(&mut Borrowed { catalog, context }, leader, pc)
        .map_err(|e| e.to_string())
}

impl Borrowed<'_, '_, '_> {
    fn shell(&self, actor: ActorId) -> Result<&ActorShell<RestoredComponent>> {
        if actor == self.context.owner {
            return Ok(self.context.owner_actor);
        }
        self.context
            .registry
            .get(actor)
            .and_then(|entry| match &entry.value {
                RestoredRecord::Actor(entity) => Some(entity),
                _ => None,
            })
            .and_then(|entity| entity.shell.actor.as_ref())
            .ok_or_else(|| {
                Error::World(format!(
                    "579A00 actor {:#010x} is not a current restored Actor",
                    actor.raw()
                ))
            })
    }
    fn element(&self, actor: ActorId) -> Result<&aa_actors::ground_states::ElementState<String>> {
        match self.shell(actor)?.components.get(ComponentSlot::Element) {
            Some(RestoredComponent::Element(element)) if element.header.owner == actor => {
                Ok(element)
            }
            Some(RestoredComponent::Element(_)) => Err(Error::World(
                "5797E0 Element belongs to a different Actor".into(),
            )),
            Some(_) => Err(Error::World("5797E0 Element slot payload mismatch".into())),
            None => Err(Error::World("5797E0 required Element is absent".into())),
        }
    }
    fn offset_index(slot: i32, axis: usize) -> Result<(usize, usize)> {
        let index = usize::try_from(slot)
            .map_err(|_| Error::World("5797E0 negative offset slot".into()))?;
        if axis > 1 {
            return Err(Error::World("5797E0 offset axis outside XY pair".into()));
        }
        Ok((index, axis))
    }
    fn org_word(&self, offset: u32) -> Result<u32> {
        self.context
            .organization
            .words
            .get(&offset)
            .copied()
            .ok_or_else(|| Error::World(format!("Organization+{offset:X} is uninitialized")))
    }
}

impl World for Borrowed<'_, '_, '_> {
    type Actor = ActorId;
    type Positions = ();
    type Formation = u32;
    type Mind = ();

    fn leader(&mut self) -> Result<Option<ActorId>> {
        let word = self.org_word(0x40)?;
        Ok((word != 0).then(|| ActorId::from_raw(word)))
    }
    fn store_leader(&mut self, leader: Option<ActorId>) -> Result<()> {
        self.context
            .organization
            .words
            .insert(0x40, leader.map_or(0, ActorId::raw));
        Ok(())
    }
    fn element_slot(&mut self, actor: ActorId) -> Result<i32> {
        Ok(self.element(actor)?.slot_index)
    }
    fn member(&mut self, slot: i32) -> Result<Option<ActorId>> {
        let index = usize::try_from(slot)
            .map_err(|_| Error::World("5797E0 negative member slot".into()))?;
        let id = *self
            .context
            .organization
            .members
            .get(index)
            .ok_or_else(|| Error::World("5797E0 member slot outside actual array".into()))?;
        Ok((!id.is_null()).then_some(id))
    }
    fn primary_slot(&mut self) -> Result<i32> {
        let data = self
            .catalog
            .layout_data(self.context.precision)
            .map_err(Error::World)?;
        // This is already a canonical LayoutData identity, not an IDS request
        // to which translation should be applied for a second time.
        let id = &self.context.organization.layout.definition;
        let layout = data
            .layouts
            .iter()
            .find(|layout| layout.ids.eq_ignore_ascii_case(id))
            .ok_or_else(|| {
                Error::World(format!("5797E0 canonical LayoutData {id} is unavailable"))
            })?;
        Ok(layout.primary_slot)
    }
    fn formation(&mut self) -> Result<Option<u32>> {
        let word = self.org_word(0x74)?;
        Ok((word != 0).then_some(word))
    }
    fn formation_oriented(&mut self, formation: u32) -> Result<bool> {
        let data = self
            .catalog
            .layout_data(self.context.precision)
            .map_err(Error::World)?;
        let layout = data.layout_formation(formation).ok_or_else(|| {
            Error::World(format!(
                "5797E0 unknown current LayoutFormation {formation}"
            ))
        })?;
        let definition = layout
            .formation
            .and_then(|index| data.formations.get(index as usize))
            .ok_or_else(|| {
                Error::World("5797E0 LayoutFormation+C source definition is absent".into())
            })?;
        Ok(definition.organized)
    }
    fn owner_facing(&mut self) -> Result<[f32; 2]> {
        Ok([
            self.context.owner_thing.facing_bits[0],
            self.context.owner_thing.facing_bits[1],
        ]
        .map(f32::from_bits))
    }
    fn formation_offset(&mut self, slot: i32) -> Result<[f32; 2]> {
        // Native578250 rereads the CURRENT Organization+74 after its caller
        // obtained owner facing. The active borrow prevents replacement here.
        let formation = self.org_word(0x74)?;
        let data = self
            .catalog
            .layout_data(self.context.precision)
            .map_err(Error::World)?;
        let layout = data.layout_formation(formation).ok_or_else(|| {
            Error::World(format!(
                "578250 unknown current LayoutFormation {formation}"
            ))
        })?;
        let index = usize::try_from(slot)
            .map_err(|_| Error::World("578250 negative formation slot".into()))?;
        Ok(layout
            .positions
            .get(index)
            .ok_or_else(|| {
                Error::World("578250 formation slot outside actual source array".into())
            })?
            .offset_bits
            .map(f32::from_bits))
    }
    fn positions(&mut self) -> Result<()> {
        Ok(())
    }
    fn offset(&mut self, _: (), slot: i32, axis: usize) -> Result<f32> {
        let (index, axis) = Self::offset_index(slot, axis)?;
        Ok(f32::from_bits(
            self.context
                .organization
                .offsets_78
                .get(index)
                .ok_or_else(|| Error::World("5797E0 offset outside actual array".into()))?[axis]
                .0,
        ))
    }
    fn store_offset(&mut self, _: (), slot: i32, axis: usize, value: f32) -> Result<()> {
        let (index, axis) = Self::offset_index(slot, axis)?;
        self.context
            .organization
            .offsets_78
            .get_mut(index)
            .ok_or_else(|| Error::World("5797E0 offset store outside actual array".into()))?
            [axis] = F32(value.to_bits());
        Ok(())
    }
    fn element_attached(&mut self, actor: ActorId) -> Result<bool> {
        Ok(self.element(actor)?.flag_1c)
    }
    fn actor_position(&mut self, actor: ActorId, axis: usize) -> Result<f32> {
        if axis > 1 {
            return Err(Error::World("5797E0 position axis outside XY pair".into()));
        }
        if actor == self.context.owner {
            return Ok(f32::from_bits(self.context.owner_thing.position_bits[axis]));
        }
        let entity = self
            .context
            .registry
            .get(actor)
            .and_then(|entry| match &entry.value {
                RestoredRecord::Actor(entity) => Some(entity),
                _ => None,
            })
            .filter(|entity| entity.shell.actor.is_some())
            .ok_or_else(|| {
                Error::World(format!(
                    "5797E0 position Actor {:#010x} is unavailable",
                    actor.raw()
                ))
            })?;
        Ok(f32::from_bits(entity.shell.thing.position_bits[axis]))
    }
    fn owner_id(&mut self) -> Result<u32> {
        Ok(self.context.owner.raw())
    }
    fn actor_id(&mut self, actor: ActorId) -> Result<u32> {
        self.shell(actor)?;
        Ok(actor.raw())
    }
    fn owner_mind(&mut self) -> Result<Option<()>> {
        match self.context.owner_actor.components.get(ComponentSlot::Mind) {
            None => Ok(None),
            Some(RestoredComponent::Mind(_)) => Ok(Some(())),
            Some(_) => Err(Error::World(
                "579A00 owner Mind slot payload mismatch".into(),
            )),
        }
    }
    fn mind_leader_changed(&mut self, _: ()) -> Result<()> {
        Err(Error::World(
            "579A00 owner Mind virtual +114 execution is unported".into(),
        ))
    }
    fn next_random_u32(&mut self) -> Result<u32> {
        Ok(self.context.rng.next_u32())
    }
    fn detached_member(
        &mut self,
        organization_id: u32,
        member_id: u32,
        offset: [f32; 2],
    ) -> Result<()> {
        self.context
            .native_messages
            .push(aa_mapworld::restore::NativeMessage {
                source_ordinal: self.context.source_ordinal,
                function: 0x404f90,
                line: 0xab0,
                text: format!(
                    "org {} detached elem {} - leaving slot position as {:.6},{:.6}",
                    organization_id as i32, member_id as i32, offset[0], offset[1]
                ),
            });
        Ok(())
    }
}
