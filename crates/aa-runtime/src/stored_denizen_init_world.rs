//!Actual5687E0/568070 over the same fresh ActorStore Denizen and source Bank.
//!Uses the held source/ordered merge/AfterInit kernels. No LoadShell conversion.
use crate::{
    actor_world::{ObjectKey, RuntimeComponent},
    catalog::{ActorBindings, Catalog},
    restoration::Staged,
};
use aa_actors::{
    actor_creation::{Error, Result},
    components::ComponentSlot as Slot,
    ActorId,
};
use aa_numeric::Precision;
use std::{collections::BTreeMap, sync::Arc};
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn denizen(staged: &Staged, id: ActorId) -> Result<&aa_actors::denizen_state::DenizenState> {
    match staged.actors.component(id, Slot::Denizen).map_err(error)? {
        Some(RuntimeComponent::Denizen(d)) if d.header.owner == id => Ok(d),
        _ => Err(error(
            "stored5687E0 actual Denizen allocation/owner differs",
        )),
    }
}
fn checked(
    staged: &Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
) -> Result<(ActorId, Arc<ActorBindings>)> {
    let id = staged
        .actors
        .id(key)
        .ok_or_else(|| error("stored5687E0 current GTID absent"))?;
    if staged.actors.key(id) != Some(key) {
        return Err(error("stored5687E0 actual generation differs"));
    }
    let object = staged.actors.object(key).map_err(error)?;
    if object.kind != aa_actors::loading::LoadKind::Actor
        || object.definition.native_class != 3
        || object.actor.is_none()
        || object.object.is_none()
        || !crate::load_spatial::same_proof(&object.thing.definition, &object.definition.proof)
    {
        return Err(error("stored5687E0 actual Actor constructor/proof differs"));
    }
    let source = crate::live_object_init::Source::bind_in_bank(
        catalog,
        &staged.definitions,
        &object.definition,
    )?;
    let binding =
        crate::map_component_init_create::actual_binding(staged, catalog, key).map_err(error)?;
    if source.source_precision() != binding.precision
        || !crate::load_spatial::same_proof(&object.definition.proof, &binding.definition.proof)
        || object.definition.flags != binding.definition.flags
        || object.definition.component_mask != binding.definition.component_mask
        || object
            .definition
            .component_mask
            .is_none_or(|m| m & (1 << Slot::Denizen.index()) == 0)
    {
        return Err(error("stored5687E0 actual source factory differs"));
    }
    let leaves = aa_actors::component_virtuals::leaves(
        aa_actors::init_create::FactoryKind::Standard(Slot::Denizen),
    );
    if leaves.init_existing != 0x5687e0 || leaves.vf14 != 0x568070 {
        return Err(error("stored Denizen actual virtual8/14 differs"));
    }
    denizen(staged, id)?;
    Ok((id, binding))
}
fn guard(binding: &ActorBindings) -> Result<u32> {
    binding
        .source_ground
        .denizen_guards
        .as_ref()
        .map(|d| d.finalized_bits[0])
        .ok_or_else(|| error("stored568070 actual DenizenData364 source absent"))
}
struct Merge<'a, 'source> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'source>,
    id: ActorId,
    binding: &'a ActorBindings,
    rows: BTreeMap<u32, Vec<u32>>,
}
impl crate::load_denizen_init_world::MaskWorld for Merge<'_, '_> {
    type Node = (u32, usize);
    fn head(&mut self, list: u32) -> Result<Option<Self::Node>> {
        self.rows.insert(
            list,
            crate::load_denizen_init_world::source_masks(
                self.catalog,
                &self.binding.definition.proof.id,
                self.binding.precision,
                list,
            )?,
        );
        Ok((!self.rows[&list].is_empty()).then_some((list, 0)))
    }
    fn value(&mut self, (list, index): Self::Node) -> Result<u32> {
        self.rows
            .get(&list)
            .and_then(|v| v.get(index))
            .copied()
            .ok_or_else(|| error("stored5687E0 actual source node absent"))
    }
    fn next(&mut self, (list, index): Self::Node) -> Result<Option<Self::Node>> {
        let values = self
            .rows
            .get(&list)
            .ok_or_else(|| error("stored5687E0 source head not read"))?;
        Ok((index + 1 < values.len()).then_some((list, index + 1)))
    }
    fn or_mask(&mut self, bits: u32) -> Result<()> {
        match self
            .staged
            .actors
            .component_mut(self.id, Slot::Denizen)
            .map_err(error)?
        {
            Some(RuntimeComponent::Denizen(d)) if d.header.owner == self.id => {
                let word = d
                    .words
                    .get_mut(&0x38)
                    .ok_or_else(|| error("stored5687E0 initialized Denizen38 absent"))?;
                *word |= bits;
                Ok(())
            }
            _ => Err(error(
                "stored5687E0 current Denizen allocation/owner changed",
            )),
        }
    }
}
pub fn init_existing(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    pc: Precision,
) -> Result<bool> {
    let (id, binding) = checked(staged, catalog, key)?;
    crate::load_denizen_init_world::merge(&mut Merge {
        staged,
        catalog,
        id,
        binding: &binding,
        rows: BTreeMap::new(),
    })?;
    let mut world = crate::stored_component_schedule::World::new(staged, key, Slot::Denizen, pc)?;
    crate::load_detection_init_world::initialize_when(&mut world, || {
        if f32::from_bits(guard(&binding)?) > 0.0 {
            return Ok(true);
        }
        //Data35C is the authored head, not current runtime Inventory count.
        Ok(!binding
            .source_ground
            .denizen_guards
            .as_ref()
            .ok_or_else(|| error("stored568848 source Inventory absent"))?
            .inventory
            .is_empty())
    })
}
struct After<'a> {
    staged: &'a mut Staged,
    key: ObjectKey,
    id: ActorId,
    binding: &'a ActorBindings,
}
impl After<'_> {
    fn placement(&self) -> Result<crate::actor_world::PlacementState> {
        if let Ok(p) = self.staged.actors.staged_placement(self.key) {
            return Ok(p);
        }
        let world = self
            .staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("stored568070 actual published World absent"))?;
        self.staged
            .actors
            .placement(self.key, world.live_spatial().map_err(error)?)
            .map_err(error)
    }
}
impl crate::load_denizen_init_world::AfterWorld for After<'_> {
    fn guard(&mut self) -> Result<u32> {
        guard(self.binding)
    }
    fn flags(&mut self) -> Result<u32> {
        Ok(self.placement()?.flags)
    }
    fn create(&mut self) -> Result<bool> {
        Err(error(
            "stored568099 actual shared CreateZone5 allocation/activation owner required",
        ))
    }
    fn mask(&mut self) -> Result<u32> {
        denizen(self.staged, self.id)?
            .words
            .get(&0x38)
            .copied()
            .ok_or_else(|| error("stored5680A0 current Denizen38 absent"))
    }
    fn zone_present(&mut self) -> Result<bool> {
        Err(error(
            "stored5680A6 actual Actor Zone5 vector owner required",
        ))
    }
    fn zone_flag(&mut self) -> Result<()> {
        Err(error("stored5680C3 actual Zone5 flag owner required"))
    }
    fn position(&mut self) -> Result<[u32; 2]> {
        let p = self.placement()?;
        Ok([p.position_bits[0], p.position_bits[1]])
    }
    fn move_zone(&mut self, _: [u32; 2], _: u32) -> Result<()> {
        Err(error(
            "stored5680FF actual same Zone5 move/activation owner required",
        ))
    }
}
pub fn after_init(
    catalog: &Catalog<'_>,
    staged: &mut Staged,
    key: ObjectKey,
    _pc: Precision,
) -> Result<()> {
    let (id, binding) = checked(staged, catalog, key)?;
    crate::load_denizen_init_world::after(&mut After {
        staged,
        key,
        id,
        binding: &binding,
    })
}
