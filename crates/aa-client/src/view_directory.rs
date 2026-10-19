//!Same-simulation-thread view allocations and render-only upload transport.
//!A GPU upload is a backend child, never43C420 Init or74FF50 completion.
use super::{ActorVisual,LiveScene,battle::Presentation,spawn::{self,ModelBinding,SpawnActor,SpawnReport}};
use crate::{gpu::{RendererWitness,SceneRenderer},inspection::SceneObjects,loading::LoadSettings};
use aa_assets::AssetStore;
use aa_actors::ActorId;
use aa_mapworld::{SourceIdentity,bootstrap::DefinitionProof,restore::RuntimeDefinition};
use aa_runtime::{actor_world::{ActorStore,ActorStoreWitness,ObjectKey},catalog::Catalog,definitions::Bank,
    restoration::Staged,spatial_world::{SpatialWorld,SpatialWorldWitness},world_revision::{ConstructedWorldRevision,Witness}};
use aa_runtime::live_world_frame::{World,ViewBindingParts};
use anyhow::{Context,Result,ensure,bail};
use std::{collections::{BTreeMap,VecDeque},sync::Arc};
use super::view_object_fields::{self,Fields};
#[path="view_object_binding.rs"]
mod object_binding;
pub use object_binding::InitContext;
#[path="view_order_leaves.rs"]
mod order_leaves;
pub use order_leaves::{DirectoryViews,OrderContext};

#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub struct ViewHandle(usize);
impl ViewHandle {pub fn raw(self)->usize{self.0}}
///Source-only factory requirement. No constructor, GPU, or Init is certified.
pub struct ViewSource {proof:DefinitionProof,view_type:u32,source_pc:aa_numeric::Precision}
impl ViewSource {
    pub fn bind(catalog:&Catalog<'_>,bank:&Bank,d:&RuntimeDefinition)->Result<Self> {
        catalog.require_source_owner().map_err(anyhow::Error::msg)?;
        let owner=bank.combat_source().map_err(anyhow::Error::msg)?;
        ensure!(aa_runtime::catalog_lookups::same_sources(catalog,owner.source_identities()),"view source Bank belongs to another Catalog");
        let canonical=catalog.actors.find(&d.proof.id).context("view source canonical Thing absent")?;
        ensure!(canonical.id.eq_ignore_ascii_case(&d.proof.id)&&canonical.source==d.proof.source
            &&canonical.template_chain==d.proof.template_chain&&catalog.actors.native_category(&d.proof.id)?==d.proof.category
            &&d.proof.category.runtime_class==d.native_class,"view source definition certificate differs");
        let pc=owner.source_precision();
        let (_,flag_inputs)=catalog.actors.native_base_flags(&d.proof.id,d.proof.category.native_kind==2,pc)?;
        ensure!(d.proof.base_flag_inputs==flag_inputs,"view source flag-input certificate differs");
        ensure!(d.flags==catalog.actors.native_final_flags(&d.proof.id,pc)?,"view source final flag word differs");
        let mask=if d.proof.category.native_kind==2 {
            let actual=d.component_mask.context("view getter ActorData174 is unbound")?;
            ensure!(actual==catalog.canonical_component_mask(&d.proof.id,pc).map_err(anyhow::Error::msg)?,"view source-PC component mask differs");
            Some(actual)
        }else{None};
        let view_type=select(d.proof.category.native_kind,d.proof.category.view_type,mask)?;
        Ok(Self {proof:d.proof.clone(),view_type,source_pc:pc})
    }
    pub fn view_type(&self)->u32{self.view_type}
}
fn select(kind:u32,descriptor:u32,mask:Option<u32>)->Result<u32> {
    let value=match kind {
        0|1|20|21=>descriptor,
        2=>if mask.context("45DAF0 ActorData174 absent")?&4!=0 {4}else{descriptor},
        _=>bail!("actual source view getter for Data kind{kind} is required"),
    };
    Ok(value)
}
fn same_source(a:&ViewSource,b:&ViewSource)->bool{
    a.view_type==b.view_type&&a.source_pc==b.source_pc&&a.proof.id==b.proof.id&&a.proof.source==b.proof.source
        &&a.proof.template_chain==b.proof.template_chain&&a.proof.category==b.proof.category
        &&a.proof.base_flag_inputs==b.proof.base_flag_inputs
}
///Error-only source context from the actual retained definition and view.
///Formatting this does not resolve or prepare another Data/Aspect allocation.
fn view_failure(caller:&str,proof:&DefinitionProof,index:Option<u32>,actor:Option<ActorId>,key:ObjectKey,
    handle:ViewHandle,error:anyhow::Error)->anyhow::Error{
    let message=format!("{error:#}; caller={caller}; DataID={:?}; AspectIndex={index:?}; GTID={:?}; ObjectKey={}; ViewHandle={}; source={:?}; template_chain={:?}",
        proof.id,actor.map(ActorId::raw),key.raw(),handle.raw(),proof.source,proof.template_chain);
    error.context(message)
}
struct Ticket {handle:ViewHandle,sequence:u64,map:SourceIdentity}
//Minted only by the future concrete43C420/43D160 publication path. Base
//constructor or backend/common-base Init success cannot create this marker.
struct Published;
///Only the concrete derived Init implementation may complete this state.
#[derive(Clone,Copy,PartialEq,Eq)]
enum Init {NotStarted,Running,Complete,ReturnedFalse}
struct TypedVObject {
    actor:ActorId,key:ObjectKey,source:ViewSource,ticket:Arc<Ticket>,
    backend:Option<RendererWitness>,
    ///Actual native constructor/Init flags remain unbound until that port runs.
    native_fields:Option<Fields>,
    publication:Option<Published>,
    residency:Option<aa_runtime::resource_depot::ViewResourceLease>,
    constructor_complete:bool,init:Init,updated:bool,
    model:Option<aa_runtime::resource_objects::Handle>,
    render_payload:Option<aa_runtime::resource_model_payload::Payload>,
    actor_fields:Option<super::view_actor_fields::Fields>,
    organization_fields:Option<super::view_organization_fields::Fields>,
    variant:Option<i32>,stance:Option<u32>,construction:Option<bool>,
}
///Lives beside the actual client VWorld owner on SimulationThread. Mutable
///ActorStore, Catalog, source Bank and spatial state never enter a UI packet.
pub struct ViewDirectory {
    actors:ActorStoreWitness,spatial:SpatialWorldWitness,revision:Witness,map:SourceIdentity,
    next:usize,sequence:u64,entries:BTreeMap<ViewHandle,TypedVObject>,
    ///Sole actual VWorld208/20C request-list owner; duplicates retain FIFO order.
    requests:VecDeque<ViewHandle>,
    ///Sole native VWorld9C/A0 updated-view list, in head-first order.
    updates:VecDeque<ViewHandle>,
    ///58CB30 identities: one per view+10 Data allocation (canonical id and
    ///source), append-only so an identity is never reused.
    data_identities:Vec<(String,aa_rules::resources::Provenance)>,
}
pub struct UploadRequest {ticket:Arc<Ticket>,request:SpawnActor,binding:ModelBinding,
    residency:aa_runtime::resource_depot::ViewResourceWitness,selected:SelectedModel}
