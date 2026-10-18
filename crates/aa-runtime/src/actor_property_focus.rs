//! Disjoint borrowed Property state and entity arena. Neither owner is moved,
//! replaced or reconstructed while a Property callback reads current actors.
use super::*;
pub(crate) struct PropertyActorAccess<'a> {
    identities:&'a IdentityRegistry<ObjectKey>,
    entities:&'a mut BTreeMap<ObjectKey,Box<Entity>>,
    pub minds:&'a mut aa_mind::Arena,
    pub abilities:&'a mut BTreeMap<u32,aa_combat::ability::Ability>,
    pub animations:&'a AnimationBank,
}
///Borrowed index only; no canonical allocation leaves its arena. This avoids
///aliasing the focused Properties box with callback reads of other actors.
pub(crate) struct Focus<'a>{
    pub id:ActorId,pub entity:&'a mut Entity,
    pub identities:&'a IdentityRegistry<ObjectKey>,pub others:BTreeMap<ObjectKey,&'a Entity>,
    pub abilities:&'a mut BTreeMap<u32,aa_combat::ability::Ability>,
}
impl ActorStore {
    pub(crate) fn split_property_state(&mut self)->(&mut crate::load_property_instances::State,PropertyActorAccess<'_>) {
        (&mut self.property_allocations,PropertyActorAccess{
            identities:&self.identities,entities:&mut self.entities,minds:&mut self.minds,
            abilities:&mut self.abilities,animations:&self.animations,
        })
    }
}
impl PropertyActorAccess<'_> {
    pub(crate) fn focus(&mut self,key:ObjectKey)->Result<Focus<'_>>{
        let id=self.id(key).ok_or(Error::Missing("Property focus current GTID"))?;
        let mut current=None;let mut others=BTreeMap::new();
        for (&token,entity) in self.entities.iter_mut(){
            if token==key{current=Some(entity.as_mut());}else{others.insert(token,&**entity);}
        }
        Ok(Focus{id,entity:current.ok_or(Error::Missing("Property focus current allocation"))?,
            identities:self.identities,others,abilities:&mut *self.abilities})
    }
    pub fn key(&self,id:ActorId)->Option<ObjectKey>{self.identities.get(id).map(|v|v.value)}
    pub fn id(&self,key:ObjectKey)->Option<ActorId>{self.identities.iter().find(|v|v.value==key).map(|v|v.id)}
    pub fn entity(&self,key:ObjectKey)->Result<&Entity>{self.entities.get(&key).map(Box::as_ref).ok_or(Error::Missing("Property callback current entity"))}
    pub fn entity_mut(&mut self,key:ObjectKey)->Result<&mut Entity>{self.entities.get_mut(&key).map(Box::as_mut).ok_or(Error::Missing("Property callback mutable current entity"))}
    pub fn object(&self,key:ObjectKey)->Result<&RuntimeObject>{self.entity(key)?.runtime.as_ref().ok_or(Error::Missing("Property callback runtime allocation"))}
    pub fn object_mut(&mut self,key:ObjectKey)->Result<&mut RuntimeObject>{self.entity_mut(key)?.runtime.as_mut().ok_or(Error::Missing("Property callback mutable runtime allocation"))}
}

impl ActorAccess<'_> {
    ///Same disjoint fields when the caller already lends the real Mind arena.
    ///No component, Entity, allocation directory or instance is moved out.
    pub(crate) fn property_focus(&mut self,key:ObjectKey)->Result<(&mut crate::load_property_instances::State,Focus<'_>)>{
        let id=self.id(key).ok_or(Error::Missing("Property focus current GTID"))?;
        let mut current=None;let mut others=BTreeMap::new();
        for (&token,entity) in self.entities.iter_mut(){
            if token==key{current=Some(entity.as_mut());}else{others.insert(token,&**entity);}
        }
        Ok((&mut *self.property_allocations,Focus{id,entity:current.ok_or(Error::Missing("Property focus current allocation"))?,
            identities:self.identities,others,abilities:&mut *self.abilities}))
    }
}
