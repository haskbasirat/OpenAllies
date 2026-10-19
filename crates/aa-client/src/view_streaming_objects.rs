//!The VGrid view streaming passes (42FAD0 / 42F670 and the 45B330 lifecycle,
//!aa_runtime::view_streaming) over the SAME view directory: VThing scalars
//!live in the directory's Fields, the VGrid+100/+108 lists and B39BB8
//!counters in the directory, the cells in the VWorld's VGrid, heights in its
//!VWorld+BC heightfield. The model/presentation virtuals (vt+0, +D0, +D4,
//!+E0, +E4) and the view+10 Data request/release are the caller's owners.
use super::{ViewDirectory,ViewHandle};
use aa_numeric::{Binary,Precision};
use aa_mapworld::restore::RuntimeDefinition;
use aa_runtime::{actor_world::ActorStore,catalog::Catalog,definitions::Bank,
    resource_objects::Handle,view_grid_state,view_heightfield,world_revision::ConstructedWorldRevision,
    view_streaming::{self as vs,BlockObjects,TerrainBlockCell,ViewKind,Views}};
use std::cell::RefCell;
type Result<T>=std::result::Result<T,String>;
#[path="view_request_services.rs"]mod request_services;

///The view model and presentation virtuals 45B330 / 45B430 / 45BB50 / 42F670
///reach. They belong to the model instance (view+78) and scene owners.
pub trait ModelLifecycle{
    ///vt+0: VObject 459E10, VActor 454780, VOrg 44FD70.
    fn update_0(&mut self,v:ViewHandle,kind:ViewKind)->Result<()>;
    ///vt+D0(near): 4553D0 (VActor 4518A0 jumps there; VOrg 44FED0).
    fn model_request_d0(&mut self,v:ViewHandle,kind:ViewKind,near:u32)->Result<()>;
    ///vt+D4: VObject 459690, VActor 4518B0 -> 459690, VOrg 44F660.
    fn model_release_d4(&mut self,v:ViewHandle,kind:ViewKind)->Result<()>;
    ///vt+E0: VObject 457D10, VActor/VOrg 451A50.
    fn show_e0(&mut self,v:ViewHandle,kind:ViewKind)->Result<()>;
    ///vt+E4: VObject 455470, VActor/VOrg 451B30.
    fn hide_e4(&mut self,v:ViewHandle,kind:ViewKind)->Result<()>;
}
///No model instance owner is bound: each virtual is an explicit error.
pub struct UnportedModelLifecycle;
fn name(kind:ViewKind,object:&'static str,actor:&'static str,org:&'static str)->&'static str{
    match kind{ViewKind::Object=>object,ViewKind::Actor=>actor,ViewKind::Organization=>org}
}
impl ModelLifecycle for UnportedModelLifecycle{
    fn update_0(&mut self,_:ViewHandle,k:ViewKind)->Result<()>{Err(format!("view vt+0 update {} is not ported",name(k,"459E10","454780","44FD70")))}
    fn model_request_d0(&mut self,_:ViewHandle,k:ViewKind,_:u32)->Result<()>{
        Err(format!("view vt+D0 model instance request {} (view+78 718AD0) is not ported",name(k,"4553D0","4518A0","44FED0")))
    }
    fn model_release_d4(&mut self,_:ViewHandle,k:ViewKind)->Result<()>{Err(format!("view vt+D4 model release {} is not ported",name(k,"459690","4518B0","44F660")))}
    fn show_e0(&mut self,_:ViewHandle,k:ViewKind)->Result<()>{Err(format!("view vt+E0 show {} is not ported",name(k,"457D10","451A50","451A50")))}
    fn hide_e4(&mut self,_:ViewHandle,k:ViewKind)->Result<()>{Err(format!("view vt+E4 hide {} is not ported",name(k,"455470","451B30","451B30")))}
}
///view+10 Data vt+14 / vt+18 and the Model (view+7C) +74/+78 words.
pub trait DataLifecycle{
    fn request_14(&mut self,definition:&RuntimeDefinition,delta:u32,mode:u32)->Result<()>;
    fn release_18(&mut self,definition:&RuntimeDefinition,delta:u32)->Result<()>;
    fn model_74_78(&mut self,model:&Handle)->Result<[u32;2]>;
}
///The actual Data/Resource application and resource-task driver.
pub struct CurrentData<'a,'c,'e>{
    pub catalog:&'a Catalog<'c>,pub bank:&'a Bank,pub biome:Option<&'a aa_runtime::vworld_resource_context::Biome>,
    pub application:&'a mut aa_runtime::resource_depot::Application,
    pub driver:RefCell<crate::battle_resource_tasks::Current<'a,'e>>,
}
impl DataLifecycle for CurrentData<'_,'_,'_>{
    ///The same Data vf14 dispatch as 45B530's (1, 0) pre-request.
    fn request_14(&mut self,definition:&RuntimeDefinition,delta:u32,mode:u32)->Result<()>{
        let mut policy=request_services::Policy(&self.driver);let mut backend=request_services::Driver(&self.driver);
        let(catalog,bank,biome)=(self.catalog,self.bank,self.biome);
        self.application.with_data_pre_request_resources(|data,resources|
            data.pre_request_object(catalog,bank,definition,biome,resources,&mut policy,&mut backend,delta,mode))
    }
    fn release_18(&mut self,definition:&RuntimeDefinition,delta:u32)->Result<()>{
        let mut driver=self.driver.try_borrow_mut().map_err(|_|"reentrant resource driver loan")?;
        let mut services=aa_runtime::resource_depot::release::ManagerRelease(&mut *driver);
        let(catalog,bank)=(self.catalog,self.bank);
        self.application.with_data_pre_request_resources(|data,resources|
            data.release_object(catalog,bank,definition,resources,&mut services,delta))
    }
    fn model_74_78(&mut self,_:&Handle)->Result<[u32;2]>{
        Err("42FB62 vt+3C/+40 read Model+74/+78; Resources has no public read of those Model words".into())
    }
}