struct SelectedModel{handle:aa_runtime::resource_objects::Handle,input_sha256:String,aspect:u32,
    payload:aa_runtime::resource_model_payload::Payload,position:[u32;3],facing:[u32;3],scale:u32}
pub struct UploadReceipt {ticket:Arc<Ticket>,renderer:RendererWitness,report:SpawnReport,selected:SelectedModel}
///Read-only field loans. Moving Staged into the active World preserves each
///witness; no Staged reconstruction or ActorStore/Spatial snapshot is needed.
struct Parts<'a>{actors:&'a ActorStore,spatial:&'a SpatialWorld,definitions:&'a Bank,
    revision:&'a ConstructedWorldRevision,map:&'a SourceIdentity}
impl<'a> Parts<'a>{
    fn staged(staged:&'a Staged)->Result<Self>{let world=staged.load_world.as_ref().context("view actual World absent")?;
        Ok(Self{actors:&staged.actors,spatial:world.live_spatial().map_err(anyhow::Error::msg)?,definitions:&staged.definitions,
            revision:world.revision(),map:&staged.inventory.map_source})}
    fn world(world:&'a World<'_>)->Result<Self>{let p=world.presentation_parts().map_err(anyhow::Error::msg)?;
        Ok(Self{actors:p.actors,spatial:p.spatial,definitions:p.definitions,revision:p.revision,map:&p.inventory.map_source})}
    fn binding(p:&'a ViewBindingParts<'_>)->Self{
        Self{actors:p.actors,spatial:p.spatial,definitions:p.definitions,revision:p.revision,map:&p.inventory.map_source}
    }
}
struct Constructor<'a,'assets>{catalog:&'a Catalog<'assets>,bank:&'a Bank,
    seeds:&'a aa_runtime::view_grid_startup::Seeds,orientation:&'a aa_runtime::view_renderer_variables::State,
    elapsed:&'a dyn Fn()->u32}
impl view_object_fields::ConstructorSources for Constructor<'_,'_>{
    fn global_word(&mut self,address:u32)->Result<u32>{
        let mut globals=self.seeds.bind(self.catalog).map_err(anyhow::Error::msg)?;
        aa_runtime::view_grid_state::Globals::word(&mut globals,address).map_err(anyhow::Error::msg)
    }
    fn renderer_default_orientation(&mut self)->Result<[u32;3]>{self.orientation.current(self.catalog,self.bank).map_err(anyhow::Error::msg)}
    fn world_elapsed_e0(&mut self)->Result<u32>{Ok((self.elapsed)())}
}
struct ActorConstructor<'a,'assets>{catalog:&'a Catalog<'assets>,bank:&'a Bank,definition:&'a RuntimeDefinition,
    seeds:&'a aa_runtime::view_grid_startup::Seeds,globals:&'a super::view_actor_globals::State,
    application:&'a mut aa_runtime::resource_depot::Application,source:Option<aa_runtime::data_pre_request::actor_view_source::ActorView>}
impl super::view_actor_fields::Sources for ActorConstructor<'_,'_>{
    fn global_word(&mut self,address:u32)->Result<u32>{
        if matches!(address,0xb322a4|0xb322a8|0xb322ac){return self.globals.word(address)}
        let mut globals=self.seeds.bind(self.catalog).map_err(anyhow::Error::msg)?;
        aa_runtime::view_grid_state::Globals::word(&mut globals,address).map_err(anyhow::Error::msg)
    }
    fn actor_data_38(&mut self)->Result<bool>{
        self.application.with_data_pre_request(|data|{
            let source=data.actor_view_source(self.catalog,self.bank,self.definition)?;
            let actor=data.actor_view_is_actor_38(&source)?;self.source=Some(source);Ok(actor)
        }).map_err(anyhow::Error::msg)
    }
    fn turret_count_1fc(&mut self)->Result<i32>{
        let source=self.source.as_ref().context("actual ActorData38 source getter has not run")?;
        self.application.with_data_pre_request(|data|data.actor_view_turret_count_1fc(source)).map_err(anyhow::Error::msg)
    }
    fn turret_pitch_57dbe0(&mut self,index:usize)->Result<u32>{
        let source=self.source.as_ref().context("actual ActorData38 source getter has not run")?;
        self.application.with_data_pre_request(|data|data.actor_view_turret_pitch_14(source,index)).map_err(anyhow::Error::msg)
    }
    fn secondary_count_244(&mut self)->Result<i32>{
        let source=self.source.as_ref().context("actual ActorData38 source getter has not run")?;
        self.application.with_data_pre_request(|data|data.actor_view_secondary_count_244(source)).map_err(anyhow::Error::msg)
    }
    fn source_word_23c(&mut self)->Result<u32>{
        let source=self.source.as_ref().context("actual ActorData38 source getter has not run")?;
        self.application.with_data_pre_request(|data|data.actor_view_source_word_23c(source)).map_err(anyhow::Error::msg)
    }
}
struct GridView<'a>{handle:ViewHandle,fields:&'a mut Fields}
impl aa_runtime::view_grid_state::ViewFields<ViewHandle> for GridView<'_>{
    fn handle(&self)->ViewHandle{self.handle}
    fn word(&mut self,offset:u32)->std::result::Result<u32,String>{
        if !matches!(offset,0x38|0x3c|0x74|0x54|0x68){return Err("VGrid requested a different VThing field".into());}
        self.fields.required(offset as usize).map_err(|e|e.to_string())
    }
    fn store(&mut self,offset:u32,value:u32)->std::result::Result<(),String>{self.fields.grid_store(offset,value).map_err(|e|e.to_string())}
}
impl ViewDirectory {
    ///Called by the concrete VWorld allocation stage; not an initialized/ready
    ///view constructor. Empty directory does not certify an empty native view.
    pub fn for_staged_allocation(staged:&Staged)->Result<Self> {
        Self::allocation(&Parts::staged(staged)?)
    }
    pub fn for_world_allocation(world:&World<'_>)->Result<Self>{Self::allocation(&Parts::world(world)?)}
    pub(crate) fn require_world(&self,world:&World<'_>)->Result<()>{let p=Parts::world(world)?;self.validate(p.actors,p.spatial,p.revision,p.map)}
    fn allocation(parts:&Parts<'_>)->Result<Self>{
        Ok(Self {actors:parts.actors.identity_witness(),spatial:parts.spatial.identity_witness(),
            revision:parts.revision.witness(),map:parts.map.clone(),next:1,sequence:0,entries:BTreeMap::new(),requests:VecDeque::new(),updates:VecDeque::new(),
            data_identities:Vec::new()})
    }
    fn validate(&self,actors:&ActorStore,spatial:&SpatialWorld,revision:&ConstructedWorldRevision,map:&SourceIdentity)->Result<()> {
        ensure!(self.actors.is_for(actors)&&self.spatial.is_for(spatial)&&self.revision.is_for(revision)&&&self.map==map,
            "view directory belongs to another/retired World allocation");Ok(())
    }
    ///43D160's first source getter. The factory below deliberately performs
    ///its own second getter, as actual43AC20 does before choosing a ctor.
    pub fn source_view_type_bound(&self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,id:ActorId)->Result<u32>{
        let parts=Parts::binding(p);self.validate(parts.actors,parts.spatial,parts.revision,parts.map)?;
        let key=parts.actors.key(id).context("view source current GTID is stale")?;
        Ok(ViewSource::bind(catalog,parts.definitions,&parts.actors.object(key)?.definition)?.view_type())
    }
    ///Actual factory allocation precedes every constructor/source/model Init
    ///child. A required ctor retains the allocation and its reached fields.
    pub fn construct_bound_factory(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,id:ActorId,
        inputs:&aa_runtime::view_startup_inputs::Inputs)->Result<Option<ViewHandle>>{
        self.construct_factory(p,catalog,id,inputs,None)
    }
    pub fn construct_bound_factory_with_sources(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,id:ActorId,
        inputs:&aa_runtime::view_startup_inputs::Inputs,globals:&super::view_actor_globals::State,
        application:&mut aa_runtime::resource_depot::Application)->Result<Option<ViewHandle>>{
        self.construct_factory(p,catalog,id,inputs,Some((globals,application)))
    }
    fn construct_factory(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,id:ActorId,
        inputs:&aa_runtime::view_startup_inputs::Inputs,
        actor_sources:Option<(&super::view_actor_globals::State,&mut aa_runtime::resource_depot::Application)>)->Result<Option<ViewHandle>>{
        let parts=Parts::binding(p);self.validate(parts.actors,parts.spatial,parts.revision,parts.map)?;
        let key=parts.actors.key(id).context("view factory current GTID is stale")?;
        let object=parts.actors.object(key)?;
        let source=ViewSource::bind(catalog,parts.definitions,&object.definition)?;
        //43AC20's unsupported/default branch returns NULL; the caller's
        //43C426 null-Init boundary remains distinct from the source getter.
        if !(2..=26).contains(&source.view_type){return Ok(None)}
        ensure!(object.thing.word_c==0,"native factory current Thing+C is already published");
        ensure!(!self.entries.values().any(|v|v.actor==id),"native factory already retains this Thing's allocation/prefix");
        let next=self.next.checked_add(1).context("portable view handle exhaustion")?;
        let sequence=self.sequence.checked_add(1).context("view allocation sequence exhaustion")?;
        let handle=ViewHandle(self.next);let ticket=Arc::new(Ticket{handle,sequence,map:self.map.clone()});
        let view_type=source.view_type;
        self.entries.insert(handle,TypedVObject{actor:id,key,source,ticket,backend:None,native_fields:None,publication:None,
            residency:None,constructor_complete:false,init:Init::NotStarted,updated:false,
            model:None,render_payload:None,
            actor_fields:None,
            organization_fields:None,
            variant:object.object.as_ref().map(|o|o.variant),stance:object.actor.as_ref().and_then(|a|a.words.get(&0x15c).copied()),
            construction:object.actor.as_ref().map(|a|a.components.state.construction())});
        self.next=next;self.sequence=sequence;
        if matches!(view_type,2|3|4){
            let elapsed=||p.clock.begin.0;
            self.construct_model_base(&parts,catalog,handle,&mut Constructor{catalog,bank:parts.definitions,
                seeds:inputs.seeds(),orientation:inputs.orientation(),elapsed:&elapsed})?;
        }
        if matches!(view_type,3|4){
            let(globals,application)=actor_sources.context("453F10 requires the actual process RGB/source Data owners after its retained455050 prefix")?;
            let mut sources=ActorConstructor{catalog,bank:parts.definitions,definition:&object.definition,seeds:inputs.seeds(),globals,application,source:None};
            let entry=self.entries.get_mut(&handle).context("native view allocation absent")?;
            match super::view_actor_fields::Fields::construct(&mut sources){
                Ok(fields)=>{entry.actor_fields=Some(fields);entry.constructor_complete=view_type==3;},
                Err(failure)=>{entry.actor_fields=Some(failure.fields);return Err(failure.error)},
            }
        }
        if view_type==4{
            let entry=self.entries.get_mut(&handle).context("native view allocation absent")?;
            match super::view_organization_fields::Fields::construct(){
                Ok(fields)=>{entry.organization_fields=Some(fields);entry.constructor_complete=true;},
                Err(failure)=>{entry.organization_fields=Some(failure.fields);return Err(failure.error)},
            }
        }
        ensure!(matches!(view_type,2|3|4),"43AC20 view type{view_type} requires its actual derived constructor after the retained prefix");
        Ok(Some(handle))
    }
    fn bound_entry(&self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,handle:ViewHandle)->Result<&TypedVObject>{
        let parts=Parts::binding(p);self.validate(parts.actors,parts.spatial,parts.revision,parts.map)?;
        let entry=self.entries.get(&handle).context("native view allocation absent")?;
        ensure!(parts.actors.key(entry.actor)==Some(entry.key),"native view current GTID allocation changed");
        let object=parts.actors.object(entry.key)?;
        let source=ViewSource::bind(catalog,parts.definitions,&object.definition)?;
        ensure!(same_source(&source,&entry.source),"native view canonical source changed");
        Ok(entry)
    }
    pub fn flags_bound(&self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,handle:ViewHandle)->Result<u32>{
        let entry=self.bound_entry(p,catalog,handle)?;
        ensure!(entry.init==Init::Complete,"43C420 flags require the actual completed derived Init");
        entry.native_fields.as_ref().context("native view constructor fields absent")?.flags()
    }
    ///43C420's actual push-front list. The private completed Init state is
    ///required before a caller can mutate9C/A0 or publish a positive link.
    pub fn prepend_update_bound(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,handle:ViewHandle)->Result<()>{
        let entry=self.bound_entry(p,catalog,handle)?;
        ensure!(entry.init==Init::Complete&&entry.native_fields.as_ref().context("native view fields absent")?.flags()?&(1<<20)!=0,
            "43C420 update list requires completed Init and actual flag20");
        ensure!(!entry.updated,"native view already entered the update list");
        self.updates.try_reserve(1)?;self.updates.push_front(handle);
        self.entries.get_mut(&handle).context("native view allocation absent")?.updated=true;Ok(())
    }
    pub fn update_order(&self)->impl Iterator<Item=ViewHandle>+'_ {self.updates.iter().copied()}
    pub fn publish_bound(&mut self,p:&mut ViewBindingParts<'_>,catalog:&Catalog<'_>,id:ActorId,view:Option<ViewHandle>)->Result<()>{
        let handle=view.context("43C420 null publication requires completed actual virtualC4 destruction")?;
        let entry=self.bound_entry(p,catalog,handle)?;
        ensure!(entry.actor==id&&entry.constructor_complete&&entry.init==Init::Complete&&entry.publication.is_none(),
            "43D160 publication lacks its actual constructor/derived Init receipt");
        let flags=entry.native_fields.as_ref().context("native view fields absent")?.flags()?;
        ensure!(flags&(1<<20)==0||entry.updated,"43D160 publication precedes required9C/A0 list insertion");
        let key=entry.key;
        ensure!(p.actors.object(key)?.thing.word_c==0,"43D160 current Thing+C changed before publication");
        p.actors.object_mut(key)?.thing.word_c=handle.raw();
        self.entries.get_mut(&handle).context("native view allocation absent")?.publication=Some(Published);Ok(())
    }
    pub fn initialize_bound(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,handle:ViewHandle)->Result<bool>{
        let entry=self.bound_entry(p,catalog,handle)?;
        ensure!(entry.constructor_complete,"view derived Init requires completed actual constructor");
        ensure!(entry.init==Init::NotStarted,"view derived Init was already attempted");
        let view_type=entry.source.view_type;
        self.entries.get_mut(&handle).context("native view allocation absent")?.init=Init::Running;
        bail!("view type{view_type} requires actual derived Init457A40/451F90 and source/resource/grid services")
    }
    ///Concrete457A40 over this SAME entry, Data/Resource application, current
    ///World fields and native Grid/AC loan. No generic success callback can
    ///mint the private completed-Init marker.
    pub fn initialize_bound_with(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,handle:ViewHandle,
        context:&mut InitContext<'_,'_>)->Result<bool>{
        let entry=self.bound_entry(p,catalog,handle)?;
        ensure!(entry.constructor_complete&&matches!(entry.source.view_type,2|3|4),"actual derived view constructor/Init is required");
        ensure!(entry.init==Init::NotStarted,"view derived Init was already attempted");
        context.require_world(p.revision)?;
        let entry=self.entries.get_mut(&handle).context("native view allocation absent")?;
        entry.init=Init::Running;
        let result=object_binding::initialize(p,catalog,entry,handle,&mut self.requests,context);
        if let Ok(success)=&result{entry.init=if *success{Init::Complete}else{Init::ReturnedFalse};}
        result.map_err(|error|view_failure("ViewDirectory::initialize_bound_with",&entry.source.proof,
            entry.native_fields.as_ref().and_then(|fields|fields.aspect_index().ok()),Some(entry.actor),entry.key,handle,error))
    }
    pub fn destroy_bound(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,handle:ViewHandle,free:u32)->Result<()>{
        let entry=self.bound_entry(p,catalog,handle)?;
        ensure!(entry.init==Init::ReturnedFalse&&free==1,"43C420 destruction requires actual false Init and free1");
        bail!("43C420 requires actual virtualC4 view destruction before null publication")
    }
    pub fn request_staged_model(&mut self,staged:&Staged,catalog:&Catalog<'_>,
        application:&mut aa_runtime::resource_depot::Application,presentation:&impl Presentation,id:ActorId)->Result<Option<UploadRequest>> {
        self.request_model(&Parts::staged(staged)?,catalog,application,presentation,id)
    }
    pub fn request_world_model(&mut self,world:&World<'_>,catalog:&Catalog<'_>,
        application:&mut aa_runtime::resource_depot::Application,presentation:&impl Presentation,id:ActorId)->Result<Option<UploadRequest>> {
        self.request_model(&Parts::world(world)?,catalog,application,presentation,id)
    }
    fn request_model(&mut self,parts:&Parts<'_>,catalog:&Catalog<'_>,
        application:&mut aa_runtime::resource_depot::Application,presentation:&impl Presentation,id:ActorId)->Result<Option<UploadRequest>> {
        let spatial=parts.spatial;
        self.validate(parts.actors,spatial,parts.revision,parts.map)?;
        let key=parts.actors.key(id).context("view request GTID is stale")?;
        let object=parts.actors.object(key)?;
        let source=ViewSource::bind(catalog,parts.definitions,&object.definition)?;
        if source.view_type==0 {
            //43D160 neither reads nor clears an old positive link here. None
            //means no new upload request; it certifies no absence of old views.
            return Ok(None);
        }
        let handle=self.entries.iter().find_map(|(h,v)|(v.actor==id).then_some(*h))
            .context("backend request requires its actual native factory allocation first")?;
        let entry=self.entries.get(&handle).context("native factory allocation absent")?;
        ensure!(object.thing.word_c==0||object.thing.word_c==handle.raw(),"backend request current Thing+C names another allocation");
        ensure!(entry.constructor_complete&&entry.init==Init::Complete,"backend model request requires its actual completed derived Init");
        ensure!(entry.residency.is_none(),"view backend request already exists");
        ensure!(entry.source.view_type==source.view_type&&entry.source.source_pc==source.source_pc
            &&same_source(&entry.source,&source),"view backend source differs from its native allocation");
        ensure!(matches!(source.view_type,2|3|4),"actual view type{} constructor/model Init service is required",source.view_type);
        let fields=object.object.as_ref().context("model-view current Object absent")?;
        let index=entry.native_fields.as_ref().context("native VObject fields absent")?.aspect_index()?;
        let model=entry.model.as_ref().context("native VObject Model7C is absent")?.clone();
        ensure!(model.kind()==aa_runtime::resource_objects::Kind::Model,"native VObject7C is not a Model allocation");
        let payload=application.resources().model_payload_40(&model).map_err(anyhow::Error::msg)?
            .context("native VObject Model7C has no actual retained source40 payload")?;
        payload.require_resource(&model).map_err(anyhow::Error::msg)?;
        let selected_path=model.source_file().context("native Model selected SourceFile is absent")?.logical_path().to_owned();
        let (aspect_id,source_folder)=application.with_data_pre_request(|data|{
            let aspect=data.object_aspect(catalog,parts.definitions,&object.definition,index as usize)?;
            Ok((data.aspect_ids(&aspect)?.to_owned(),data.aspect_source_folder(&aspect)?.to_owned()))
        }).map_err(|error|view_failure("ViewDirectory::request_model -> State::object_aspect",
            &object.definition.proof,Some(index),Some(id),key,handle,anyhow::Error::msg(error)))?;
        let placement=parts.actors.placement(key,spatial)?;
        let mut appearance=presentation.appearance(parts.actors,spatial,id,object,placement)?;
        if let Some(actor)=&entry.actor_fields{
            let color=[actor.word(0xc8)?.context("VActor C8 color absent")?,actor.word(0xcc)?.context("VActor CC color absent")?,actor.word(0xd0)?.context("VActor D0 color absent")?];
            appearance.team_color=Some(color.map(f32::from_bits));
        }
        let request=SpawnActor {thing_id:object.definition.proof.id.clone(),object_variant:fields.variant,resolved_aspect:Some(index),
            visual:ActorVisual {actor_id:id.raw(),native_position:placement.position_bits.map(f32::from_bits),native_facing:placement.facing_bits.map(f32::from_bits),
                scale:f32::from_bits(fields.scale_bits),visible:appearance.visible,team_color:appearance.team_color}};
        let binding=ModelBinding{thing_id:object.definition.proof.id.clone(),aspect_index:index,aspect_id,source_folder,
            model_path:selected_path,source:object.definition.proof.source.clone(),fields:None};
        let selected=SelectedModel{handle:model,input_sha256:payload.source_sha256().to_owned(),payload,aspect:index,position:placement.position_bits,
            facing:placement.facing_bits,scale:fields.scale_bits};
        //Registration precedes the UI packet conversion/upload. This
        //directory retains its move-only lease and same source40 allocation.
        let residency=application.register_view_resource(catalog,parts.definitions,parts.revision,&binding.model_path,&binding.source)
            .map_err(anyhow::Error::msg)?;
        let residency_witness=residency.witness();
        let entry=self.entries.get_mut(&handle).context("native factory allocation absent")?;
        let ticket=Arc::clone(&entry.ticket);entry.residency=Some(residency);entry.render_payload=Some(selected.payload.clone());
        Ok(Some(UploadRequest {ticket,request,binding,residency:residency_witness,selected}))
    }
    ///Only records the successful backend child. It deliberately does not
    ///publish Thing+C: native43C420's actual Init/list effects remain required.
    pub fn accept_staged_upload(&mut self,staged:&Staged,catalog:&Catalog<'_>,receipt:UploadReceipt)->Result<ViewHandle> {
        self.accept_upload(&Parts::staged(staged)?,catalog,receipt)
    }
    pub fn accept_world_upload(&mut self,world:&World<'_>,catalog:&Catalog<'_>,receipt:UploadReceipt)->Result<ViewHandle>{
        self.accept_upload(&Parts::world(world)?,catalog,receipt)
    }
    fn accept_upload(&mut self,parts:&Parts<'_>,catalog:&Catalog<'_>,receipt:UploadReceipt)->Result<ViewHandle>{
        self.validate(parts.actors,parts.spatial,parts.revision,parts.map)?;
        ensure!(receipt.renderer.is_current(),"GPU receipt renderer was dropped/reloaded");
        let entry=self.entries.get_mut(&receipt.ticket.handle).context("GPU receipt view allocation absent")?;
        ensure!(entry.residency.as_ref().is_some_and(|r|r.is_for(parts.revision)),"GPU receipt resource belongs to another World allocation");
        ensure!(Arc::ptr_eq(&entry.ticket,&receipt.ticket)&&receipt.ticket.map==self.map&&receipt.ticket.sequence<=self.sequence,
            "GPU receipt belongs to another view request");
        ensure!(parts.actors.key(entry.actor)==Some(entry.key),"GPU receipt current GTID allocation changed");
        let current=parts.actors.object(entry.key)?;
        let source=ViewSource::bind(catalog,parts.definitions,&current.definition)?;
        ensure!((current.thing.word_c==0||current.thing.word_c==receipt.ticket.handle.raw())&&current.definition.proof.id==entry.source.proof.id
            &&current.definition.proof.source==entry.source.proof.source&&source.view_type==entry.source.view_type&&source.source_pc==entry.source.source_pc
            &&current.object.as_ref().map(|o|o.variant)==entry.variant
            &&current.actor.as_ref().and_then(|a|a.words.get(&0x15c).copied())==entry.stance
            &&current.actor.as_ref().map(|a|a.components.state.construction())==entry.construction,
            "GPU receipt current definition/aspect/view link changed");
        ensure!(entry.init==Init::Complete&&entry.native_fields.as_ref().context("GPU receipt native fields absent")?.aspect_index()?==receipt.selected.aspect
            &&entry.model.as_ref().is_some_and(|m|m.same_identity(&receipt.selected.handle)),"GPU receipt nativeB4/Model7C allocation changed");
        receipt.selected.payload.require_resource(&receipt.selected.handle).map_err(anyhow::Error::msg)?;
        ensure!(entry.render_payload.as_ref().is_some_and(|p|p.same_identity(&receipt.selected.payload))
            &&receipt.selected.payload.source_sha256()==receipt.selected.input_sha256,"GPU receipt retained source40 allocation changed");
        let placement=parts.actors.placement(entry.key,parts.spatial)?;
        ensure!(placement.position_bits==receipt.selected.position&&placement.facing_bits==receipt.selected.facing
            &&current.object.as_ref().is_some_and(|o|o.scale_bits==receipt.selected.scale),"GPU receipt current pose/scale changed");
        ensure!(receipt.report.actor_id==entry.actor.raw()&&receipt.report.draw_slots>0,"GPU receipt has no actual uploaded current model");
        ensure!(entry.backend.is_none(),"GPU receipt already accepted");
        entry.backend=Some(receipt.renderer);Ok(receipt.ticket.handle)
    }
    pub fn flags_54(&self,handle:ViewHandle)->Result<u32> {
        self.entries.get(&handle).context("actual view directory handle absent")?.native_fields.as_ref()
            .context("native view constructor fields are not yet bound")?.flags()
    }
    pub fn request_order(&self)->impl Iterator<Item=ViewHandle>+'_ {self.requests.iter().copied()}
    fn enqueue_native_request(&mut self,handle:ViewHandle)->Result<()> {
        ensure!(self.entries.contains_key(&handle),"43D030 requires its actual current view allocation");
        append_request(&mut self.requests,handle)
    }
    ///Retains the actual base scalar allocation, including a constructor-error
    ///prefix. Derived ctor/Init and publication remain separate reached stages.
    pub(crate) fn construct_staged_model_base(&mut self,staged:&Staged,catalog:&Catalog<'_>,handle:ViewHandle,
        sources:&mut impl view_object_fields::ConstructorSources)->Result<()> {
        self.construct_model_base(&Parts::staged(staged)?,catalog,handle,sources)
    }
    pub(crate) fn construct_world_model_base(&mut self,world:&World<'_>,catalog:&Catalog<'_>,handle:ViewHandle,
        sources:&mut impl view_object_fields::ConstructorSources)->Result<()>{self.construct_model_base(&Parts::world(world)?,catalog,handle,sources)}
    ///Concrete current seed/AVar/World-clock binding for the common VObject
    ///base constructor. It cannot complete a derived Actor/Organization ctor.
    pub(crate) fn construct_world_object_base(&mut self,world:&World<'_>,catalog:&Catalog<'_>,handle:ViewHandle,
        seeds:&aa_runtime::view_grid_startup::Seeds,orientation:&aa_runtime::view_renderer_variables::State)->Result<()> {
        let parts=Parts::world(world)?;
        let elapsed=||world.clock().begin.0;
        self.construct_model_base(&parts,catalog,handle,&mut Constructor{catalog,bank:parts.definitions,seeds,orientation,elapsed:&elapsed})
    }
    fn construct_model_base(&mut self,parts:&Parts<'_>,catalog:&Catalog<'_>,handle:ViewHandle,
        sources:&mut impl view_object_fields::ConstructorSources)->Result<()> {
        self.validate(parts.actors,parts.spatial,parts.revision,parts.map)?;
        let entry=self.entries.get_mut(&handle).context("view constructor allocation absent")?;
        ensure!(entry.native_fields.is_none(),"native view base constructor already reached");
        ensure!(parts.actors.key(entry.actor)==Some(entry.key),"view constructor current GTID allocation changed");
        let current=parts.actors.object(entry.key)?;
        let source=ViewSource::bind(catalog,parts.definitions,&current.definition)?;
        ensure!(current.thing.word_c==0&&source.proof.id==entry.source.proof.id&&source.proof.source==entry.source.proof.source
            &&source.proof.template_chain==entry.source.proof.template_chain&&source.proof.category==entry.source.proof.category&&source.view_type==entry.source.view_type
            &&source.source_pc==entry.source.source_pc,"view constructor current source/view changed");
        match Fields::construct(true,sources){
            Ok(fields)=>{entry.native_fields=Some(fields);entry.constructor_complete=entry.source.view_type==2;Ok(())},
            Err(failure)=>{entry.native_fields=Some(failure.fields);Err(failure.error)},
        }
    }
    ///Resolver for runtime's positive-view callbacks. An integer alone never
    ///authorizes a view flag read; it must name this current actor allocation
    ///in this same directory and retain a current initialized backend owner.
    pub fn flags_for_current(&self,actors:&ActorStore,id:ActorId)->Result<Option<u32>> {
        ensure!(self.actors.is_for(actors),"view flag read uses another ActorStore");
        let key=actors.key(id).context("view flag read current GTID is stale")?;
        let current=actors.object(key)?;
        if current.thing.word_c==0 {return Ok(None);}
        let entry=self.entries.get(&ViewHandle(current.thing.word_c)).context("positive Thing+C has no owned typed view allocation")?;
        ensure!(entry.actor==id&&entry.key==key&&entry.source.proof.source==current.definition.proof.source
            &&entry.source.proof.id==current.definition.proof.id,"positive Thing+C view allocation/source differs");
        ensure!(entry.publication.is_some(),"positive Thing+C lacks actual native Init/list/publication effects");
        ensure!(entry.init==Init::Complete,"positive view lacks completed derived Init");
        Ok(Some(entry.native_fields.as_ref().context("positive view actual native fields are unbound")?.flags()?))
    }
}
fn append_request<V>(queue:&mut VecDeque<V>,value:V)->Result<()> {queue.try_reserve(1)?;queue.push_back(value);Ok(())}
///One queued view's two native callbacks during the VWorld+208 drain.
trait RequestDrain{
    type Entry:Clone;
    ///View vf74: 45B140 (VObject, VActor and organization views alike) tail
    ///jumps to its Data+10 vf1C.
    fn ready_74(&mut self,entry:&Self::Entry)->Result<bool>;
    ///45B720: Data vf18(1), then +54 &= ~bit28.
    fn release_45b720(&mut self,entry:&Self::Entry)->Result<()>;
}
///43EC60 segment 43ED1A..43ED53: while the head cell's view is ready, release
///it and unlink that head (50C510). The first not-ready view ends the walk;
///a repeated allocation is checked and released again. A Rust callback error
///leaves the head cell in place (the native callbacks cannot fail).
fn drain_ready_43ec60<W:RequestDrain>(queue:&mut VecDeque<W::Entry>,w:&mut W)->Result<()>{
    while let Some(front)=queue.front().cloned(){
        if !w.ready_74(&front)?{break}
        w.release_45b720(&front)?;
        queue.pop_front();
    }
    Ok(())
}
///One view's 45BCA0 request prefix.
trait UninitRequest{
    ///+54 bit28
    fn pending(&mut self)->Result<bool>;
    ///Data vf18(1)
    fn release(&mut self)->Result<()>;
    ///+54 &= ~bit28
    fn clear(&mut self)->Result<()>;
}
///45BCA0..45BCCB: a view still owing its pre-request releases it, clears
///bit28, then 43C490 -> 4F1430 removes the FIRST cell holding this view from
///[B36538]+208 (nothing when absent).
fn uninit_request_45bca0<E,W:UninitRequest>(queue:&mut VecDeque<E>,w:&mut W,is_view:impl Fn(&E)->bool)->Result<()>{
    if !w.pending()?{return Ok(())}
    w.release()?;w.clear()?;
    if let Some(index)=queue.iter().position(is_view){queue.remove(index);}
    Ok(())
}
pub use aa_runtime::data_pre_request::request::release::{ReleaseServices,RequiredReleaseServices};
struct DynServices<'a>(&'a mut dyn ReleaseServices);
impl ReleaseServices for DynServices<'_>{
    fn release_14(&mut self,resources:&mut aa_runtime::resource_depot::Resources<'_>,
        handle:&aa_runtime::resource_objects::Handle,delta:u32)->std::result::Result<(),String>{
        self.0.release_14(resources,handle,delta)
    }
}
///The SAME directory, World loan, Data/Resource application and release owner.
struct Requests<'a,'b,'w,'c>{directory:&'a mut ViewDirectory,p:&'b ViewBindingParts<'w>,catalog:&'b Catalog<'c>,
    application:&'a mut aa_runtime::resource_depot::Application,services:&'a mut dyn ReleaseServices}
