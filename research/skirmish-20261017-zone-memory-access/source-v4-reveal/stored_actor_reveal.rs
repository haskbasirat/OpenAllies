//!4EC010 over the same stored Object fields during a disjoint active-Zone loan.
//!The coordinator is shared with held-load Actor64. Prefix writes are immediate.
use crate::{
    catalog::Catalog, definitions::Bank, load_kingdom_init_world::KingdomRegistryState,
    load_zone_services::Context, stored_memory_access::Access,
};
use aa_actors::{
    ActorId,
    actor_creation::{Error, Result},
};
use std::collections::BTreeSet;
fn error(e: impl std::fmt::Display) -> Error {
    Error::World(e.to_string())
}
fn validate(catalog: &Catalog<'_>, bank: &Bank, access: &impl Access, id: ActorId) -> Result<()> {
    let r = access.read(id)?;
    crate::live_object_init::Source::bind_in_bank(catalog, bank, r.definition)?;
    let valid = match r.definition.native_class {
        2 | 6 | 7 | 9 => r.kind == aa_actors::loading::LoadKind::Object && r.actor.is_none(),
        3 | 4 => r.kind == aa_actors::loading::LoadKind::Actor && r.actor.is_some(),
        _ => false,
    };
    if !valid || !crate::load_spatial::same_proof(&r.thing.definition, &r.definition.proof) {
        return Err(error("stored Actor64 actual constructor/source differs"));
    }
    access.placement(id)?;
    Ok(())
}
struct Owned<'a, 'source, 'services, A: Access> {
    catalog: &'a Catalog<'source>,
    bank: &'a Bank,
    access: &'a mut A,
    services: &'a Context<'services>,
    kingdoms: &'a KingdomRegistryState,
    inactive: Option<&'a crate::restore_lifecycle::InactiveLoad>,
    actor: ActorId,
    target: ActorId,
    explored: bool,
    visible: bool,
    container: Option<ActorId>,
    visiting: &'a mut BTreeSet<ActorId>,
}
impl<A: Access> Owned<'_, '_, '_, A> {
    fn index(&self) -> Result<Option<u32>> {
        if self.target.is_null() {
            return Err(Error::NullDereference("54EC70/54ECB0 Kingdom+190"));
        }
        validate(self.catalog, self.bank, self.access, self.target)?;
        let r = self.access.read(self.target)?;
        if r.definition.native_class != 4 {
            return Err(error("stored Actor64 target is not actual Kingdom"));
        }
        let crate::actor_world::Source::Special(s) = r.source else {
            return Err(error("stored Actor64 Kingdom derived allocation absent"));
        };
        if !matches!(s.source, crate::actor_world::SpecialSource::Kingdom(_)) {
            return Err(error("stored Actor64 derived owner differs"));
        }
        let d = s
            .derived
            .as_ref()
            .ok_or_else(|| error("stored Actor64 Kingdom fields absent"))?;
        let word = |offset| {
            d.words
                .get(&offset)
                .copied()
                .ok_or_else(|| error(format!("stored Actor64 Kingdom+{offset:X} unwritten")))
        };
        if word(0x190)? != 0 {
            return Ok(None);
        }
        Ok(Some(word(0x18c)?))
    }
    fn flag(&self, shift: u32) -> Result<bool> {
        let Some(i) = self.index()? else {
            return Ok(true);
        };
        Ok(self.access.read(self.actor)?.object.words_50_to_54[1]
            & 1u32.wrapping_shl(i.wrapping_add(shift))
            != 0)
    }
    fn put_flag(&mut self, shift: u32) -> Result<()> {
        if let Some(i) = self.index()? {
            self.access.object_mut(self.actor)?.words_50_to_54[1] |=
                1u32.wrapping_shl(i.wrapping_add(shift));
        }
        Ok(())
    }
}
impl<A: Access> crate::load_actor_reveal::World for Owned<'_, '_, '_, A> {
    fn shown(&mut self) -> Result<bool> {
        self.flag(16)
    }
    fn seen(&mut self) -> Result<bool> {
        self.flag(0)
    }
    fn show(&mut self) -> Result<()> {
        self.put_flag(16)
    }
    fn explore(&mut self) -> Result<()> {
        self.put_flag(0)
    }
    fn actor_b4(&mut self) -> Result<bool> {
        Ok(matches!(
            self.access.read(self.actor)?.definition.native_class,
            3 | 4
        ))
    }
    fn container(&mut self) -> Result<bool> {
        let r = self.access.read(self.actor)?;
        let a = r
            .actor
            .as_ref()
            .ok_or(Error::NullDereference("4CED60 stored Actor"))?;
        self.container = crate::load_detection_memory::actor_container(self.actor, a.components)?;
        Ok(self.container.is_some())
    }
    fn reveal_container(&mut self) -> Result<()> {
        let child = self
            .container
            .ok_or(Error::NullDereference("4EC010 current container"))?;
        reveal_call(
            self.catalog,
            self.bank,
            self.access,
            self.services,
            self.kingdoms,
            self.inactive,
            child,
            self.target,
            self.explored,
            self.visible,
            self.visiting,
        )
    }
    fn memory(&mut self) -> Result<()> {
        crate::load_store_fog::refresh_memory_access(
            self.catalog,
            self.bank,
            self.access,
            self.services,
            self.kingdoms,
            self.inactive,
            self.actor,
        )
    }
    fn announce(&mut self) -> Result<u8> {
        self.services.native_announce_byte_in_bank(
            self.catalog,
            self.bank,
            self.access.read(self.actor)?.definition,
        )
    }
    fn first_sighting(&mut self) -> Result<()> {
        Err(Error::Unsupported(
            "4EC010 actual first-sighting432E40 message/SAI owner",
        ))
    }
}
fn reveal_call(
    catalog: &Catalog<'_>,
    bank: &Bank,
    access: &mut impl Access,
    services: &Context<'_>,
    kingdoms: &KingdomRegistryState,
    inactive: Option<&crate::restore_lifecycle::InactiveLoad>,
    actor: ActorId,
    target: ActorId,
    explored: bool,
    visible: bool,
    visiting: &mut BTreeSet<ActorId>,
) -> Result<()> {
    validate(catalog, bank, access, actor)?;
    if !visiting.insert(actor) {
        return Err(error("stored Actor64 cyclic current container graph"));
    }
    let result = crate::load_actor_reveal::reveal_inner(
        &mut Owned {
            catalog,
            bank,
            access,
            services,
            kingdoms,
            inactive,
            actor,
            target,
            explored,
            visible,
            container: None,
            visiting,
        },
        explored,
        visible,
    );
    visiting.remove(&actor);
    result
}
#[allow(clippy::too_many_arguments)]
pub(crate) fn reveal(
    catalog: &Catalog<'_>,
    bank: &Bank,
    access: &mut impl Access,
    services: &Context<'_>,
    kingdoms: &KingdomRegistryState,
    inactive: Option<&crate::restore_lifecycle::InactiveLoad>,
    actor: ActorId,
    target: ActorId,
    explored: bool,
    visible: bool,
) -> Result<()> {
    reveal_call(
        catalog,
        bank,
        access,
        services,
        kingdoms,
        inactive,
        actor,
        target,
        explored,
        visible,
        &mut BTreeSet::new(),
    )
    .map_err(|e| {
        error(format!(
            "Actor64(actor={}, kingdom={}, explored={explored}, visible={visible}): {e}",
            actor.raw(),
            target.raw()
        ))
    })
}
