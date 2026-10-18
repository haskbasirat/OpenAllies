//! Fresh4FA590 Actor dispatch over the same staged map owners.
//!
//! Organization bit7 takes4FA3D0/4FA150 before the ordinary4FA030 path. The
//! first-slot554AD0 placement happens before allocation. Fresh construction
//! uses source-Bank precision separately from live arithmetic, and never
//! substitutes saved Init(NULL) for the new actor's InitCreate/InitExisting.
use crate::{
    actor_world::{self, ActorStore, ObjectKey},
    catalog::{ActorBindings, Catalog}, definitions::Bank,
    map_replacement_queue::DefinitionRef, restoration::Staged,
    spatial_world::SpatialBindings,
};
use aa_actors::{ActorId, actor_creation::ActorParams, components::ComponentSlot as Slot,
    creation::{ActorCreationRecord, CreationRecord, ThingDefinition},
    world_creation::{self, CreateRequest, CreateWorld}};
use aa_numeric::Precision;
use aa_spatial::{collision::{self,Descriptor}, index::{ActorKey,SpatialIndex},
    index_world::CollisionPolicy, placement::{self,PlacementSearch},
    query::{QueryGeometry,QueryPolicy}};
use std::sync::Arc;
type Result<T> = std::result::Result<T,String>;
pub type Params = ActorParams<DefinitionRef,String>;
fn err(e:impl std::fmt::Display)->String {e.to_string()}
fn cr(e:impl std::fmt::Display)->world_creation::Error {world_creation::Error::World(e.to_string())}

struct Source {
    binding:Arc<ActorBindings>, pc:Precision,
}
impl Source {
    fn bind(catalog:&Catalog<'_>,bank:&Bank,d:&DefinitionRef)->Result<Self> {
        catalog.require_source_owner()?;
        let receipt=bank.combat_source()?;
        receipt.completed_means()?;
        if receipt.source_identities()!=catalog.actors.sources.as_slice() {
            return Err("fresh Actor source Bank belongs to another Catalog universe".into());
        }
        let entry=bank.require_usable(d.token)?;
        if entry.key.domain!="actordefinition" || !entry.key.name.eq_ignore_ascii_case(&d.canonical_ids) {
            return Err("4FA590 actual request definition/Bank identity differs".into());
        }
        catalog.require_untranslated_actor_identity(&d.canonical_ids)?;
        let pc=receipt.source_precision();
        let binding=catalog.actor_bindings(&d.canonical_ids,pc)?;
        if binding.definition.native_class!=3 || binding.definition.proof.category.native_kind!=2 {
            return Err("4FA590 requires actual ordinary ActorData/class3".into());
        }
        Ok(Self{binding,pc})
    }
    fn mask(&self)->Result<u32> {self.binding.definition.component_mask.ok_or("fresh Actor completed component mask absent".into())}
}

/// The exact source-layout first-slot rule4C2BC0, before578290.
/// None is native's assertion/-1 branch and requires its process latch owner.
pub fn first_defined_slot(primary:i32,groups:&[u32],parameters:&[Option<String>])->Result<Option<u32>> {
    let defined=|slot:usize|->Result<bool>{
        let group=*groups.get(slot).ok_or("4C2BC0 slot outside actual slot-group array")? as usize;
        Ok(parameters.get(group).ok_or("4C2BC0 group outside actual parameter array")?.is_some())
    };
    if primary>=0 && defined(primary as usize)? {return Ok(Some(primary as u32));}
    for slot in 0..groups.len() {if defined(slot)?{return Ok(Some(slot as u32));}}
    Ok(None)
}

