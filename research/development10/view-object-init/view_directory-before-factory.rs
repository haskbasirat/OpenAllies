//!Same-simulation-thread view allocations and render-only upload transport.
//!A GPU upload is a backend child, never43C420 Init or74FF50 completion.
use super::{ActorVisual,LiveScene,battle::Presentation,spawn::{self,ModelBinding,SpawnActor,SpawnReport}};
use crate::{gpu::{RendererWitness,SceneRenderer},inspection::SceneObjects,loading::LoadSettings};
use aa_assets::AssetStore;
use aa_actors::ActorId;
use aa_mapworld::{SourceIdentity,bootstrap::DefinitionProof,restore::RuntimeDefinition};
use aa_runtime::{actor_world::{ActorStore,ActorStoreWitness,ObjectKey},catalog::Catalog,definitions::Bank,
    restoration::Staged,spatial_world::{SpatialWorld,SpatialWorldWitness},world_revision::{ConstructedWorldRevision,Witness}};
use aa_runtime::live_world_frame::World;
use anyhow::{Context,Result,ensure,bail};
use std::{collections::{BTreeMap,VecDeque},sync::Arc};
use super::view_object_fields::{self,Fields};

#[derive(Clone,Copy,Debug,PartialEq,Eq,PartialOrd,Ord)]
pub struct ViewHandle(usize);
impl ViewHandle {pub fn raw(self)->usize{self.0}}
///Source-only factory requirement. No constructor, GPU, or Init is certified.
pub struct ViewSource {proof:DefinitionProof,view_type:u32,source_pc:aa_numeric::Precision}
impl ViewSource {
    pub fn bind(catalog:&Catalog<'_>,bank:&Bank,d:&RuntimeDefinition)->Result<Self> {
        catalog.require_source_owner().map_err(anyhow::Error::msg)?;
        let owner=bank.combat_source().map_err(anyhow::Error::msg)?;
        ensure!(owner.source_identities()==catalog.actors.sources.as_slice(),"view source Bank belongs to another Catalog");
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
    ensure!(value==0||(2..=26).contains(&value),"43AC20 view type{value} has no supported native factory branch");
    Ok(value)
}
struct Ticket {handle:ViewHandle,sequence:u64,map:SourceIdentity}
//Minted only by the future concrete43C420/43D160 publication path. Base
//constructor or backend/common-base Init success cannot create this marker.
struct Published;
struct TypedVObject {
    actor:ActorId,key:ObjectKey,source:ViewSource,ticket:Arc<Ticket>,
    backend:Option<RendererWitness>,
    ///Actual native constructor/Init flags remain unbound until that port runs.
    native_fields:Option<Fields>,
    publication:Option<Published>,
    residency:aa_runtime::resource_depot::ViewResourceLease,
    variant:i32,stance:Option<u32>,construction:Option<bool>,
}
///Lives beside the actual client VWorld owner on SimulationThread. Mutable
///ActorStore, Catalog, source Bank and spatial state never enter a UI packet.
pub struct ViewDirectory {
    actors:ActorStoreWitness,spatial:SpatialWorldWitness,revision:Witness,map:SourceIdentity,
    next:usize,sequence:u64,entries:BTreeMap<ViewHandle,TypedVObject>,
    ///Sole actual VWorld208/20C request-list owner; duplicates retain FIFO order.
    requests:VecDeque<ViewHandle>,
}
pub struct UploadRequest {ticket:Arc<Ticket>,request:SpawnActor,binding:ModelBinding,
    residency:aa_runtime::resource_depot::ViewResourceWitness}
pub struct UploadReceipt {ticket:Arc<Ticket>,renderer:RendererWitness,report:SpawnReport}
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
}
struct Constructor<'a,'assets,'source>{catalog:&'a Catalog<'assets>,bank:&'a Bank,
    seeds:&'a aa_runtime::view_grid_startup::Seeds,orientation:&'a aa_runtime::view_renderer_variables::State,
    world:&'a World<'source>}
