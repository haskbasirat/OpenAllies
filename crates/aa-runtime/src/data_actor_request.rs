//!Selected immutable ActorData request fields from464BD0, including each
//!AbilityData's actual linked AbilityDefinition five-list context4BF840.
//!This does not complete unrelated Ability/Actor LoadData or global source
//!ordering. Positive unresolved shared definitions and derived534 stay required.
use super::{Data,State,Result};
use crate::{catalog::Catalog,definitions::Bank,fx_graph::{self,Graph},
    fx_graph::actor_request_source::Group,resource_depot::{Resources,Backend},resource_objects::ModelPolicy};
use aa_mapworld::restore::RuntimeDefinition;
use aa_rules::{actors::ResolvedNode,resources::Provenance};
use std::sync::Arc;
#[path="actor_request_core.rs"]
pub(crate) mod core;

#[derive(Clone)]
pub struct Ability { data:Data, list:usize,index:usize }
struct AbilityFields { source:Vec<Provenance>,graph:Option<Arc<Graph>> }
pub(super) struct Fields {
    definition:RuntimeDefinition,pairs:[Option<Arc<Graph>>;4],abilities:[Vec<AbilityFields>;3],
    step:usize,complete:bool,preparing:bool,failed:Option<String>,partial:Option<fx_graph::Failure>,pending_ability:Option<Vec<Provenance>>,
}
#[cfg(test)]
#[path="data_actor_request_tests.rs"]
mod tests;
impl std::fmt::Debug for Fields{
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("ActorRequestFields").field("id",&self.definition.proof.id).field("step",&self.step).field("complete",&self.complete).field("failed",&self.failed).finish()}
}
impl Fields{
    fn constructed(definition:&RuntimeDefinition)->Self{Self{definition:definition.clone(),pairs:[None,None,None,None],abilities:[vec![],vec![],vec![]],step:0,complete:false,preparing:false,failed:None,partial:None,pending_ability:None}}
}
fn error(e:impl std::fmt::Display)->String{e.to_string()}
fn root<'a>(catalog:&'a Catalog<'_>,d:&RuntimeDefinition)->Result<ResolvedNode<'a>>{
    let node=catalog.actors.resolved_actor(&d.proof.id).map_err(error)?;
    if node.sources()!=d.proof.template_chain{return Err("Actor request canonical source layers differ".into())}Ok(node)
}
impl State{
    fn actor_fields(&self,data:&Data)->Result<&Fields>{self.entry(data)?.actor_requests.as_ref().ok_or_else(||"actual Actor request fields were not source-prepared".into())}
    fn actor_fields_mut(&mut self,data:&Data)->Result<&mut Fields>{self.entry_mut(data)?.actor_requests.as_mut().ok_or_else(||"actual Actor request fields were not source-prepared".into())}
    fn ready_actor_fields(&self,data:&Data)->Result<&Fields>{let f=self.actor_fields(data)?;if !f.complete{return Err(f.failed.clone().unwrap_or_else(||"Actor request source projection unfinished".into()))}Ok(f)}
    ///Run once in the actual pre-World source-resource interval. The selected
    ///ordering is root pair, Character animations, Performer, Defender, Mover,
    ///Body OnDeathAbility/health pair, then Portal. Graph/Resource failures
    ///retain preceding same-owner allocations; this is not whole LoadData.
    pub fn prepare_actor_request_fields(&mut self,catalog:&Catalog<'_>,bank:&Bank,d:&RuntimeDefinition,
        resources:&mut Resources<'_>,policy:&mut impl ModelPolicy)->Result<()>{
        self.prepare_actor_request_fields_with_textures(catalog,bank,d,resources,policy,&mut crate::resource_objects::RequiredTexturePolicy)
    }
    pub(crate) fn prepare_actor_request_fields_with_textures(&mut self,catalog:&Catalog<'_>,bank:&Bank,d:&RuntimeDefinition,
        resources:&mut Resources<'_>,policy:&mut impl ModelPolicy,texture_policy:&mut impl crate::resource_objects::TexturePolicy)->Result<()>{
        self.bind_resources(resources)?;
        let pc=super::validate_object_data(catalog,bank,d)?;
        if d.proof.category.native_kind!=2{return Err("Actor request source requires actual ActorData".into())}
        let data=self.bind_object(catalog,bank,d)?;
        if self.entry(&data)?.actor_requests.is_none(){self.entry_mut(&data)?.actor_requests=Some(Fields::constructed(d));}
        let f=self.actor_fields(&data)?;
        if f.definition.proof.id!=d.proof.id||f.definition.proof.source!=d.proof.source||f.definition.proof.template_chain!=d.proof.template_chain||f.definition.proof.category!=d.proof.category||f.definition.flags!=d.flags||f.definition.component_mask!=d.component_mask||f.definition.native_class!=d.native_class{return Err("Actor request held definition changed".into())}
        if let Some(error)=&f.failed{return Err(error.clone())}if f.complete{return Ok(())}if f.preparing{return Err("recursive Actor request source preparation".into())}
        self.actor_fields_mut(&data)?.preparing=true;
        let result=(||{
            let actor=root(catalog,d)?;
            if self.actor_fields(&data)?.step==0{
                match Graph::compile_actor_leave_request_resources(catalog,bank,d,resources,texture_policy){
                    Ok(graph)=>self.actor_fields_mut(&data)?.pairs[0]=Some(Arc::new(graph)),
                    Err(failure)=>{let e=failure.error.clone();self.actor_fields_mut(&data)?.partial=Some(failure);return Err(e)}
                }
                let graph=self.actor_fields(&data)?.pairs[0].as_ref().unwrap().clone();self.prepare_actor_request_graph(catalog,bank,&graph,resources,policy,texture_policy)?;
                self.actor_fields_mut(&data)?.step=1;
            }
            if self.actor_fields(&data)?.step==1{self.prepare_actor_animation_requests_with_textures(catalog,bank,d,resources,policy,texture_policy)?;self.actor_fields_mut(&data)?.step=2;}
            for (list,component)in ["PerformerComponent","DefenderComponent"].into_iter().enumerate(){
                if self.actor_fields(&data)?.step==list+2{
                    if let Some(scope)=actor.child(component).map_err(error)?{
                        for ability in scope.children("Ability").map_err(error)?{self.prepare_actor_ability(catalog,bank,d,&data,list,&ability,resources,policy,texture_policy)?;}
                    }
                    self.actor_fields_mut(&data)?.step+=1;
                }
            }
            for (pair,group)in [(1,Group::Mover),(2,Group::Body),(3,Group::Portal)]{
                if self.actor_fields(&data)?.step==pair+3{
                    let scope=actor.child(group.component().unwrap()).map_err(error)?;
                    if group==Group::Body && let Some(body)=&scope && let Some(ability)=body.child("OnDeathAbility").map_err(error)?{
                        self.prepare_actor_ability(catalog,bank,d,&data,2,&ability,resources,policy,texture_policy)?;
                    }
                    //An absent component retains the actual4624B0 zero heads;
                    //the canonical single-declaration guard is required above.
                    if let Some(scope)=scope{
                        match fx_graph::actor_request_source::compile_resources(catalog,bank,d,&scope,group,resources,texture_policy){
                            Ok(graph)=>self.actor_fields_mut(&data)?.pairs[pair]=Some(Arc::new(graph)),
                            Err(failure)=>{let e=failure.error.clone();self.actor_fields_mut(&data)?.partial=Some(failure);return Err(e)}
                        }
                        let graph=self.actor_fields(&data)?.pairs[pair].as_ref().unwrap().clone();self.prepare_actor_request_graph(catalog,bank,&graph,resources,policy,texture_policy)?;
                    }
                    self.actor_fields_mut(&data)?.step+=1;
                }
            }
            //The exact source precision is checked by every graph/compiler;
            //no live-frame precision is substituted for it.
            let _=pc;self.actor_fields_mut(&data)?.complete=true;Ok(())
        })();
        self.actor_fields_mut(&data)?.preparing=false;
        if let Err(error)=&result{self.actor_fields_mut(&data)?.failed=Some(error.clone());}result
    }
    fn prepare_actor_request_graph(&mut self,catalog:&Catalog<'_>,bank:&Bank,graph:&Graph,resources:&mut Resources<'_>,policy:&mut impl ModelPolicy,
        texture_policy:&mut impl crate::resource_objects::TexturePolicy)->Result<()>{
        for i in 0..graph.nodes().len(){self.prepare_graph_node_with_textures(catalog,bank,graph,graph.node_id(i)?,resources,policy,texture_policy)?;}Ok(())
    }
    fn prepare_actor_ability(&mut self,catalog:&Catalog<'_>,bank:&Bank,d:&RuntimeDefinition,data:&Data,list:usize,
        ability:&ResolvedNode<'_>,resources:&mut Resources<'_>,policy:&mut impl ModelPolicy,texture_policy:&mut impl crate::resource_objects::TexturePolicy)->Result<()>{
        self.actor_fields_mut(data)?.pending_ability=Some(ability.sources());
        //4658A*: Defender appends its actual allocation before4BF990;
        //Performer appends only after the loader/duplicate checks.2A4 is
        //also stored before its load. Failed prefixes keep those identities.
        if list!=0{self.actor_fields_mut(data)?.abilities[list].push(AbilityFields{source:ability.sources(),graph:None});}
        let Some(definition)=ability.child("AbilityDefinition").map_err(error)? else{
            return Err("4BF990 shared ability_IDS requires actual source-order shared AbilityDefinition allocation owner".into())
        };
        let graph=match fx_graph::actor_request_source::compile_resources(catalog,bank,d,&definition,Group::Ability,resources,texture_policy){
            Ok(graph)=>Arc::new(graph),Err(failure)=>{let e=failure.error.clone();self.actor_fields_mut(data)?.partial=Some(failure);return Err(e)}
        };
        //4BE420 only reads the linked definition's five FX heads. This
        //retains that exact local source identity, not a fake whole4BF990.
        if list==0{self.actor_fields_mut(data)?.abilities[list].push(AbilityFields{source:ability.sources(),graph:Some(graph.clone())});}
        else{self.actor_fields_mut(data)?.abilities[list].last_mut().unwrap().graph=Some(graph.clone());}
        self.actor_fields_mut(data)?.pending_ability=None;
        self.prepare_actor_request_graph(catalog,bank,&graph,resources,policy,texture_policy)
    }
    pub(crate) fn actor_fx_request(&mut self,catalog:&Catalog<'_>,bank:&Bank,data:&Data,offset:u32,
        resources:&mut Resources<'_>,backend:&mut impl Backend,delta:u32,mode:u32)->Result<()>{
        let pair=core::PAIR_OFFSETS.iter().position(|&v|v==offset).ok_or("Actor request offset is not one of8 actual FX pairs")?/2;
        let graph=self.ready_actor_fields(data)?.pairs[pair].clone();
        if let Some(graph)=graph{self.request_fx_list(catalog,bank,&graph,graph.actor_pair_roots(offset)?,resources,backend,delta,mode)?;}Ok(())
    }
    pub(crate) fn actor_ability_count(&self,data:&Data,secondary:bool)->Result<isize>{
        isize::try_from(self.ready_actor_fields(data)?.abilities[usize::from(secondary)].len()).map_err(|_|"Actor Ability count outside signed native domain".into())
    }
    pub(crate) fn actor_ability(&self,data:&Data,secondary:bool,index:usize)->Result<Ability>{self.ability_handle(data,usize::from(secondary),index)}
    fn ability_handle(&self,data:&Data,list:usize,index:usize)->Result<Ability>{
        let a=self.ready_actor_fields(data)?.abilities.get(list).and_then(|v|v.get(index)).ok_or("Actor AbilityData index is outside source allocation")?;
        if a.source.is_empty(){return Err("Actor AbilityData source identity absent".into())}Ok(Ability{data:data.clone(),list,index})
    }
    pub(crate) fn actor_extra_ability_2a4(&self,data:&Data)->Result<Option<Ability>>{
        if self.ready_actor_fields(data)?.abilities[2].is_empty(){Ok(None)}else{self.ability_handle(data,2,0).map(Some)}
    }
    pub(crate) fn request_actor_ability(&mut self,catalog:&Catalog<'_>,bank:&Bank,ability:Ability,
        resources:&mut Resources<'_>,backend:&mut impl Backend,delta:u32,mode:u32)->Result<()>{
        let graph=self.ready_actor_fields(&ability.data)?.abilities.get(ability.list).and_then(|v|v.get(ability.index)).and_then(|a|a.graph.as_ref()).ok_or("Ability handle outside completed same Data allocation")?.clone();
        graph.require_in_bank(catalog,bank)?;
        core::request_ability(delta,mode,|offset,delta,mode|self.request_fx_list(catalog,bank,&graph,graph.actor_request_roots(offset)?,resources,backend,delta,mode))
    }
    pub(crate) fn actor_settlement_534(&self,data:&Data)->Result<Option<RuntimeDefinition>>{
        let f=self.ready_actor_fields(data)?;
        if f.definition.component_mask.ok_or("Actor mask174 absent")?&(1<<19)!=0{return Err("45DD80 positive ActorData534 requires actual derived SettlementData source writer".into())}Ok(None)
    }
}