/// Real4FA590 entry. Partial allocation/GTID/RNG/component stores stay in
/// Staged on a later required callback. The native parameter writebacks remain
/// in `params`, including an accepted organization placement and facing fixup.
pub fn create(staged:&mut Staged,catalog:&Catalog<'_>,params:&mut Params,startup:&crate::actor_creation_startup::StartupInputs,pc:Precision)->Result<Option<ObjectKey>> {
    let direction_table_precision=startup.startup_precision();
    let source=Source::bind(catalog,&staged.definitions,&params.definition)?;
    if source.mask()?&(1<<7)!=0 {
        //4FA590's first branch is unconditional once bit7 is set. Never route
        //this through ordinary4FA030 to avoid creating a memberless HQ actor.
        if !organization_preplacement(staged,catalog,params,&source,pc)? {return Ok(None);}
        let created=create_thing(staged,catalog,params,&source,direction_table_precision,pc)?;
        if let Some(actor)=created {
            spawn_organization(staged,catalog,params,actor,&source,pc)?;
            //4FA3D0's hero postpass follows the complete4FA150 member pass.
            return Err("4FA3D0 actual created-member hero traversal is required after member creation".into());
        }
        return Ok(None);
    }
    if source.mask()?&(1<<13)!=0 {
        let world=staged.load_world.as_ref().ok_or("4FA590 actual World owner absent")?;
        let id=&source.binding.definition.proof.id;
        if world.source_fields.word(id,0x430).map_err(err)?!=0 {
            return Err("4FA590 existing Settlement4F0B70/586B40 owner path required".into());
        }
        if world.source_fields.word(id,0x444).map_err(err)?!=0 {
            return Err("4FA590 Settlement affiliation4F0B70 path required".into());
        }
    }
    let world=staged.load_world.as_ref().ok_or("4FA590 actual World owner absent")?;
    if params.word_30==0 && world.source_fields.reference(&source.binding.definition.proof.id,0x4e0).map_err(err)?.is_some() {
        return Err("4FA590 positive required Object construction path required".into());
    }
    create_thing(staged,catalog,params,&source,direction_table_precision,pc)
}

fn organization_preplacement(staged:&mut Staged,catalog:&Catalog<'_>,params:&mut Params,source:&Source,pc:Precision)->Result<bool> {
    if params.organization_layout.is_some() {
        return Err("4FA150 explicit OrganizationLayout needs its actual parameter allocation, not only its IDS".into());
    }
    let data=catalog.layout_data(source.pc)?;
    let org=data.organization_component(catalog.actors,&source.binding.definition.proof.id,source.pc).map_err(err)?
        .ok_or("4FA150 bit7 source Organization component absent")?;
    let layout=data.layouts.get(org.layout as usize).ok_or("4FA150 canonical default LayoutData identity absent")?;
    if layout.slot_groups.is_empty(){return Ok(true);}
    let slot=first_defined_slot(layout.primary_slot,&layout.slot_groups,&org.parameters)?
        .ok_or("4C2BC0 no defined slot requires actual B3B6F5 assertion owner")?;
    let group=*layout.slot_groups.get(slot as usize).ok_or("578290 first slot outside actual array")? as usize;
    let id=org.parameters.get(group).and_then(Option::as_ref).ok_or("578290 native null slot ActorData")?;
    catalog.require_untranslated_actor_identity(id)?;
    let object=catalog.actors.native_object_definition(id,source.pc).map_err(err)?;
    let flags=catalog.actors.native_final_flags(id,source.pc).map_err(err)?;
    let shape=object.blocking.words;
    let world=staged.load_world.as_mut().ok_or("578290 actual World absent")?;
    if world.terrain.source_precision()!=source.pc {return Err("578290 terrain/source Bank precision differs".into());}
    if world.source_fields.source_precision()!=source.pc || world.source_fields.receipt().sources!=catalog.actors.sources {
        return Err("578290 actual collision source owner differs from Bank/Catalog".into());
    }
    let crate::load_world::SpatialOwner::Adopted(adopted)=&mut world.spatial else {
        return Err("578290 requires same fully adopted spatial owner".into());
    };
    if adopted.error.is_some() || !adopted.pending.is_empty() {return Err("578290 actual spatial adoption incomplete".into());}
    let extent=world.terrain.bound.layout.extent_i32.map(|n|(n as f32).to_bits());
    if adopted.world.index().precision()!=pc {return Err("578290 live spatial precision differs".into());}
    let geometry=QueryGeometry::from_native_extents(adopted.world.index(),extent,pc).map_err(err)?;
    let radius=placement::wrapper_radius(shape[1],adopted.world.index().grid().cell_size_bits,pc).map_err(err)?;
    let search=PlacementSearch{shape:collision::Shape{kind:shape[0],radius_bits:shape[1],bounds:[shape[2],shape[3],shape[4],shape[5]]},radius_bits:radius,rings:10,keep_direction:true,strict:false,large:flags&(1<<21)!=0};
    let mut xy=[params.position_bits[0],params.position_bits[1]];
    let mut facing=params.direction_bits;
    let (index,bindings)=adopted.world.split_index();
    let mut policy=PlacementPolicy{catalog,definitions:&staged.definitions,source_fields:&world.source_fields,bindings};
    if !placement::find_free_placement(index,&geometry,&mut xy,&mut facing,&search,&mut policy).map_err(err)? {
        return Ok(false);
    }
    params.position_bits=[xy[0],xy[1],0];
    Ok(true)
}

