//!Concrete VObject457A40/common45B530 over one retained Directory allocation.
//!Positive construction/stance source fields, radius-change callbacks and full
//!Data pre-request stay explicit reached boundaries until their owners exist.
use super::{ViewHandle,TypedVObject,GridView,append_request};
use crate::{battle_ui_bridge::Client,live_scene::{model_policy::UiPolicy,view_object_fields::{self,Fields},view_object_init,view_actor_init}};
use aa_runtime::{catalog::Catalog,live_world_frame::ViewBindingParts,actor_world::{ObjectKey,RuntimeObject},
    resource_objects::Handle,data_pre_request::aspect::Aspect,simulation_execution::Execution,
    view_init_world::ObjectInitParts,world_revision::ConstructedWorldRevision};
use anyhow::{Context,Result,ensure,bail};
use std::collections::VecDeque;
#[path="view_request_services.rs"]mod request_services;

///Only concrete application/World owner loans enter this context. The opaque
///Grid/AC loan is minted by the retained VWorld aggregate, not from raw flags.
pub struct InitContext<'owners,'run>{
    object:ObjectInitParts<'owners,ViewHandle>,
    application:&'owners mut aa_runtime::resource_depot::Application,
    client:&'owners Client,execution:&'owners mut Execution<'run>,
    request_policy:&'owners crate::live_scene::view_request_policy::State,
    replacement_color:Option<&'owners aa_runtime::view_replacement_color::State>,
    session:Option<&'owners aa_netcode::local_bootstrap::LocalBootstrap>,
    precision:aa_numeric::Precision,
}
impl<'owners,'run> InitContext<'owners,'run>{
    pub fn new(object:ObjectInitParts<'owners,ViewHandle>,application:&'owners mut aa_runtime::resource_depot::Application,
        client:&'owners Client,execution:&'owners mut Execution<'run>,
        request_policy:&'owners crate::live_scene::view_request_policy::State,precision:aa_numeric::Precision)->Self{
        Self{object,application,client,execution,request_policy,replacement_color:None,session:None,precision}
    }
    pub fn with_replacement_color(mut self,color:&'owners aa_runtime::view_replacement_color::State)->Self{self.replacement_color=Some(color);self}
    pub fn with_session(mut self,session:&'owners aa_netcode::local_bootstrap::LocalBootstrap)->Self{self.session=Some(session);self}
    pub(super) fn require_world(&self,world:&ConstructedWorldRevision)->Result<()>{self.object.require_world(world).map_err(anyhow::Error::msg)}
}
struct Current<'a,'world,'assets,'owners,'run>{
    parts:&'a ViewBindingParts<'world>,catalog:&'a Catalog<'assets>,key:ObjectKey,handle:ViewHandle,
    fields:&'a mut Fields,model:&'a mut Option<Handle>,requests:&'a mut VecDeque<ViewHandle>,
    source_pc:aa_numeric::Precision,context:&'a mut InitContext<'owners,'run>,
}
impl Current<'_,'_,'_,'_,'_>{
    fn object(&self)->Result<&RuntimeObject>{self.parts.actors.object(self.key).map_err(anyhow::Error::from)}
    fn actor(&self)->Result<bool>{
        let object=self.object()?;
        let actor=match object.definition.native_class{
            3|4=>true,2|6|7|9=>false,other=>bail!("actual runtime class{other} B4 dispatch is required"),
        };
        let expected=if actor{aa_actors::loading::LoadKind::Actor}else{aa_actors::loading::LoadKind::Object};
        ensure!(object.kind==expected&&object.object.is_some()&&object.actor.is_some()==actor,
            "view Init runtime constructor/storage disagrees with canonical class");Ok(actor)
    }
}
impl view_object_init::Services for Current<'_,'_,'_,'_,'_>{
    type Aspect=Aspect;type Model=Handle;
    fn variations(&mut self)->Result<bool>{
        Ok(self.catalog.actors.native_creation_definition(&self.object()?.definition.proof.id,self.source_pc)?.variations_enabled)
    }
    fn thing_present(&mut self)->Result<bool>{self.object()?;Ok(true)}
    fn thing_variant(&mut self)->Result<u32>{Ok(self.object()?.object.as_ref().context("7A7D60 actual Object+40 absent")?.variant as u32)}
    fn actor_present(&mut self)->Result<bool>{self.actor()}
    fn under_construction(&mut self)->Result<bool>{Ok(self.object()?.actor.as_ref().context("current Actor state100 absent")?.components.state.construction())}
    fn construction_count(&mut self)->Result<u32>{bail!("457A40 requires actual source construction-aspect count120 owner")}
    fn first_construction_aspect(&mut self)->Result<u32>{bail!("457A40 requires actual first source construction-aspect11C owner")}
    fn stance_present(&mut self)->Result<bool>{
        Ok(*self.object()?.actor.as_ref().context("4CF100 actual Actor absent")?.words.get(&0x15c).context("actual Actor stance15C unbound")?!=u32::MAX)
    }
    fn stance_aspect(&mut self)->Result<u32>{bail!("4D0700/45E5E0 requires actual source stance-aspect188 owner")}
    fn default_aspect(&mut self)->Result<u32>{
        Ok(aa_runtime::object_aspect_source::DefaultIndex::bind(self.catalog,self.parts.definitions,&self.object()?.definition)?.index())
    }
    fn random_aspect(&mut self)->Result<u32>{bail!("49B900 requires actual UIWorld+D4 RNG for a null Thing")}
    fn index(&self)->Result<u32>{self.fields.aspect_index()}
    fn store_index(&mut self,index:u32)->Result<()>{self.fields.set_aspect_index(index)}
    fn aspect(&mut self)->Result<Aspect>{
        let definition=&self.parts.actors.object(self.key)?.definition;let index=self.fields.aspect_index()?as usize;
        self.context.application.with_data_pre_request(|data|data.object_aspect(self.catalog,self.parts.definitions,definition,index))
            .map_err(|error|super::view_failure("VObject457A40::aspect -> State::object_aspect",
                &definition.proof,Some(index as u32),self.parts.actors.id(self.key),self.key,self.handle,anyhow::Error::msg(error)))
    }
    fn model_for_aspect(&mut self,aspect:&Aspect)->Result<Handle>{
        let context=&mut self.context;let mut policy=UiPolicy::new(context.client,context.execution);
        context.application.with_data_pre_request_resources(|data,resources|
            data.model_for_view454f70(self.catalog,self.parts.definitions,aspect,self.parts.view_biome,resources,&mut policy)).map_err(anyhow::Error::msg)
    }
    fn store_model(&mut self,model:Handle)->Result<()>{self.fields.bind_model_pointer()?;*self.model=Some(model);Ok(())}
    fn model_flags(&mut self)->Result<u32>{
        let model=self.model.as_ref().context("actual VObject Model7C absent")?;
        self.context.application.resources().flags_08(model).map_err(anyhow::Error::msg)
    }
    fn model_radius(&mut self)->Result<u32>{
        let model=self.model.as_ref().context("actual VObject Model7C absent")?;
        self.context.application.resources().model_radius_64(model).map_err(anyhow::Error::msg)
    }
    fn aspect_radius(&mut self,aspect:&Aspect)->Result<u32>{
        self.context.application.with_data_pre_request(|data|data.aspect_visual_radius_64(aspect)).map_err(anyhow::Error::msg)
    }
    fn thing_scale(&mut self)->Result<u32>{self.actor()?;Ok(self.object()?.object.as_ref().context("4D1E60 Object scale3C absent")?.scale_bits)}
    fn set_radius(&mut self,radius:u32)->Result<()>{
        struct Required;impl view_object_fields::RadiusUpdate for Required{}
        self.fields.set_radius(radius,&mut Required)
    }
    fn common_init(&mut self)->Result<bool>{
        self.fields.common_init(&mut Common{parts:self.parts,catalog:self.catalog,key:self.key,handle:self.handle,
            requests:self.requests,context:self.context})?;Ok(true)
    }
}
struct Common<'a,'world,'assets,'owners,'run>{parts:&'a ViewBindingParts<'world>,catalog:&'a Catalog<'assets>,key:ObjectKey,
    handle:ViewHandle,requests:&'a mut VecDeque<ViewHandle>,context:&'a mut InitContext<'owners,'run>}
