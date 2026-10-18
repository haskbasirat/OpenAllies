//! Fresh Actor4CF300 on the same staged allocation and nonnull Params.
//! The existing native caller controls component rereads and owner order.
//! Initial Properties use the moved instance arena; no saved LoadShell is made.
use crate::{
    actor_world::{ObjectKey, RuntimeComponent, RuntimeObject},
    catalog::Catalog,
    load_post_create_world::InitialProperty,
    map_actor_creation::Params,
    restoration::Staged,
};
use aa_actors::{
    ActorId,
    components::ComponentSlot as Slot,
    world_publication::{self, CreateWorld, Error},
};
use aa_numeric::Precision;
use aa_rules::native::layout_data::{self, PropertyRef};

type Result<T> = std::result::Result<T, String>;
fn text(e: impl std::fmt::Display) -> String {
    e.to_string()
}
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}

///Read the current allocation through its generation and the SAME moved Bank.
pub(crate) fn canonical<'a>(
    staged: &'a Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
) -> Result<&'a RuntimeObject> {
    catalog.require_source_owner()?;
    let source = staged.definitions.combat_source()?;
    source.completed_means()?;
    let id = staged
        .actors
        .id(key)
        .ok_or("4CF300 actual current GTID absent")?;
    if id.is_null() || staged.actors.key(id) != Some(key) {
        return Err("4CF300 actual generation differs".into());
    }
    let runtime = staged.actors.object(key).map_err(text)?;
    crate::live_object_init::Source::bind_in_bank(
        catalog,
        &staged.definitions,
        &runtime.definition,
    )
    .map_err(text)?;
    if !matches!(runtime.definition.native_class, 3 | 4)
        || runtime.kind != aa_actors::loading::LoadKind::Actor
        || runtime.object.is_none()
        || runtime.actor.is_none()
        || runtime.definition.proof.category.native_kind != 2
        || runtime.definition.component_mask
            != Some(catalog.canonical_component_mask(
                &runtime.definition.proof.id,
                source.source_precision(),
            )?)
        || !crate::load_spatial::same_proof(&runtime.thing.definition, &runtime.definition.proof)
    {
        return Err("4CF300 actual Actor constructor/source/storage differs".into());
    }
    Ok(runtime)
}
#[cfg(test)]
#[path = "fresh_post_create_world_tests.rs"]
mod tests;