///554AD0 constructs an unfiltered descriptor: every unexpected external
///predicate is refused. The only reached typed source read is ActorData+1A8.
struct PlacementPolicy<'a,'source> {
    catalog:&'a Catalog<'source>, definitions:&'a Bank,
    source_fields:&'a aa_rules::native_source_requirements::NativeSourceRequirements,
    bindings:SpatialBindings<'a>,
}
impl PlacementPolicy<'_,'_> {
    fn id(&self,token:u32)->collision::Result<&str>{
        let entry=self.definitions.entry(token).map_err(|e|collision::Error::World(e))?;
        if entry.key.domain!="actordefinition" {return Err(collision::Error::World("578290 source definition domain differs".into()));}
        self.catalog.require_untranslated_actor_identity(&entry.key.name).map_err(collision::Error::World)?;
        let actual=self.bindings.source_geometry(token).map_err(|e|collision::Error::World(e.to_string()))?;
        if !actual.source.id.eq_ignore_ascii_case(&entry.key.name) {return Err(collision::Error::World("578290 spatial/source identity differs".into()));}
        Ok(&entry.key.name)
    }
}
fn unexpected<T>(name:&str)->collision::Result<T>{Err(collision::Error::World(format!("554AD0 unexpected positive descriptor child {name}")))}
impl QueryPolicy for PlacementPolicy<'_,'_>{fn definition_has_object_data(&self,_:u32)->collision::Result<bool>{unexpected("definition virtual30")}}
impl CollisionPolicy for PlacementPolicy<'_,'_>{
    fn definition_flags(&self,d:u32)->collision::Result<u32>{let id=self.id(d)?;self.catalog.actors.native_final_flags(id,self.source_fields.source_precision()).map_err(|e|collision::Error::World(e.to_string()))}
    fn definition_flag_1a8(&self,d:u32)->collision::Result<bool>{let id=self.id(d)?;self.source_fields.byte(id,0x1a8).map(|v|v!=0).map_err(|e|collision::Error::World(e.to_string()))}
    fn custom_filter(&mut self,_:&mut SpatialIndex,_:ActorKey,_:&mut Descriptor)->collision::Result<bool>{unexpected("custom_filter")}
    fn source_excludes(&mut self,_:&mut SpatialIndex,_:ActorKey,_:ActorKey,_:&mut Descriptor)->collision::Result<bool>{unexpected("source_excludes")}
    fn allegiance(&mut self,_:&mut SpatialIndex,_:ActorKey,_:&mut Descriptor)->collision::Result<Option<u32>>{unexpected("allegiance")}
    fn hostility_dispatch(&self,_:u32)->collision::Result<u32>{unexpected("hostility")}
    fn hostile(&mut self,_:&mut SpatialIndex,_:u32,_:Option<u32>,_:Option<u32>,_:&mut Descriptor)->collision::Result<bool>{unexpected("hostile")}
    fn moving_exclusion(&mut self,_:&mut SpatialIndex,_:ActorKey,_:&mut Descriptor)->collision::Result<bool>{unexpected("moving")}
    fn outer_actor(&mut self,_:&mut SpatialIndex,_:ActorKey,_:&mut Descriptor)->collision::Result<Option<u32>>{unexpected("outer_actor")}
}

