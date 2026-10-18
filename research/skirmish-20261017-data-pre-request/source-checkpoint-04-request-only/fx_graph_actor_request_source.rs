//!The existing42BF10 compiler over the exact request-list contexts in
//!464BD0 and4BF840. This is a selected source projection, not completion
//!of unrelated Actor/AbilityDefinition fields or their LoadData calls.
use super::*;
use crate::definitions::Bank;

pub(crate) use crate::data_pre_request::actor_request::core::Group;
use crate::data_pre_request::actor_request::core::{self,Compile};
///Only canonical source selection code in the sibling Data owner calls
///this function. `node` must be a real selected component or the actual
///linked LOCAL AbilityDefinition. The source is reselected from this exact
///canonical Actor; shared definitions require their distinct registry owner.
pub(crate) fn compile(catalog:&Catalog<'_>,bank:&Bank,definition:&RuntimeDefinition,
    node:&ResolvedNode<'_>,group:Group)->std::result::Result<Graph,Failure>{
    let mut draft=Draft::new();
    let result:Result<Graph>=(||{
        let source=ObjectFxSource::bind_in_bank(catalog,bank,definition)?;
        if definition.proof.category.native_kind!=2{return Err("Actor request FX requires actual ActorData".into())}
        let actor=catalog.actors.resolved_actor(&definition.proof.id).map_err(|e|e.to_string())?;
        let actual=if let Some(component)=group.component(){actor.child(component).map_err(|e|e.to_string())?.is_some_and(|actual|actual.sources()==node.sources())}
        else{
            let mut found=false;
            for component in ["PerformerComponent","DefenderComponent","BodyComponent"]{
                if let Some(scope)=actor.child(component).map_err(|e|e.to_string())?{
                    let abilities=if component=="BodyComponent"{scope.child("OnDeathAbility").map_err(|e|e.to_string())?.into_iter().collect()}else{scope.children("Ability").map_err(|e|e.to_string())?};
                    for ability in abilities{if ability.child("AbilityDefinition").map_err(|e|e.to_string())?.is_some_and(|d|d.sources()==node.sources()){found=true;}}
                }
            }found
        };
        if !actual{return Err("Actor request FX node is outside the actual selected source allocation".into())}
        let mut input=Bound::new(catalog.actors,source.source_precision(),definition.proof.source.clone())?;
        input.request_only=true;
        let mut lists=Vec::new();
        core::compile_group(group,|operation|match operation{
            Compile::Load(name)=>{
                let roots=node.children(name).map_err(|e|e.to_string())?;
                append_roots(&mut input,roots,&mut draft)?;
                lists.push(std::mem::take(&mut draft.roots));Ok(())
            },
            Compile::Synthesize{source,destination}=>{
                //Body uses the SAME list for source and destination. Capture
                //its original nodes before42B8E0 prepends new StopVFX nodes.
                let roots=lists[source].clone();draft.roots=std::mem::take(&mut lists[destination]);
                actor_state::synthesize_stops(&input,&roots,&mut draft)?;
                lists[destination]=std::mem::take(&mut draft.roots);Ok(())
            },
            Compile::ValidateHealth=>{
                for id in &lists[1]{core::validate_health(draft.nodes[id.0].word(0x30).ok_or("FXC health trigger absent")?)?;}Ok(())
            },
        })?;
        draft.roots=lists.pop().unwrap();
        Ok(Graph{draft:std::mem::replace(&mut draft,Draft::new()),prior_roots:lists,
            owner:definition.proof.id.clone(),source:definition.proof.source.clone(),
            precision:source.source_precision(),sources:catalog.actors.sources.clone(),kind:GraphKind::ActorRequest(group.base()),request_only:true})
    })();
    result.map_err(|error|Failure{error,partial:draft})
}
impl Graph{
    pub(crate) fn actor_request_roots(&self,offset:u32)->Result<&[NodeId]>{
        let base=match self.kind{GraphKind::ActorRequest(base)=>base,_=>return Err("graph is not an Actor request context".into())};
        let index=offset.checked_sub(base).filter(|v|v%8==0).ok_or("Actor request list offset differs")?/8;
        if index as usize==self.prior_roots.len(){Ok(&self.draft.roots)}else{
            self.prior_roots.get(index as usize).map(Vec::as_slice).ok_or_else(||"Actor request list was not loaded".into())
        }
    }
    pub(crate) fn actor_pair_roots(&self,offset:u32)->Result<&[NodeId]>{
        match self.kind{
            GraphKind::ActorLeave=>match offset{0x194=>self.prior_roots.first().map(Vec::as_slice).ok_or("Actor enter head absent".into()),0x19c=>Ok(self.roots()),_=>Err("Actor root list offset differs".into())},
            GraphKind::ActorRequest(_)=>self.actor_request_roots(offset),
            _=>Err("Actor pair requires its actual source context".into()),
        }
    }
}
