//! Actual ordered StartMission tail: CityVictoryRatio, World48, selected Sun.
//! City counts borrow the same typed Kingdom30C list mutated by4E3C70;
//! no second list is reconstructed by scanning actor ownership.
use crate::{actor_world::{ActorStore,Source,SpecialSource},catalog::Catalog,restoration::Staged};
use aa_actors::ActorId;
use aa_mapworld::special_state::SpecialShell;
use aa_numeric::{Binary,Precision};
type Result<T>=std::result::Result<T,String>;
fn ratio(current:i32,maximum:i32,available:i32,pc:Precision)->Result<Option<i32>> {
 if current<=0||available<=0{return Ok(None)}
 let percent=Binary::from_i32(maximum).div(&Binary::from_i32(available),pc).map_err(|e|e.to_string())?
  .mul(&Binary::from_i32(100),pc).map_err(|e|e.to_string())?;
 //4FDF38 stores a binary32 temporary without popping ST0. The comparison
 //uses the retained x87 value; the later _ftol reads the stored float again.
 let stored=percent.store_f32().map_err(|e|e.to_string())?;
 if percent.compare(&Binary::from_i32(current))==std::cmp::Ordering::Less{return Ok(None)}
 let integer=Binary::from_f32_bits(stored).map_err(|e|e.to_string())?.truncate_i32()
  .map_err(|_|"CityVictoryRatio requires native indefinite integer-conversion handling outside signed32 domain")?;
 Ok(Some(integer.wrapping_add(1).min(100)))
}
fn shell(actors:&ActorStore,id:ActorId,kingdom:bool)->Result<&SpecialShell> {
 let key=actors.key(id).ok_or("mission City current GTID absent")?;
 let entity=actors.entity(key).map_err(|e|e.to_string())?;
 let Source::Special(v)=&entity.source else{return Err("mission City member is not actual special allocation".into())};
 match (&v.source,kingdom) {
  (SpecialSource::Kingdom(saved),true)if saved.serialized_id==id.raw()=>{},
  (SpecialSource::Team(saved),false)if saved.serialized_id==id.raw()=>{},
  _=>return Err("mission City current special class/identity differs".into()),
 }
 v.derived.as_ref().ok_or_else(||"mission City derived owner absent".into())
}
fn count_cities(actors:&ActorStore,id:ActorId)->Result<i32> {
 //4E3AB0 increments one native signed DWORD per actual node. The same
 //typed head-first list replaces addresses; no actor ownership scan occurs.
 let lists=shell(actors,id,true)?.actor_lists.as_ref().ok_or("4E3AB0 actual Kingdom30C list owner unbound")?;
 Ok(crate::kingdom_ownership_track::count(&lists.city_ownership))
}
fn team(kingdom:&SpecialShell)->Result<Option<ActorId>> {
 match kingdom.identity_references.get(&0x1ec){Some(v)=>Ok(*v),None if kingdom.words.get(&0x1ec)==Some(&0)=>Ok(None),_=>Err("mission City actual Team pointer unbound".into())}
}
fn maximum_cities(staged:&Staged)->Result<i32> {
 let world=staged.load_world.as_ref().ok_or("mission City actual World absent")?;let mut maximum=0;
 for id in world.kingdoms.majors().iter().flatten().copied() {
  let value=if let Some(id)=team(shell(&staged.actors,id,true)?)? {
   let list=shell(&staged.actors,id,false)?.team_members.as_ref().ok_or("mission City actual Team44 list unbound")?;
   let mut n=0i32;for &id in list {if !id.is_null(){n=n.wrapping_add(count_cities(&staged.actors,id)?);}}n
  }else{count_cities(&staged.actors,id)?};
  if maximum<value {maximum=value;}
 }
 Ok(maximum)
}
fn instant_colors(sun:&mut crate::sun_state::State,colors:[[u32;3];3]) {
 sun.target_colors=colors;sun.color_duration=0;sun.current_colors=colors;
}
pub(crate) fn run(staged:&mut Staged,catalog:&Catalog<'_>,pc:Precision)->Result<()> {
 let world=staged.load_world.as_ref().ok_or("mission finalization actual World absent")?;
 world.variables.validate_in_bank(catalog,&staged.definitions)?;
 world.world_rules.validate_in_bank(catalog,&staged.definitions)?;
 let current=world.variables.city_victory_ratio();
 if current>0 {
  let available=i32::try_from(world.registration.available_cities().len()).map_err(|e|e.to_string())?;
  if available>0 {
   let maximum=maximum_cities(staged)?;
   if let Some(next)=ratio(current,maximum,available,pc)? {
    let world=staged.load_world.as_mut().unwrap();
    world.tail.mission_city_clamps.push((current,maximum,available));
    //The retained current Var has proved flags0, so4B29B0 succeeds and no
    //range branch runs.511B20 updates the SAME Rules before the SVar store.
    world.world_rules.assign_word("CityVictoryRatio",1,next as u32)?;
    world.variables.assign_city_victory_ratio(next);
   }
  }
 }
 let world=staged.load_world.as_mut().unwrap();
 world.mission_fields.mark_started()?; //4FDFF1 precedes EVERY selected-Sun read.
 let sun=world.world_rules.records().find(|v|v.ids().eq_ignore_ascii_case("Sun"));
 let Some(rule)=sun else{return Ok(())};
 if rule.value()==rule.default_value(){return Ok(())}
 let id=rule.enum_ids(rule.value())?;
 let registry=catalog.actors.native_data_registry("Sun").ok_or("actual Sun source registry absent")?;
 let Some(entry)=registry.find(id) else {
  world.tail.mission_sun_warnings.push(format!("World rule specified sun '{id}' not found - using default"));return Ok(())
 };
 if !entry.reloads.is_empty(){return Err("selected Sun requires its reloaded field storage".into())}
 let source=staged.definitions.combat_source()?;
 let node=catalog.actors.native_data_node(entry).map_err(|e|e.to_string())?;
 let mut colors=[[0;3];3];
 for (i,field)in ["diffuse","ambient","specular"].iter().enumerate() {
  let Some(aa_rules::native::TypedValue::Point{bits,..})=node.field(field,source.source_precision()).map_err(|e|e.to_string())?else{return Err(format!("4B5E10 selected Sun {field} point3 field absent"))};
  colors[i]=bits.try_into().map_err(|_|"selected Sun point3 width differs")?;
 }
 instant_colors(&mut world.sun,colors);Ok(())
}
#[cfg(test)]#[path="mission_finalize_world_tests.rs"]mod tests;
