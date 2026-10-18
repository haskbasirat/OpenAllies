//! Native579EB0 member terrain propagation over the same held allocations.
//! Every non-null member slot is visited in order (attached-only=false). The
//! actual Element owner is required; no missing member or component is skipped.
use crate::{catalog::Catalog,live_object_init::Record,load_actor_terrain_refresh::{self,Effects},
    load_spatial::HeldView,battle_terrain::BattleTerrain};
use aa_actors::{ActorId,actor_creation::{Error,Result},components::ComponentSlot};
use aa_mapworld::restore::{RestoredComponent,RestoredRecord};
use aa_numeric::Precision;

#[cfg(test)]
#[path="load_organization_terrain_refresh_tests.rs"]
mod tests;

fn required(name:&'static str)->Error {Error::Unsupported(name)}
fn member(record:&Record<'_>,slot:usize)->Result<Option<ActorId>> {
    match record.shell.actor.as_ref().and_then(|a|a.components.get(ComponentSlot::Organization)) {
        Some(RestoredComponent::Organization(org)) if org.header.owner==record.owner => {
            if org.members.len()>i32::MAX as usize {return Err(required("4364C0 signed native Organization count"))}
            Ok(org.members.get(slot).copied())
        }
        _=>Err(required("579EB0 actual Organization payload/owner")),
    }
}

struct Tree<'a,'source,'parents> {
    catalog:&'a Catalog<'source>,
    terrain:&'a BattleTerrain,
    parents:&'parents [HeldView<'parents>],
}
impl Effects for Tree<'_, '_, '_> {
    fn organization_refresh(&mut self,record:&mut Record<'_>,pc:Precision)->Result<()> {
        let mut slot=0usize;
        loop {
            //4364C0 rereads count and each next slot after the preceding call.
            let Some(id)=member(record,slot)? else {return Ok(())};
            slot=slot.checked_add(1).ok_or(required("4364C0 native slot representation"))?;
            if id.is_null(){continue}
            if id==record.owner||self.parents.iter().any(|p|p.owner==id) {
                return Err(required("579EB0 cyclic held member requires native reentrant owner service"));
            }
            let entry=record.registry.value_mut(id)
                .ok_or(Error::NullDereference("579EB0 actual non-null member generation"))?;
            let mut held=std::mem::replace(entry,RestoredRecord::Loading);
            let outcome=(|| {
                let (shell,definition)=match &mut held {
                    RestoredRecord::Actor(v)=>(&mut v.shell,&v.definition),
                    RestoredRecord::Kingdom(v)=> {
                        let v=v.runtime.as_mut().ok_or(required("579EB0 member Kingdom runtime"))?;
                        (&mut v.base,&v.definition)
                    }
                    _=>return Err(required("579EB0 actual member Actor allocation")),
                };
                match shell.actor.as_ref().and_then(|a|a.components.get(ComponentSlot::Element)) {
                    Some(RestoredComponent::Element(element)) if element.header.owner==id=>(),
                    None=>return Err(Error::NullDereference("570550 member Element+4 owner")),
                    _=>return Err(required("570550 actual member Element payload/owner")),
                }
                // Explicit reborrows shorten all ancestor lifetimes to this
                // synchronous call; the actual current parent is appended.
                let mut parents=self.parents.iter().map(|p|HeldView {
                    owner:p.owner,shell:p.shell,definition:p.definition }).collect::<Vec<_>>();
                parents.push(HeldView {owner:record.owner,shell:&*record.shell,definition:record.definition});
                let mut child=Record {owner:id,shell,definition,registry:&mut *record.registry};
                let mut effects=Tree {catalog:self.catalog,terrain:self.terrain,parents:&parents};
                load_actor_terrain_refresh::refresh_with_held(self.catalog,&mut child,self.terrain,pc,&parents,&mut effects)
            })();
            *record.registry.value_mut(id).expect("held member generation retained")=held;
            outcome?;
        }
    }
}

/// Actual terrain transition plus recursive579EB0. All other positive Property
/// effects retain the original required defaults; no instance arena is forged.
pub fn refresh(catalog:&Catalog<'_>,record:&mut Record<'_>,terrain:&BattleTerrain,pc:Precision)->Result<()> {
    load_actor_terrain_refresh::refresh(catalog,record,terrain,pc,&mut Tree {catalog,terrain,parents:&[]})
}
