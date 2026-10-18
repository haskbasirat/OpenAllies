//!4F61A0 StartMission camera over the SAME World camera and current ActorStore.
//! Local Kingdom resolution reuses4F5290's existing native kernel. None of the
//! unrelated4F1C20 priority/membership inputs authorizes an invented fallback.
use crate::{actor_world::{self,Source,SpecialSource,Spatial},catalog::Catalog,
 load_tail_world::{self,SessionRead},restoration::Staged};
use aa_actors::{ActorId,world_load_tail};
use aa_numeric::{Binary,Precision};
use std::collections::BTreeMap;
type Result<T>=std::result::Result<T,String>;
#[derive(Clone,Debug,PartialEq,Eq)]
pub enum Log {
 Observer{xy:[u32;2],double_arguments:[u64;2]},
 Capital{xy:[u32;2],double_arguments:[u64;2]},
 MissingCapital,
}
#[derive(Clone,Copy)]
enum Choice {Observer([u32;2]),Capital([u32;2]),MissingCapital}
fn apply(camera:&mut load_tail_world::CameraState,messages:&mut Vec<Log>,choice:Choice,pc:Precision)->Result<()> {
 let (xy,double_arguments,observer)=match choice {
  Choice::Observer(extents)=>{
   let half=Binary::from_f32_bits(0x3f000000).map_err(|e|e.to_string())?;
   let mut xy=[0;2];let mut double_arguments=[0;2];
   for i in 0..2 {
    let value=Binary::from_f32_bits(extents[i]).map_err(|e|e.to_string())?.mul(&half,pc).map_err(|e|e.to_string())?;
    xy[i]=value.store_f32().map_err(|e|e.to_string())?;
    //The observer path retains ST0 after the float camera store. The log's
    //qword argument can retain subnormal bits rounded out of that float.
    double_arguments[i]=aa_numeric::crt_pow::store_binary64(&value).map_err(|e|e.to_string())?;
   }
   (xy,double_arguments,true)
  },
  Choice::Capital(xy)=>{
   if xy.iter().any(|&b|!f32::from_bits(b).is_finite()){return Err("4F61A0 nonfinite capital x87 load/store conversion requires its numeric producer".into())}
   (xy,xy.map(|bits|f64::from(f32::from_bits(bits)).to_bits()),false)
  },
  Choice::MissingCapital=>{messages.push(Log::MissingCapital);return Ok(())},
 };
 camera.look_at=Some(xy);
 messages.push(if observer{Log::Observer{xy,double_arguments}}else{Log::Capital{xy,double_arguments}});Ok(())
}
struct CurrentSpatial<'a>(&'a crate::load_world::World);
impl Spatial for CurrentSpatial<'_> {
 fn placement(&self,id:ActorId)->actor_world::Result<actor_world::PlacementState> {
  match &self.0.spatial {crate::load_world::SpatialOwner::Adopted(v)=>v.world.placement(id),
   _=>Err(actor_world::Error::Missing("mission camera published placement has no adopted owner"))}
 }
 fn publish(&mut self,_:ActorId,_:actor_world::PlacementState)->actor_world::Result<()> {Err(actor_world::Error::Missing("camera cannot publish placement"))}
}
pub(crate) fn run<S:SessionRead+?Sized>(staged:&mut Staged,catalog:&Catalog<'_>,session:&S,pc:Precision)->Result<()> {
 catalog.require_source_owner()?;
 let source=staged.definitions.combat_source()?;source.completed_means()?;
 if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("mission camera moved source universe differs".into())}
 let world=staged.load_world.as_mut().ok_or("mission camera actual World absent")?;
 let mode=world.map_mode.as_ref().ok_or("mission camera actual MapResource+A8 absent")?;
 //These absent/empty inputs are required-error bindings for UNREACHED4F1C20
 //services. Only4F5290 is called: it reads actual mode/48/player/Kingdom names.
 let members=BTreeMap::new();let priorities=BTreeMap::new();
 let kingdom={
  let mut local=load_tail_world::World{actors:&mut staged.actors,session,
   facts:load_tail_world::WorldFacts{map_settings_mode:mode,world_start_flag:None,
    world_word_48:Some(world.mission_fields.started48()),map_size_bits:None,
    kingdom_slots:Some(world.kingdoms.all()),kingdom_actors:&members,definition_priorities:&priorities},
   camera:&mut world.tail.camera,precision:pc,world_start:None};
  world_load_tail::local_kingdom(&mut local).map_err(|e|e.to_string())?
 };
 let choice=if let Some(id)=kingdom {
  let key=staged.actors.key(ActorId::from_raw(id)).ok_or("mission camera local Kingdom GTID absent")?;
  let entity=staged.actors.entity(key).map_err(|e|e.to_string())?;
  let Source::Special(v)=&entity.source else{return Err("mission camera local owner is not special".into())};
  let SpecialSource::Kingdom(saved)=&v.source else{return Err("mission camera local owner is not Kingdom".into())};
  if saved.serialized_id!=id{return Err("mission camera local Kingdom identity differs".into())}
  let shell=v.derived.as_ref().ok_or("mission camera local Kingdom fields absent")?;
  let capital=match shell.identity_references.get(&0x1f8){Some(v)=>*v,None if shell.words.get(&0x1f8)==Some(&0)=>None,_=>return Err("mission camera capital pointer unbound".into())};
  if let Some(id)=capital {
   let key=staged.actors.key(id).ok_or("mission camera capital GTID absent")?;
   let p=staged.actors.placement(key,&CurrentSpatial(world)).map_err(|e|e.to_string())?;
   Choice::Capital([p.position_bits[0],p.position_bits[1]])
  }else{Choice::MissingCapital}
 }else{
  Choice::Observer(crate::load_tail_binding::gmap_extent_words(world.terrain.bound.layout.extent_i32)?)
 };
 apply(&mut world.tail.camera,&mut world.tail.mission_camera_messages,choice,pc)
}
#[cfg(test)]#[path="mission_camera_world_tests.rs"]mod tests;
