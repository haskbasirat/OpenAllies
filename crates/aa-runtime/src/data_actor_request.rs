//!Selected immutable ActorData request fields from464BD0, including each
//!AbilityData's actual linked AbilityDefinition five-list context4BF840.
//!This does not complete unrelated Ability/Actor LoadData or global source
//!ordering. Positive unresolved shared definitions stay required. +534 is the
//!MobileDeployment forward unpack target (464BD0 block,4605B0), see deployment.
use super::{Data,State,Result};
use crate::{catalog::Catalog,definitions::Bank,fx_graph::{self,Graph},
    fx_graph::actor_request_source::Group,resource_depot::{Resources,Backend},resource_objects::ModelPolicy};
use aa_mapworld::restore::RuntimeDefinition;
use aa_rules::{actors::ResolvedNode,resources::Provenance};
use std::sync::Arc;
#[path="actor_request_core.rs"]
pub(crate) mod core;
#[path="actor_deployment_534.rs"]
pub(crate) mod deployment;

#[derive(Clone)]
pub struct Ability { data:Data, list:usize,index:usize }
struct AbilityFields { source:Vec<Provenance>,graph:Option<Arc<Graph>> }
pub(super) struct Fields {
    definition:RuntimeDefinition,pairs:[Option<Arc<Graph>>;4],abilities:[Vec<AbilityFields>;3],
    step:usize,complete:bool,preparing:bool,failed:Option<String>,partial:Option<fx_graph::Failure>,pending_ability:Option<Vec<Provenance>>,
    ///The464BD0 MobileDeployment block of THIS actor (None: no component).
    mobile_block:Option<deployment::Block>,
    ///+534: None until the post-LoadData475230/4605B0 pass ran; then the
    ///forward target or the4624B0 null.
    deployment_534:Option<Option<RuntimeDefinition>>,
}
#[cfg(test)]
#[path="data_actor_request_tests.rs"]
mod tests;
impl std::fmt::Debug for Fields{
    fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("ActorRequestFields").field("id",&self.definition.proof.id).field("step",&self.step).field("complete",&self.complete).field("failed",&self.failed).finish()}
}
impl Fields{
    fn constructed(definition:&RuntimeDefinition)->Self{Self{definition:definition.clone(),pairs:[None,None,None,None],abilities:[vec![],vec![],vec![]],step:0,complete:false,preparing:false,failed:None,partial:None,pending_ability:None,mobile_block:None,deployment_534:None}}
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
    ///Body OnDeathAbility/health pair, Portal, then the MobileDeployment+534
    ///source writer (no target is loaded here). Graph/Resource failures
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
                    //464BD0 TrackInfo (465DA4) precedes the Mover FX lists.
                    if group==Group::Mover{self.prepare_actor_track_infos(catalog,bank,d,&data,&actor,pc,resources,texture_policy)?;}
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
            //464BD0 46852F..4686D9 on THIS actor's own component (468538 follows
            //Portal 46832A natively too). Its410C40 fails this definition and so
            //the whole source preparation. The475230/4605B0 deferred pass runs
            //only after every LoadData: complete_deferred_links_475230.
            if self.actor_fields(&data)?.step==7{
                let mask=d.component_mask.ok_or("Actor mask174 absent")?;
                let block=match actor.child(deployment::COMPONENT).map_err(error)?{
                    Some(scope)=>{
                        let block=deployment::read_block(&mut deployment::Node{node:&scope,pc})?;
                        deployment::validate_block(&block,&d.proof.id)?;Some(block)
                    }
                    None=>None,
                };
                if (mask&deployment::MOBILE_DEPLOYMENT_BIT!=0)!=block.is_some(){return Err("ActorData+174 bit19 and MobileDeploymentComponent source differ".into())}
                let f=self.actor_fields_mut(&data)?;f.mobile_block=block;f.step=8;
            }
            //The exact source precision is checked by every graph/compiler;
            //no live-frame precision is substituted for it.
            self.actor_fields_mut(&data)?.complete=true;Ok(())
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
    ///The completed Graph holding one of the eight Actor FX pair lists, for
    ///45DF40/45EF20's 428900/428950 (same roots45DD80's 4288B0 requests).
    pub(crate) fn actor_fx_graph(&self,data:&Data,offset:u32)->Result<Option<Arc<Graph>>>{
        let pair=core::PAIR_OFFSETS.iter().position(|&v|v==offset).ok_or("Actor FX offset is not one of8 actual FX pairs")?/2;
        Ok(self.ready_actor_fields(data)?.pairs[pair].clone())
    }
    ///The SAME linked AbilityDefinition five-list Graph4BE420 requests, for4BE450/4BE480.
    pub(crate) fn actor_ability_graph(&self,catalog:&Catalog<'_>,bank:&Bank,ability:&Ability)->Result<Arc<Graph>>{
        let graph=self.ready_actor_fields(&ability.data)?.abilities.get(ability.list).and_then(|v|v.get(ability.index)).and_then(|a|a.graph.as_ref()).ok_or("Ability handle outside completed same Data allocation")?.clone();
        graph.require_in_bank(catalog,bank)?;Ok(graph)
    }
    ///45DF17: the completed+534 of THIS Data, read only after45DF0B saw bit19.
    ///Null for the pack side (packable buildings) and every non-mobile actor.
    pub(crate) fn actor_deployment_534(&self,data:&Data)->Result<Option<RuntimeDefinition>>{
        let f=self.ready_actor_fields(data)?;
        if f.step!=8{return Err("ActorData+534 MobileDeployment source writer unfinished".into())}
        f.deployment_534.clone().ok_or_else(||"ActorData+534 requires the post-LoadData475230/4605B0 deferred pass".into())
    }
    ///475230 after the data files load: the whole deferred reference pass
    ///(aa_rules::native_mobile_deployment, Unicorn-proven), then +534 of every
    ///prepared ActorData. A deferred410C40 therefore follows every LoadData
    ///error, as natively. Repeating it recomputes the same cached links.
    pub fn complete_deferred_links_475230(&mut self,catalog:&Catalog<'_>,bank:&Bank)->Result<()>{
        let pc=bank.combat_source()?.source_precision();
        let mobile=catalog.native_mobile_deployment_data(pc)?;
        let tokens:Vec<u64>=self.entries.iter().filter(|(_,e)|e.actor_requests.as_ref().is_some_and(|f|f.complete)).map(|(t,_)|*t).collect();
        for token in tokens{
            let f=self.entries.get(&token).and_then(|e|e.actor_requests.as_ref()).ok_or("prepared ActorData request fields disappeared")?;
            let mask=f.definition.component_mask.ok_or("Actor mask174 absent")?;
            let id=f.definition.proof.id.clone();
            let present=mask&deployment::MOBILE_DEPLOYMENT_BIT!=0;
            let target=match deployment::forward_534(mask,&id,f.mobile_block.as_ref(),present.then_some(&*mobile))?{
                Some(target_id)=>{
                    let target=catalog.resolve_registered(&target_id,pc)?;
                    if target.proof.category.native_kind!=2||!target.proof.id.eq_ignore_ascii_case(&target_id){
                        return Err("4605B0 ActorData+534 target is not its registered ActorData allocation".into())
                    }
                    Some(target)
                }
                None=>None,
            };
            self.entries.get_mut(&token).and_then(|e|e.actor_requests.as_mut()).ok_or("prepared ActorData request fields disappeared")?.deployment_534=Some(target);
        }
        Ok(())
    }
}