///One VWorld update's view streaming over the SAME owners (4312F0's object
///side, aa_runtime::view_streaming::BlockObjects).
pub struct StreamingObjects<'a>{
    pub(super) directory:&'a mut ViewDirectory,
    pub(super) actors:&'a ActorStore,
    pub(super) grid:&'a view_grid_state::State<ViewHandle>,
    pub(super) height:&'a view_heightfield::State,
    pub(super) revision:&'a ConstructedWorldRevision,
    pub(super) model:&'a mut dyn ModelLifecycle,
    pub(super) data:&'a mut dyn DataLifecycle,
    pub(super) pc:Precision,
}
fn text(e:impl std::fmt::Display)->String{format!("{e:#}")}
impl<'a> StreamingObjects<'a>{
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn new(directory:&'a mut ViewDirectory,actors:&'a ActorStore,grid:&'a view_grid_state::State<ViewHandle>,
        height:&'a view_heightfield::State,revision:&'a ConstructedWorldRevision,model:&'a mut dyn ModelLifecycle,
        data:&'a mut dyn DataLifecycle,pc:Precision)->Self{
        Self{directory,actors,grid,height,revision,model,data,pc}
    }
    ///The SAME VWorld+BC heightfield, for the terrain pass beside this one.
    pub fn heightfield(&self)->&'a view_heightfield::State{self.height}
}
impl StreamingObjects<'_>{
    fn entry(&self,v:ViewHandle)->Result<&super::TypedVObject>{
        let entry=self.directory.entries.get(&v).ok_or("VGrid names no view allocation")?;
        if self.actors.key(entry.actor)!=Some(entry.key){return Err("VGrid view+14 Thing was destroyed; the VThing uninit (45BCA0) is not ported".into())}
        Ok(entry)
    }
    fn fields(&self,v:ViewHandle)->Result<&super::Fields>{self.entry(v)?.native_fields.as_ref().ok_or_else(||"VGrid view native fields are unbound".into())}
    fn definition(&self,v:ViewHandle)->Result<RuntimeDefinition>{
        let entry=self.entry(v)?;Ok(self.actors.object(entry.key).map_err(text)?.definition.clone())
    }
    ///Run one kernel with the directory's lists/counters lent out of it.
    fn with_lists<R>(&mut self,call:impl FnOnce(&mut vs::Lists<ViewHandle>,&mut vs::Stats,&mut Self)->Result<R>)->Result<R>{
        let mut lists=std::mem::take(&mut self.directory.streaming);
        let mut stats=self.directory.streaming_stats;
        let result=call(&mut lists,&mut stats,self);
        self.directory.streaming=lists;self.directory.streaming_stats=stats;
        result
    }
}
impl BlockObjects for StreamingObjects<'_>{
    fn sweep_42fad0(&mut self,block:TerrainBlockCell,planes:&[[u32;4];6])->Result<()>{
        let pc=self.pc;self.with_lists(|lists,_,w|vs::sweep_42fad0(lists,w,block,planes,pc))
    }
    fn traverse_42f670(&mut self)->Result<()>{
        self.with_lists(|lists,stats,w|vs::traverse_42f670(lists,stats,w))?;
        self.directory.streaming_passes=self.directory.streaming_passes.saturating_add(1);Ok(())
    }
}
impl Views for StreamingObjects<'_>{
    type V=ViewHandle;
    fn word(&mut self,v:ViewHandle,offset:u32)->Result<u32>{
        if !matches!(offset,0x18|0x1c|0x38|0x3c|0x40|0x54|0x58|0x74){return Err(format!("view streaming read VThing+{offset:X}"))}
        self.fields(v)?.required(offset as usize).map_err(text)
    }
    fn store(&mut self,v:ViewHandle,offset:u32,value:u32)->Result<()>{
        self.entry(v)?;
        self.directory.entries.get_mut(&v).and_then(|e|e.native_fields.as_mut()).ok_or("VGrid view native fields are unbound")?
            .streaming_store(offset as usize,value).map_err(text)
    }
    fn kind(&mut self,v:ViewHandle)->Result<ViewKind>{
        match self.entry(v)?.source.view_type{2=>Ok(ViewKind::Object),3=>Ok(ViewKind::Actor),4=>Ok(ViewKind::Organization),
            t=>Err(format!("VGrid view type{t} has no VObject/VActor/VOrg vtable"))}
    }
    fn attached_vfx_9c(&mut self,v:ViewHandle)->Result<bool>{Ok(self.fields(v)?.required(0x9c).map_err(text)?!=0)}
    fn model_74_78(&mut self,v:ViewHandle)->Result<[u32;2]>{
        let model=self.entry(v)?.model.clone().ok_or("42FB5A vt+FC 5A4ED0 reads a NULL view+7C Model")?;
        self.data.model_74_78(&model)
    }
    fn height_431550(&mut self,xy:[u32;2])->Result<Binary>{self.height.sample_for_world(self.revision,xy,self.pc)}
    fn grid_word(&mut self,offset:u32)->Result<u32>{
        self.grid.word(offset)?.ok_or_else(||format!("VGrid+{offset:X} has not been written"))
    }
    fn cell_views(&mut self,index:usize)->Result<Vec<ViewHandle>>{self.grid.cell_views(index)}
    fn update_0(&mut self,v:ViewHandle)->Result<()>{let k=self.kind(v)?;self.model.update_0(v,k)}
    fn model_request_d0(&mut self,v:ViewHandle,near:u32)->Result<()>{let k=self.kind(v)?;self.model.model_request_d0(v,k,near)}
    fn model_release_d4(&mut self,v:ViewHandle)->Result<()>{let k=self.kind(v)?;self.model.model_release_d4(v,k)}
    fn show_e0(&mut self,v:ViewHandle)->Result<()>{let k=self.kind(v)?;self.model.show_e0(v,k)}
    fn hide_e4(&mut self,v:ViewHandle)->Result<()>{let k=self.kind(v)?;self.model.hide_e4(v,k)}
    fn data_request_14(&mut self,v:ViewHandle,delta:u32,mode:u32)->Result<()>{let d=self.definition(v)?;self.data.request_14(&d,delta,mode)}
    fn data_release_18(&mut self,v:ViewHandle,delta:u32)->Result<()>{let d=self.definition(v)?;self.data.release_18(&d,delta)}
}
