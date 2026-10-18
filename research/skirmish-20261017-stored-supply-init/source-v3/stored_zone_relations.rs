//! Same borrowed4CF040/4E52D0/4E10F0 kernels used by the saved zone adapter.
//! Kingdom and Team reads borrow their actual derived fields in ActorStore.
use super::*;

impl<E: Effects> View<'_, '_, '_, E> {
    fn relation_special(
        &self,
        id: ActorId,
        class: u32,
    ) -> Result<&aa_mapworld::special_state::SpecialShell> {
        if !self.fields.present(id) {
            return Err(error("stored zone relation generation absent"));
        }
        let definition = self.fields.definition(id).map_err(error)?;
        let (kind, object, actor) = self.fields.kind(id).map_err(error)?;
        if definition.native_class != class
            || definition.proof.category.runtime_class != class
            || !self.fields.same_definition_proof(id).map_err(error)?
            || (class == 4 && !crate::load_spatial::ordinary_layout(class, kind, object, actor))
            || (class == 5 && (kind != aa_actors::loading::LoadKind::Thing || object || actor))
        {
            return Err(error(
                "stored zone relation actual source/class layout differs",
            ));
        }
        if class == 4 {
            self.checked(id.raw())?;
        }
        let Source::Special(special) = self.fields.source(id).map_err(error)? else {
            return Err(error("stored zone relation actual derived owner absent"));
        };
        if !matches!(
            (&special.source, class),
            (crate::actor_world::SpecialSource::Kingdom(_), 4)
                | (crate::actor_world::SpecialSource::Team(_), 5)
        ) {
            return Err(error("stored zone relation actual source kind differs"));
        }
        special
            .derived
            .as_ref()
            .ok_or_else(|| error("stored zone relation actual derived fields absent"))
    }

    pub(super) fn related_code(&self, actor: u32, other: u32) -> Result<u32> {
        let id = self.checked(actor)?;
        let other = (other != 0).then_some(ActorId::from_raw(other));
        match self.fields.definition(id).map_err(error)?.native_class {
            3 => {
                let owner = self.word(actor, 0xe8)?;
                crate::relations::borrowed::actor(
                    self,
                    (owner != 0).then_some(ActorId::from_raw(owner)),
                    other,
                )
            }
            4 => crate::relations::borrowed::kingdom(self, id, other),
            _ => required("stored zone Actor144 class dispatch"),
        }
    }
}

impl<E: Effects> crate::relations::borrowed::Read for View<'_, '_, '_, E> {
    type Id = ActorId;
    type Error = Error;

    fn kingdom_team(&self, id: ActorId) -> Result<Option<ActorId>> {
        self.relation_special(id, 4)?
            .identity_references
            .get(&0x1ec)
            .copied()
            .ok_or_else(|| error("stored zone relation actual Kingdom1EC owner absent"))
    }
    fn kingdom_slot(&self, id: ActorId) -> Result<u32> {
        self.relation_special(id, 4)?
            .words
            .get(&0x188)
            .copied()
            .ok_or_else(|| error("stored zone relation actual Kingdom188 absent"))
    }
    fn team_slot(&self, id: ActorId) -> Result<u32> {
        self.relation_special(id, 5)?
            .words
            .get(&0x38)
            .copied()
            .ok_or_else(|| error("stored zone relation actual Team38 absent"))
    }
    fn kingdom_word(&self, id: ActorId, team: bool, slot: u32) -> Result<u32> {
        self.relation_special(id, 4)?
            .vectors
            .get(&if team { 0x1dc } else { 0x1d0 })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| error("stored zone relation actual Kingdom vector slot absent"))
    }
    fn team_word(&self, id: ActorId, team: bool, slot: u32) -> Result<u32> {
        self.relation_special(id, 5)?
            .vectors
            .get(&if team { 0x58 } else { 0x4c })
            .and_then(|v| v.values.get(slot as usize))
            .copied()
            .ok_or_else(|| error("stored zone relation actual Team vector slot absent"))
    }
    fn orphan_relation(&self) -> Result<u32> {
        required("stored zone current mutable globalB28158 owner")
    }
    fn team_null_relation(&self) -> Result<u32> {
        required("stored zone current mutable globalB2815C owner")
    }
}