fn create_thing(staged:&mut Staged,catalog:&Catalog<'_>,params:&mut Params,source:&Source,direction_table_precision:Precision,pc:Precision)->Result<Option<ObjectKey>> {
    let request=CreateRequest{class:3,definition_ids:source.binding.definition.proof.id.clone(),instance_name:params.name_utf16.clone(),actor_data:true,kingdom_data:false,
        organization_layout:params.organization_layout.clone(),mode:params.mode,byte_50:params.byte_50!=0,payload:()};
    world_creation::create(&request,&mut Bound{staged,catalog,params,source,direction_table_precision,pc}).map_err(err)
}
fn spawn_organization(_staged:&mut Staged,_catalog:&Catalog<'_>,_params:&Params,_actor:ObjectKey,_source:&Source,_pc:Precision)->Result<()> {
    Err("4FA150 requires actual Organization leader-first57AD30/4FBF40 member creation after fresh Actor initialization".into())
}
struct Bound<'a,'source>{staged:&'a mut Staged,catalog:&'a Catalog<'source>,params:&'a mut Params,source:&'a Source,direction_table_precision:Precision,pc:Precision}
impl Bound<'_,'_>{
    fn construct(&mut self)->Result<ObjectKey>{
        //The existing constructor kernel excludes scene callbacks. Admit its
        //null scene branch using the SAME manager/phase, never an assumed
        //headless flag. Positive presentation needs a callback inside the
        //constructor before later variant draws, so refuse that domain here.
        let world=self.staged.load_world.as_ref().ok_or("4EE9F0 actual World owner absent")?;
        let phase=world.dispatcher.as_ref().ok_or("50C270 actual WorldD8 owner absent")?.phase;
        if phase!=6 && world.presentation.attached_view().is_some(){
            return Err("4EE9F0 positive scene transform requires constructor-interleaved43B7D0 binding".into());
        }
        let record=ActorCreationRecord{base:CreationRecord{definition:ThingDefinition{key:self.source.binding.definition.proof.id.clone(),flags:self.source.binding.definition.flags},position_bits:self.params.position_bits,direction_bits:self.params.direction_bits,name_utf16:self.params.name_utf16.clone(),saved_id:ActorId::from_raw(self.params.requested_id)},variant:self.params.variant as i32,owner_word:self.params.owner,word_30:self.params.word_30,request_nonblocking:self.params.byte_52!=0};
        let key=self.staged.actors.construct_new_bound(self.catalog,Arc::clone(&self.source.binding),record,self.staged.clock.begin.0,&mut self.staged.rng,self.direction_table_precision,self.pc).map_err(err)?;
        let actor_world::Source::New{record,..}=&self.staged.actors.entity(key).map_err(err)?.source else{return Err("fresh allocation lost creation block".into());};
        self.params.direction_bits=record.base.direction_bits;self.params.owner=record.owner_word;
        Ok(key)
    }
}
impl CreateWorld<String,()> for Bound<'_,'_>{
    type Object=ObjectKey;
    fn instance_name_gtid(&mut self,_:&[u16])->world_creation::Result<Option<u32>>{Err(cr("4F9C10 positive instance-name4154D0 owner required"))}
    fn gtid_live(&self,id:u32)->world_creation::Result<bool>{Ok(self.staged.actors.key(ActorId::from_raw(id)).is_some())}
    fn log(&mut self,_:u32,_:&'static str,_:&str)->world_creation::Result<()>{Err(cr("4F9C10 actual diagnostic owner required"))}
    fn construct(&mut self,_:u32,_:u32,_:&CreateRequest<String,()>)->world_creation::Result<Option<ObjectKey>>{Bound::construct(self).map(Some).map_err(cr)}
    fn set_state(&mut self,key:ObjectKey,index:u8,enabled:bool)->world_creation::Result<()>{set_state(&mut self.staged.actors,key,index,enabled).map_err(cr)}
    fn create_component(&mut self,key:ObjectKey,slot:Slot)->world_creation::Result<()>{self.staged.actors.construct_ground(key,slot,aa_body::F32(self.staged.clock.begin.0),self.pc).map_err(cr)}
    fn definition_flags(&self,key:ObjectKey)->world_creation::Result<u32>{self.staged.actors.object(key).map_err(cr)?.definition.component_mask.ok_or_else(||cr("fresh mask absent"))}
    fn construction_assert_latch(&self)->world_creation::Result<bool>{Err(cr("4F9C10 actual B3CC42 latch owner required"))}
    fn set_construction_assert_latch(&mut self,_:bool)->world_creation::Result<()>{Err(cr("4F9C10 actual B3CC42 latch owner required"))}
    fn state_active(&mut self,key:ObjectKey,i:u8)->world_creation::Result<bool>{let v=self.staged.actors.object(key).map_err(cr)?.components().map_err(cr)?.state;Ok((u64::from(v.low)|(u64::from(v.high)<<32))&(1u64<<i)!=0)}
    fn assertion_dialog(&mut self,_:&'static str,_:u32)->world_creation::Result<bool>{Err(cr("fresh actual assertion process owner required"))}
    fn set_organization_layout(&mut self,_:ObjectKey,_:&String)->world_creation::Result<()>{Err(cr("57C450 actual requested layout allocation required"))}
    fn init_create(&mut self,key:ObjectKey,_:&CreateRequest<String,()>)->world_creation::Result<bool>{
        self.staged.actors.initialize_object(key,&mut self.staged.rng,self.pc).map_err(cr)?;
        let s=self.staged.actors.object(key).map_err(cr)?.components().map_err(cr)?.state;
        if aa_actors::state::group(u64::from(s.low)|(u64::from(s.high)<<32),13).map_err(cr)?.is_none(){set_state(&mut self.staged.actors,key,27,true).map_err(cr)?;}
        let mut effects=Assembly{inactive:self.staged.inactive_load.as_ref()};
        self.staged.actors.assemble_ground(key,self.catalog,aa_body::F32(self.staged.clock.begin.0),&mut self.staged.rng,&mut self.staged.owners,&mut self.staged.definitions,self.pc,&mut effects).map_err(cr)?;
        init_create_suffix(self.staged,self.catalog,key,self.params,self.source,self.pc).map_err(cr)
    }
    fn init_existing(&mut self,key:ObjectKey,_:&CreateRequest<String,()>)->world_creation::Result<bool>{
        crate::map_actor_init_existing::initialize(self.staged,self.catalog,key,self.params,self.pc).map_err(cr)
    }
    fn post_create(&mut self,_:ObjectKey,_:&CreateRequest<String,()>,_:u32)->world_creation::Result<()>{Err(cr("4CF300 fresh PostCreate owner required"))}
    fn register(&mut self,_:ObjectKey)->world_creation::Result<()>{Err(cr("4F9510 fresh registration owner required"))}
    fn register_updatable(&mut self,_:ObjectKey)->world_creation::Result<()>{Err(cr("50C220 fresh scene publication owner required"))}
    fn fires_creation_fx(&mut self,_:ObjectKey)->world_creation::Result<bool>{Ok(true)} // actual class3 B0=6A71D0
    fn creation_fx(&mut self,_:ObjectKey)->world_creation::Result<()>{Err(cr("42A590 fresh creation FX owner required"))}
    fn destroy(&mut self,_:ObjectKey)->world_creation::Result<()>{Err(cr("4F5490 fresh failed-initialization destruction owner required"))}
}
fn source_text(catalog:&Catalog<'_>,source:&Source,field:&str)->Result<Option<String>>{
    match catalog.actors.native_field(&source.binding.definition.proof.id,field,source.pc).map_err(err)? {
        None=>Ok(None),Some(aa_rules::native::TypedValue::String{value,..})=>{
            Ok((!value.is_empty()&&!value.eq_ignore_ascii_case("null")).then_some(value))
        },Some(_)=>Err(format!("fresh native source field{field} has wrong type")),
    }
}
fn init_create_suffix(staged:&mut Staged,catalog:&Catalog<'_>,key:ObjectKey,params:&Params,source:&Source,pc:Precision)->Result<bool>{
    //4D6F10 numbers after complete assembly, and mode4 skips both branches.
    if params.mode!=4 {
        let list=match source_text(catalog,source,"name_list_ids")? {
            Some(name)=>catalog.actors.native_name_list_with_lookup(&name,source.pc,&mut |id|{
                staged.definition_text.lookup(id).map_err(|e|aa_rules::native::Error::Unsupported(e.to_string()))
            }).map_err(err)?,None=>None,
        };
        if list.is_some(){return Err("4D6F10 positive4F4A20 same World name-assignment owner required".into());}
        let id=staged.actors.id(key).ok_or("4D6F10 actual number owner GTID absent")?;
        if staged.actors.ordinary_owner(id).map_err(err)?.is_some() {
            let numbered=catalog.actors.native_field(&source.binding.definition.proof.id,"numbered",source.pc)
                .map_err(err)?.ok_or("4D6F10 numbered source absent")?.boolean().map_err(err)?;
            if numbered{crate::map_actor_numbers::assign_automatic(staged,key,&params.definition)?;}
        }
    }
    let has_character=staged.actors.object(key).map_err(err)?.components().map_err(err)?.present(Slot::Character);
    if params.byte_51!=0 && has_character {
        let object=staged.actors.object(key).map_err(err)?;
        let actor=object.actor.as_ref().ok_or("4D0720 actual Actor absent")?;
        if actor.words.get(&0x15c).copied()!=Some(u32::MAX){return Err("4D0720 actual state-view animation source required".into());}
        let character=source.binding.character.as_ref().ok_or("4D0720 actual Character source absent")?;
        let variant=object.object.as_ref().ok_or("4D0720 actual Object absent")?.variant;
        let set=if variant==-1 {u32::try_from(character.aspect_ids.len()).map_err(err)?}else{u32::try_from(variant).map_err(err)?};
        //view+48/+4C is the kind5 summon/start list, not a preview-model flag.
        if character.for_variant(set).map_err(err)?.iter().any(|a|a.timing.kind==5){
            return Err("4D6F10 positive563D50 fresh Character start owner required".into());
        }
    }
    if has_character {
        crate::map_character_stance::default_stance(staged,catalog,key,pc)?;
    }else{
        //4CFC80 no-Character path clears the same active word before old virtual40.
        let object=staged.actors.object_mut(key).map_err(err)?;
        let old=std::mem::replace(&mut object.active_component,actor_world::ActiveComponent::None);
        if old!=actor_world::ActiveComponent::None{return Err("4CFC80 positive old active-component virtual40 owner required after clear".into());}
    }
    if source_text(catalog,source,"MoverComponent.trailer_actor")?.is_some(){
        return Err("4D6F10 positive linked4FA030 creation owner required".into());
    }
    for slot in aa_actors::components::ALL_SLOTS {
        if staged.actors.object(key).map_err(err)?.components().map_err(err)?.present(slot)
            && !crate::map_component_init_create::initialize(staged,catalog,key,slot,params,pc)? {
            return Ok(false);
        }
    }
    if staged.actors.object(key).map_err(err)?.components().map_err(err)?.state.construction(){
        return Err("4D6F10 positive54BFC0 current owner Construction registration required".into());
    }
    Ok(true)
}
struct Assembly<'a>{inactive:Option<&'a crate::restore_lifecycle::InactiveLoad>}
impl actor_world::AssemblyEffects for Assembly<'_>{
    fn completed(&mut self,store:&mut ActorStore,actor:ActorId,_:&[Slot])->actor_world::Result<()>{
        if store.ordinary_owner(actor)?.is_none(){return Ok(());}
        let scope=self.inactive.ok_or(actor_world::Error::Missing("4D30BA actual SAI lifetime owner"))?;
        if !scope.observer_present().map_err(actor_world::Error::Source)? || !scope.observer_active().map_err(actor_world::Error::Source)? {return Ok(());}
        Err(actor_world::Error::Missing("4D30BA positive79EA90/current parent/view7B41E0 owner"))
    }
}
pub(crate) fn set_state(store:&mut ActorStore,key:ObjectKey,index:u8,enabled:bool)->Result<()> {
    let placement=store.staged_placement(key).map_err(err)?;
    let state=store.object(key).map_err(err)?.components().map_err(err)?.state;
    let mut f=aa_actors::state::Fields{bits:u64::from(state.low)|(u64::from(state.high)<<32),thing_flags:placement.flags};
    let mut effects=StateEffects{store,key};
    let result=aa_actors::state::set(&mut f,index,enabled,&mut effects).map_err(err);
    effects.commit(&f).map_err(err)?;result
}
struct StateEffects<'a>{store:&'a mut ActorStore,key:ObjectKey}
impl StateEffects<'_>{fn commit(&mut self,f:&aa_actors::state::Fields)->actor_world::Result<()>{
    let s=&mut self.store.object_mut(self.key)?.components_mut()?.state;s.low=f.bits as u32;s.high=(f.bits>>32)as u32;
    self.store.staged_placement_mut(self.key)?.flags=f.thing_flags;Ok(())
}}
impl aa_actors::state::Effects for StateEffects<'_>{
    fn leave(&mut self,f:&mut aa_actors::state::Fields,_:&'static aa_actors::state::Descriptor)->aa_actors::state::Result<()>{self.commit(f).map_err(|e|aa_actors::state::Error::Callback(e.to_string()))?;Err(aa_actors::state::Error::Callback("fresh live4D16B0 required".into()))}
    fn enter(&mut self,f:&mut aa_actors::state::Fields,_:&'static aa_actors::state::Descriptor,_:Option<&'static aa_actors::state::Descriptor>)->aa_actors::state::Result<()>{self.commit(f).map_err(|e|aa_actors::state::Error::Callback(e.to_string()))?;Err(aa_actors::state::Error::Callback("fresh live4D4A90 required".into()))}
    fn rebuild_abilities(&mut self,f:&mut aa_actors::state::Fields)->aa_actors::state::Result<()>{self.commit(f).map_err(|e|aa_actors::state::Error::Callback(e.to_string()))?;let id=self.store.id(self.key).ok_or(aa_actors::state::Error::Callback("fresh state GTID absent".into()))?;self.store.rebuild_construction_actions(id).map_err(|e|aa_actors::state::Error::Callback(e.to_string()))}
}