///Actual4FA030 caller supplies the retained creation block and full flag word.
///A reached unported child stops without undoing prior native stores.
pub fn post_create(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    params: &Params,
    flag: u32,
    pc: Precision,
) -> Result<()> {
    post_create_with_ground_scratch(staged, catalog, key, params, flag, None, pc)
}
pub(crate) fn post_create_with_ground_scratch(
    staged: &mut Staged,
    catalog: &Catalog<'_>,
    key: ObjectKey,
    params: &Params,
    flag: u32,
    ground_scratch: Option<crate::map_object_ground::CreationGroundScratch>,
    pc: Precision,
) -> Result<()> {
    post_create_with_startup_and_ground_scratch(staged,catalog,key,params,flag,None,ground_scratch,pc)
}
pub(crate) fn post_create_with_startup_and_ground_scratch(
    staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,params:&Params,flag:u32,
    startup:Option<&crate::actor_creation_startup::StartupInputs>,
    ground_scratch:Option<crate::map_object_ground::CreationGroundScratch>,pc:Precision,
) -> Result<()> {
    let runtime = canonical(staged, catalog, key)?;
    if runtime.definition.native_class != 3 {
        return Err("fresh4CF300 requires ordinary class3".into());
    }
    if !matches!(
        staged.actors.entity(key).map_err(text)?.source,
        crate::actor_world::Source::New { .. }
    ) {
        return Err("fresh4CF300 requires the actual new Actor allocation".into());
    }
    let token = staged.definitions.require_usable(params.definition.token)?;
    if token.key.domain != "actordefinition"
        || !token
            .key
            .name
            .eq_ignore_ascii_case(&runtime.definition.proof.id)
        || !params
            .definition
            .canonical_ids
            .eq_ignore_ascii_case(&runtime.definition.proof.id)
    {
        return Err("fresh4CF300 Params definition and actual Actor differ".into());
    }
    let id = staged
        .actors
        .id(key)
        .ok_or("fresh4CF300 current GTID absent")?;
    world_publication::post_create(
        &mut Current {
            staged,
            catalog,
            key,
            id,
            pc,
            ground_scratch,
            startup,
        },
        id,
        params,
        flag,
    )
    .map_err(text)
}
struct Current<'a, 's> {
    staged: &'a mut Staged,
    catalog: &'a Catalog<'s>,
    key: ObjectKey,
    id: ActorId,
    pc: Precision,
    ground_scratch: Option<crate::map_object_ground::CreationGroundScratch>,
    startup:Option<&'a crate::actor_creation_startup::StartupInputs>,
}
impl Current<'_, '_> {
    fn check(&self, id: ActorId) -> world_publication::Result<&RuntimeObject> {
        if id != self.id || self.staged.actors.key(id) != Some(self.key) {
            return Err(error("4CF300 current generation changed"));
        }
        self.staged.actors.object(self.key).map_err(error)
    }
}
impl CreateWorld for Current<'_, '_> {
    type Record = Params;
    fn has_component(&self, id: ActorId, slot: Slot) -> world_publication::Result<bool> {
        Ok(self.check(id)?.components().map_err(error)?.present(slot))
    }
    fn map_mode_a8(&self) -> world_publication::Result<u32> {
        self.staged
            .load_world
            .as_ref()
            .ok_or_else(|| error("4EBA40 actual World absent"))?
            .map_mode
            .as_ref()
            .as_ref()
            .copied()
            .ok_or_else(|| error("4EBA40 actual MapResource+A8 unbound"))
    }
    fn object_ground_post_create(
        &mut self,
        id: ActorId,
        params: &Params,
    ) -> world_publication::Result<()> {
        self.check(id)?;
        crate::map_object_ground::run(
            self.staged,
            self.catalog,
            self.key,
            params,
            self.ground_scratch.take(),
            self.pc,
        )
        .map_err(error)
    }
    fn component_post_create(
        &mut self,
        id: ActorId,
        slot: Slot,
        params: &Params,
        flag: u32,
    ) -> world_publication::Result<()> {
        let c = self
            .check(id)?
            .components()
            .map_err(error)?
            .get(slot)
            .ok_or_else(|| error("4CF300 current component vanished"))?;
        if c.slot() != slot {
            return Err(error("4CF300 component payload/slot differs"));
        }
        if slot == Slot::City && flag as u8 == 0 {
            if !matches!(c,RuntimeComponent::City(c)if c.header.owner==id) {
                return Err(error("564BE0 actual City owner differs"));
            }
            return crate::fresh_city_post_create::run(self.staged,self.catalog,self.key,params.word_34,true,self.startup,self.pc).map_err(error);
        }
        if slot != Slot::Properties {
            return Ok(());
        } //actual other bound +C leaves RET8
        if !matches!(c,RuntimeComponent::Properties(p)if p.owner.0==id.raw()) {
            return Err(error("581880 actual Properties allocation/owner differs"));
        }
        let source_pc = self
            .staged
            .definitions
            .combat_source()
            .map_err(error)?
            .source_precision();
        let data = self.catalog.layout_data(source_pc).map_err(error)?;
        let definition = &self.check(id)?.definition.proof.id;
        let properties =
            layout_data::native_actor_properties(self.catalog.actors, &data, definition, source_pc)
                .map_err(error)?;
        for reference in properties {
            let sources = match reference {
                PropertyRef::Registered(i) => data
                    .properties
                    .get(i as usize)
                    .ok_or_else(|| error("581880 registered source absent"))?
                    .sources
                    .clone(),
                PropertyRef::Embedded(_) => {
                    return Err(error(
                        "581880 ActorData initial property requires registered source",
                    ));
                }
            };
            crate::staged_property_instances::initial(
                self.staged,
                self.catalog,
                self.key,
                &InitialProperty { reference, sources },
                self.pc,
            )
            .map_err(error)?;
        }
        Ok(())
    }
    fn stored_owner(&self, id: ActorId) -> world_publication::Result<Option<ActorId>> {
        let value = *self
            .check(id)?
            .actor
            .as_ref()
            .ok_or_else(|| error("4CF300 Actor absent"))?
            .words
            .get(&0xe8)
            .ok_or_else(|| error("4CF300 actual Actor+E8 unwritten"))?;
        Ok((value != 0).then(|| ActorId::from_raw(value)))
    }
    fn owner_post_create_4e6180(
        &mut self,
        owner: ActorId,
        id: ActorId,
    ) -> world_publication::Result<()> {
        self.check(id)?;
        crate::stored_owner_add_actor::add_actor(
            self.staged,
            self.catalog,
            self.key,
            owner,
            self.pc,
        )
        .map_err(error)
    }
    fn definition_mask(&self, id: ActorId) -> world_publication::Result<u32> {
        self.check(id)?
            .definition
            .component_mask
            .ok_or_else(|| error("4CF300 actual component mask absent"))
    }
    fn city_predicate_45d6a0(&mut self, id: ActorId) -> world_publication::Result<bool> {
        let definition = &self.check(id)?.definition.proof.id;
        let source_pc = self
            .staged
            .definitions
            .combat_source()
            .map_err(error)?
            .source_precision();
        let bits = self
            .catalog
            .actors
            .native_city_zone_radius(definition, source_pc)
            .map_err(error)?
            .ok_or_else(|| error("45D6A0 actual source City+5B4 absent"))?;
        Ok(f32::from_bits(bits) > 0.0) //45D6A0 compares source+5B4 with native0
    }
    fn city_argument_34(&self, params: &Params) -> world_publication::Result<u32> {
        Ok(params.word_34)
    }
    fn city_post_create_5644f0(&mut self, id: ActorId, arg: u32) -> world_publication::Result<()> {
        self.check(id)?;
        crate::fresh_city_post_create::run(self.staged,self.catalog,self.key,arg,false,self.startup,self.pc).map_err(error)
    }
}