impl view_object_fields::ConstructorSources for Constructor<'_,'_,'_>{
    fn global_word(&mut self,address:u32)->Result<u32>{
        let mut globals=self.seeds.bind(self.catalog).map_err(anyhow::Error::msg)?;
        aa_runtime::view_grid_state::Globals::word(&mut globals,address).map_err(anyhow::Error::msg)
    }
    fn renderer_default_orientation(&mut self)->Result<[u32;3]>{self.orientation.current(self.catalog,self.bank).map_err(anyhow::Error::msg)}
    fn world_elapsed_e0(&mut self)->Result<u32>{Ok(self.world.clock().begin.0)}
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
            revision:parts.revision.witness(),map:parts.map.clone(),next:1,sequence:0,entries:BTreeMap::new(),requests:VecDeque::new()})
    }
    fn validate(&self,actors:&ActorStore,spatial:&SpatialWorld,revision:&ConstructedWorldRevision,map:&SourceIdentity)->Result<()> {
        ensure!(self.actors.is_for(actors)&&self.spatial.is_for(spatial)&&self.revision.is_for(revision)&&&self.map==map,
            "view directory belongs to another/retired World allocation");Ok(())
    }
    pub fn request_staged_model(&mut self,staged:&Staged,catalog:&Catalog<'_>,
        application:&aa_runtime::resource_depot::Application,presentation:&impl Presentation,id:ActorId)->Result<Option<UploadRequest>> {
        self.request_model(&Parts::staged(staged)?,catalog,application,presentation,id)
    }
    pub fn request_world_model(&mut self,world:&World<'_>,catalog:&Catalog<'_>,
        application:&aa_runtime::resource_depot::Application,presentation:&impl Presentation,id:ActorId)->Result<Option<UploadRequest>> {
        self.request_model(&Parts::world(world)?,catalog,application,presentation,id)
    }
    fn request_model(&mut self,parts:&Parts<'_>,catalog:&Catalog<'_>,
        application:&aa_runtime::resource_depot::Application,presentation:&impl Presentation,id:ActorId)->Result<Option<UploadRequest>> {
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
        ensure!(object.thing.word_c==0,"OnVWorldCreated only creates an absent current view");
        ensure!(!self.entries.values().any(|v|v.actor==id),"view allocation/request already exists for current GTID");
        ensure!(matches!(source.view_type,2|3),"actual view type{} constructor/model Init service is required",source.view_type);
        if let Some(actor)=&object.actor {
            ensure!(!actor.components.state.construction(),"current construction aspect4EAAA0 required");
            ensure!(actor.words.get(&0x15c)==Some(&u32::MAX),"current stance aspect4D0700 required");
        }
        let fields=object.object.as_ref().context("model-view current Object absent")?;
        let creation=catalog.actors.native_creation_definition(&object.definition.proof.id,source.source_pc)?;
        let index=if creation.variations_enabled&&fields.variant!=-1 {u32::try_from(fields.variant).context("negative view variant")?}
            else {aa_runtime::object_aspect_source::DefaultIndex::bind(catalog,parts.definitions,&object.definition)?.index()};
        let placement=parts.actors.placement(key,spatial)?;
        let appearance=presentation.appearance(parts.actors,spatial,id,object,placement)?;
        let request=SpawnActor {thing_id:object.definition.proof.id.clone(),object_variant:fields.variant,resolved_aspect:Some(index),
            visual:ActorVisual {actor_id:id.raw(),native_position:placement.position_bits.map(f32::from_bits),native_facing:placement.facing_bits.map(f32::from_bits),
                scale:f32::from_bits(fields.scale_bits),visible:appearance.visible,team_color:appearance.team_color}};
        let binding=spawn::bind(catalog.actors,&request,source.source_pc)?;
        //Registration precedes any model load/upload. The SAME directory
        //retains its move-only lease, while UI gets only immutable metadata.
        let residency=application.register_view_resource(catalog,parts.definitions,parts.revision,&binding.model_path,&binding.source)
            .map_err(anyhow::Error::msg)?;
        let residency_witness=residency.witness();
        let next=self.next.checked_add(1).context("portable view handle exhaustion")?;
        let sequence=self.sequence.checked_add(1).context("view request sequence exhaustion")?;
        let handle=ViewHandle(self.next);let ticket=Arc::new(Ticket {handle,sequence,map:self.map.clone()});
        self.next=next;self.sequence=sequence;
        self.entries.insert(handle,TypedVObject {actor:id,key,source,ticket:Arc::clone(&ticket),backend:None,native_fields:None,publication:None,residency,
            variant:fields.variant,stance:object.actor.as_ref().and_then(|a|a.words.get(&0x15c).copied()),
            construction:object.actor.as_ref().map(|a|a.components.state.construction())});
        Ok(Some(UploadRequest {ticket,request,binding,residency:residency_witness}))
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
        ensure!(entry.residency.is_for(parts.revision),"GPU receipt resource belongs to another World allocation");
        ensure!(Arc::ptr_eq(&entry.ticket,&receipt.ticket)&&receipt.ticket.map==self.map&&receipt.ticket.sequence<=self.sequence,
            "GPU receipt belongs to another view request");
        ensure!(parts.actors.key(entry.actor)==Some(entry.key),"GPU receipt current GTID allocation changed");
        let current=parts.actors.object(entry.key)?;
        let source=ViewSource::bind(catalog,parts.definitions,&current.definition)?;
        ensure!(current.thing.word_c==0&&current.definition.proof.id==entry.source.proof.id
            &&current.definition.proof.source==entry.source.proof.source&&source.view_type==entry.source.view_type&&source.source_pc==entry.source.source_pc
            &&current.object.as_ref().map(|o|o.variant)==Some(entry.variant)
            &&current.actor.as_ref().and_then(|a|a.words.get(&0x15c).copied())==entry.stance
            &&current.actor.as_ref().map(|a|a.components.state.construction())==entry.construction,
            "GPU receipt current definition/aspect/view link changed");
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
        self.construct_model_base(&parts,catalog,handle,&mut Constructor{catalog,bank:parts.definitions,seeds,orientation,world})
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
            Ok(fields)=>{entry.native_fields=Some(fields);Ok(())},
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
        ensure!(entry.backend.as_ref().is_some_and(RendererWitness::is_current),"positive view backend has been dropped/reloaded");
        Ok(Some(entry.native_fields.as_ref().context("positive view actual native fields are unbound")?.flags()?))
    }
}
fn append_request<V>(queue:&mut VecDeque<V>,value:V)->Result<()> {queue.try_reserve(1)?;queue.push_back(value);Ok(())}
impl UploadRequest {
    pub fn upload(self,store:&AssetStore,settings:LoadSettings,live:&mut LiveScene,
        renderer:&mut SceneRenderer,objects:&mut SceneObjects)->Result<UploadReceipt> {
        self.residency.validate(&self.binding.model_path,&self.binding.source).map_err(anyhow::Error::msg)?;
        live.rendered_map_source.as_ref().context("view upload requires the actual selected static map source")?
            .require_identity(&self.ticket.map)?;
        let prepared=spawn::prepare_bound(store,settings,self.request,self.binding)?;
        let report=live.append(renderer,objects,prepared)?;
        Ok(UploadReceipt {ticket:self.ticket,renderer:renderer.identity_witness(),report})
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
    #[test]fn native_request_list_fifo_retains_repeated_view_allocations(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/native-view-request-queue.json")).unwrap();
        for row in fixture["rows"].as_array().unwrap(){let mut queue=VecDeque::new();for value in row["input"].as_array().unwrap(){append_request(&mut queue,value.as_u64().unwrap()).unwrap();}assert_eq!(serde_json::json!(queue),row["output"]);}
    }
}