impl view_object_fields::InitServices for Common<'_,'_,'_,'_,'_>{
    fn source_flags_1c(&mut self)->Result<u32>{Ok(self.parts.actors.object(self.key)?.definition.flags)}
    fn thing_pose(&mut self)->Result<Option<view_object_fields::ThingPose>>{
        let p=self.parts.actors.placement(self.key,self.parts.spatial)?;
        Ok(Some(view_object_fields::ThingPose{position:p.position_bits,facing:p.facing_bits,flags:p.flags}))
    }
    fn virtual_8c(&mut self,fields:&mut Fields)->Result<()>{
        let flags=self.parts.actors.placement(self.key,self.parts.spatial)?.flags;fields.refresh_thing_flag14(flags)
    }
    fn occupy(&mut self,fields:&mut Fields)->Result<()>{
        self.context.object.grid_mut().occupy(&mut GridView{handle:self.handle,fields},self.context.precision).map_err(anyhow::Error::msg)
    }
    fn pre_request_uvar_108(&mut self)->Result<bool>{
        Ok(self.context.client.pre_request_all_vthings_current(self.context.execution).map_err(anyhow::Error::msg)?!=0)
    }
    fn global_request_mode_b39930(&mut self)->Result<bool>{Ok(self.context.request_policy.current()!=0)}
    fn vworld_ac(&mut self)->Result<bool>{Ok(self.context.object.request_mode_ac())}
    fn source_pre_request(&mut self)->Result<()>{
        let definition=&self.parts.actors.object(self.key)?.definition;
        super::ViewSource::bind(self.catalog,self.parts.definitions,definition)?;
        let context=&mut self.context;let guards=context.application.resource_task_guards();
        let mut driver=crate::battle_resource_tasks::Current::new(context.client,context.execution,guards,context.precision);
        if let Some(session)=context.session{driver=driver.with_session(session);}
        let driver=std::cell::RefCell::new(driver);
        let mut policy=request_services::Policy(&driver);let mut backend=request_services::Driver(&driver);
        //The real source prepass stays before WorldInit. This dispatch never
        //prepares missing source fields late: it retains Data20/earlier child
        //stores and returns the precise reached source/payload error.
        context.application.with_data_pre_request_resources(|data,resources|data.pre_request_object(
            self.catalog,self.parts.definitions,definition,self.parts.view_biome,resources,&mut policy,&mut backend,1,0)).map_err(anyhow::Error::msg)
    }
    fn enqueue_request(&mut self,_:&mut Fields)->Result<()>{append_request(self.requests,self.handle)}
}
pub(super) fn initialize(p:&ViewBindingParts<'_>,catalog:&Catalog<'_>,entry:&mut TypedVObject,handle:ViewHandle,
    requests:&mut VecDeque<ViewHandle>,context:&mut InitContext<'_,'_>)->Result<bool>{
    context.require_world(p.revision)?;
    if matches!(entry.source.view_type,3|4){
        let precision=context.precision;
        return view_actor_init::initialize(&mut ActorCurrent{p,catalog,entry,handle,requests,context,source:None},precision)
    }
    let fields=entry.native_fields.as_mut().context("actual VObject constructor fields absent")?;
    let mut current=Current{parts:p,catalog,key:entry.key,handle,fields,model:&mut entry.model,requests,source_pc:entry.source.source_pc,context};
    let precision=current.context.precision;view_object_init::initialize(&mut current,precision)
}
struct ActorCurrent<'a,'world,'assets,'owners,'run>{
    p:&'a ViewBindingParts<'world>,catalog:&'a Catalog<'assets>,entry:&'a mut TypedVObject,handle:ViewHandle,
    requests:&'a mut VecDeque<ViewHandle>,context:&'a mut InitContext<'owners,'run>,
    source:Option<aa_runtime::data_pre_request::actor_view_source::ActorView>,
}
impl view_actor_init::Services for ActorCurrent<'_,'_,'_,'_,'_>{
    type TrackArray=std::convert::Infallible;
    fn base_init(&mut self)->Result<bool>{
        let fields=self.entry.native_fields.as_mut().context("actual VObject base fields absent")?;
        let mut current=Current{parts:self.p,catalog:self.catalog,key:self.entry.key,handle:self.handle,fields,
            model:&mut self.entry.model,requests:self.requests,source_pc:self.entry.source.source_pc,context:self.context};
        let pc=current.context.precision;view_object_init::initialize(&mut current,pc)
    }
    fn actor_present(&mut self)->Result<bool>{
        let object=self.p.actors.object(self.entry.key)?;
        let actor=matches!(object.definition.native_class,3|4);
        ensure!(matches!(object.definition.native_class,2|3|4|6|7|9)&&object.object.is_some()
            &&object.actor.is_some()==actor&&object.kind==if actor{aa_actors::loading::LoadKind::Actor}else{aa_actors::loading::LoadKind::Object},"451F90 runtime B4/storage differs");Ok(actor)
    }
    fn set_current_owner_color(&mut self)->Result<()>{
        let actor=self.p.actors.object(self.entry.key)?.actor.as_ref().context("451F90 Actor108 dispatch missing")?;
        let owner=*actor.words.get(&0xe8).context("451F90 actual ActorE8 owner unbound")?;
        let rgb=if owner==0{
            self.context.replacement_color.context("451F00 requires actual current AVar30 ReplacementColorDefault")?
                .current(self.catalog,self.p.definitions).map_err(anyhow::Error::msg)?
        }else{
            let owner=aa_actors::ActorId::from_raw(owner);let key=self.p.actors.key(owner).context("451F00 stored Kingdom identity is stale")?;
            let entity=self.p.actors.entity(key)?;let runtime=entity.runtime.as_ref().context("451F00 current Kingdom runtime absent")?;
            ensure!(runtime.definition.native_class==4,"451F00 stored owner is not a Kingdom");
            let aa_runtime::actor_world::Source::Special(special)=&entity.source else{bail!("451F00 Kingdom derived storage is absent")};
            ensure!(matches!(special.source,aa_runtime::actor_world::SpecialSource::Kingdom(_)),"451F00 Special owner is not Kingdom");
            let color=special.derived.as_ref().context("451F00 Kingdom derived shell absent")?.references.get(&0x1e8)
                .context("451F00 current KingdomColor pointer unbound")?.as_ref().context("451F00 would dereference NULL KingdomColor")?;
            aa_runtime::load_kingdom_glyph::current_rgb(self.catalog,self.p.definitions,color).map_err(anyhow::Error::msg)?
        };
        let fields=self.entry.actor_fields.as_mut().context("451F00 actual VActor fields absent")?;
        for(i,word)in rgb.into_iter().enumerate(){fields.store_init_word(0xc8+i*4,word)?;}Ok(())
    }
    fn position(&self)->Result<[u32;3]>{let f=self.entry.native_fields.as_ref().context("451F90 actual base fields absent")?;Ok([f.required(0x38)?,f.required(0x3c)?,f.required(0x40)?])}
    fn facing(&self)->Result<[u32;3]>{let f=self.entry.native_fields.as_ref().context("451F90 actual base fields absent")?;Ok([f.required(0x44)?,f.required(0x48)?,f.required(0x4c)?])}
    fn store_tail(&mut self,o:usize,v:u32)->Result<()>{self.entry.actor_fields.as_mut().context("451F90 VActor tail absent")?.store_init_word(o,v)}
    fn source_actor_38(&mut self)->Result<bool>{
        let d=&self.p.actors.object(self.entry.key)?.definition;
        self.context.application.with_data_pre_request(|data|{
            let source=data.actor_view_source(self.catalog,self.p.definitions,d)?;
            let value=data.actor_view_is_actor_38(&source)?;self.source=Some(source);Ok(value)
        }).map_err(anyhow::Error::msg)
    }
    fn track_count_260(&mut self)->Result<i32>{let source=self.source.as_ref().context("451F90 source38 not reached")?;
        self.context.application.with_data_pre_request(|data|data.actor_view_track_count_260(source)).map_err(anyhow::Error::msg)}
    fn construct_track_array(&mut self,count:i32)->Result<Self::TrackArray>{self.entry.actor_fields.as_mut().context("451F90 VActor tail absent")?.construct_pending_tracks(count)}
    fn store_track_array(&mut self,array:Self::TrackArray)->Result<()>{match array{}}
    fn current_track_count_148(&self)->Result<i32>{bail!("451F90 positive Track array constructor has not completed")}
    fn initialize_track(&mut self,_:usize)->Result<()>{bail!("42D830 requires the actual constructed Track owner")}
    fn ui_random_word_d4(&mut self)->Result<u32>{self.context.client.call(self.context.execution,|c|c.ui.next_view_random_word_d4()).map_err(anyhow::Error::msg)}
}