impl Requests<'_,'_,'_,'_>{
    ///View+10: the current definition of this view's own allocation.
    fn definition(&self,handle:ViewHandle)->Result<RuntimeDefinition>{
        let entry=self.directory.bound_entry(self.p,self.catalog,handle)?;
        Ok(self.p.actors.object(entry.key)?.definition.clone())
    }
    fn release_data(&mut self,handle:ViewHandle)->Result<()>{
        let definition=self.definition(handle)?;
        let (catalog,bank)=(self.catalog,self.p.definitions);
        let services=&mut *self.services;
        self.application.with_data_pre_request_resources(|data,resources|
            data.release_object(catalog,bank,&definition,resources,&mut DynServices(services),1)).map_err(anyhow::Error::msg)
    }
    fn fields(&mut self,handle:ViewHandle)->Result<&mut Fields>{
        self.directory.entries.get_mut(&handle).context("native view allocation absent")?
            .native_fields.as_mut().context("native view fields absent")
    }
}
impl RequestDrain for Requests<'_,'_,'_,'_>{
    type Entry=ViewHandle;
    fn ready_74(&mut self,handle:&ViewHandle)->Result<bool>{
        let definition=self.definition(*handle)?;
        let (catalog,bank)=(self.catalog,self.p.definitions);
        self.application.with_data_pre_request_resources(|data,resources|data.object_ready(catalog,bank,&definition,resources))
            .map_err(anyhow::Error::msg)
    }
    fn release_45b720(&mut self,handle:&ViewHandle)->Result<()>{
        self.release_data(*handle)?;
        self.fields(*handle)?.clear_request_pending()
    }
}
struct Uninit<'r,'a,'b,'w,'c>{requests:&'r mut Requests<'a,'b,'w,'c>,handle:ViewHandle}
impl UninitRequest for Uninit<'_,'_,'_,'_,'_>{
    fn pending(&mut self)->Result<bool>{self.requests.fields(self.handle)?.request_pending()}
    fn release(&mut self)->Result<()>{self.requests.release_data(self.handle)}
    fn clear(&mut self)->Result<()>{self.requests.fields(self.handle)?.clear_request_pending()}
}
impl ViewDirectory{
    ///The VWorld+208 request-queue segment of the VWorld update 43EC60, which
    ///43FF60 (VWorld vtable 9FDFF8+C, the per-UI-frame update) runs while
    ///VWorld+AD is set. Readiness is Data vf1C; release is Data vf18(1)
    ///through `services` (RequiredReleaseServices until 5AA550 lands).
    pub fn drain_ready_requests_43ec60(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,
        application:&mut aa_runtime::resource_depot::Application,services:&mut dyn ReleaseServices)->Result<()>{
        let parts=Parts::binding(p);self.validate(parts.actors,parts.spatial,parts.revision,parts.map)?;
        let mut queue=std::mem::take(&mut self.requests);
        let result=drain_ready_43ec60(&mut queue,&mut Requests{directory:self,p,catalog,application,services});
        //Every reached unlink is retained; on error the head cell stays.
        self.requests=queue;
        result
    }
    ///The 45BCA0 request prefix of a VThing uninit (vtable+CC chain, e.g.
    ///VActor 452D10 -> VObject 458CE0 -> 45BCA0). The caller must already
    ///have run the derived uninit parts that precede 45BCA0 natively.
    pub fn uninit_request_45bca0(&mut self,p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,handle:ViewHandle,
        application:&mut aa_runtime::resource_depot::Application,services:&mut dyn ReleaseServices)->Result<()>{
        let parts=Parts::binding(p);self.validate(parts.actors,parts.spatial,parts.revision,parts.map)?;
        let mut queue=std::mem::take(&mut self.requests);
        let mut requests=Requests{directory:self,p,catalog,application,services};
        let result=uninit_request_45bca0(&mut queue,&mut Uninit{requests:&mut requests,handle},|v|*v==handle);
        self.requests=queue;
        result
    }
}
impl UploadRequest {
    pub fn upload(self,store:&AssetStore,settings:LoadSettings,live:&mut LiveScene,
        renderer:&mut SceneRenderer,objects:&mut SceneObjects)->Result<UploadReceipt> {
        self.residency.validate(&self.binding.model_path,&self.binding.source).map_err(anyhow::Error::msg)?;
        live.rendered_map_source.as_ref().context("view upload requires the actual selected static map source")?
            .require_identity(&self.ticket.map)?;
        self.selected.payload.require_resource(&self.selected.handle).map_err(anyhow::Error::msg)?;
        ensure!(self.selected.payload.source_sha256()==self.selected.input_sha256,"view upload source40 input SHA differs");
        let prepared=spawn::prepare_retained(store,settings,self.request,self.binding,&self.selected.payload)?;
        let report=live.append(renderer,objects,prepared)?;
        Ok(UploadReceipt {ticket:self.ticket,renderer:renderer.identity_witness(),report,selected:self.selected})
    }
}
#[cfg(test)]mod tests {
    use super::*;
    #[test]fn native_source_view_getters_all_pc() {
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-view-publication.json")).unwrap();
        for row in fixture["rows"].as_array().unwrap() {
            let c=&row["input"];let kind=match c["source"].as_str().unwrap(){"Thing"=>0,"Object"=>1,"Actor"=>2,"Camera"=>20,"Doodad"=>21,_=>unreachable!()};
            let mask=c.get("mask").and_then(|v|v.as_u64()).map(|v|v as u32);
            assert_eq!(select(kind,c["type"].as_u64().unwrap()as u32,mask).unwrap(),row["output"]["selected"].as_u64().unwrap()as u32);
        }
        fn is_send<T:Send>(){}is_send::<UploadRequest>();is_send::<UploadReceipt>();
    }
    ///research/skirmish-20261019-view-request-queue: original 43ED1A..43ED53
    ///(with real 45B720/50C510) and 45BCA0..45BCCB (real 43C490/4F1430).
    #[test]fn native_request_queue_drain_and_uninit_replay(){
        use serde_json::{Value,json};
        let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261019-view-request-queue/native-view-request-queue.json")).unwrap();
        struct Lab<'a>{c:&'a Value,flags:BTreeMap<String,u32>,trace:Vec<Value>,checks:BTreeMap<String,usize>,view:String}
        impl Lab<'_>{fn release(&mut self,v:&str){self.trace.push(json!({"call":"release","view":v,"delta":1}));}}
        impl RequestDrain for Lab<'_>{
            type Entry=(usize,String);
            fn ready_74(&mut self,e:&(usize,String))->Result<bool>{
                let n=self.checks.entry(e.1.clone()).or_insert(0);let answers=self.c["ready"][&e.1].as_array().unwrap();
                let answer=answers[(*n).min(answers.len()-1)].as_u64().unwrap()!=0;*n+=1;
                self.trace.push(json!({"call":"vf74","view":e.1}));Ok(answer)
            }
            fn release_45b720(&mut self,e:&(usize,String))->Result<()>{
                self.release(&e.1);*self.flags.get_mut(&e.1).unwrap()&=!0x10000000;
                self.trace.push(json!({"call":"free","cell":e.0}));Ok(())
            }
        }
        impl UninitRequest for Lab<'_>{
            fn pending(&mut self)->Result<bool>{Ok(self.flags[&self.view]&0x10000000!=0)}
            fn release(&mut self)->Result<()>{let v=self.view.clone();Lab::release(self,&v);Ok(())}
            fn clear(&mut self)->Result<()>{let v=self.view.clone();*self.flags.get_mut(&v).unwrap()&=!0x10000000;Ok(())}
        }
        let mut rows=0;
        for case in fixture["cases"].as_array().unwrap(){
            let (c,o)=(&case["input"],&case["output"]);
            let flags=c["views"].as_object().unwrap().iter().map(|(k,v)|(k.clone(),v.as_u64().unwrap() as u32)).collect();
            let mut lab=Lab{c,flags,trace:vec![],checks:BTreeMap::new(),view:c["uninit"].as_str().unwrap_or("").into()};
            let cells:VecDeque<(usize,String)>=c["queue"].as_array().unwrap().iter().enumerate().map(|(i,v)|(i,v.as_str().unwrap().to_string())).collect();
            let remaining:Vec<(usize,String)>=if c["kernel"]=="drain"{
                let mut q=cells;drain_ready_43ec60(&mut q,&mut lab).unwrap();q.into_iter().collect()
            }else{
                //4F1430 frees the removed cell: the cell absent afterwards.
                let mut q=cells.clone();let view=lab.view.clone();
                uninit_request_45bca0(&mut q,&mut lab,|e|e.1==view).unwrap();
                if let Some((i,_))=cells.iter().find(|e|!q.contains(e)){lab.trace.push(json!({"call":"free","cell":i}));}
                q.into_iter().collect()
            };
            let queue:Vec<&str>=remaining.iter().map(|(_,v)|v.as_str()).collect();
            let tail=remaining.last().map(|(i,_)|json!(i)).unwrap_or(Value::Null);
            assert_eq!(json!({"trace":lab.trace,"queue":queue,"tail":tail,"flags":lab.flags}),*o,"{}",c["label"]);
            rows+=1;
        }
        assert_eq!(rows,66);
    }
    #[test]fn native_request_list_fifo_retains_repeated_view_allocations(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-view-request-queue.json")).unwrap();
        for row in fixture["rows"].as_array().unwrap(){let mut queue=VecDeque::new();for value in row["input"].as_array().unwrap(){append_request(&mut queue,value.as_u64().unwrap()).unwrap();}assert_eq!(serde_json::json!(queue),row["output"]);}
    }
}
