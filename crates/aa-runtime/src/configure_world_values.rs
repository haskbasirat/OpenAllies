//! Concrete50E8B0 field/list prefix over the actual after-KINGDOMS owners.
//! Source parsing is immutable and uses the Bank's source precision; runtime
//! draws consume the same GWorld RNG. Unsupported reached children fail in
//! place, preserving Team/name/color/resource writes already performed.
use crate::{catalog::Catalog,load_kingdom_init_world::KingdomRegistryState,
    live_registration,resource_creator::Creator};
use aa_actors::{ActorId,IdentityRegistry};
use aa_mapworld::{restore::{RestoredRecord,RestoreDefinitions,NamedDefinition,NativeMessage},special_state::SpecialShell};
use aa_numeric::Precision;
use aa_simulation::NativeRng;
type Result<T>=std::result::Result<T,String>;
#[cfg(test)]#[path="configure_world_values_tests.rs"]mod tests;
fn prefix(v:&[u16])->&[u16] {&v[..v.iter().position(|&v|v==0).unwrap_or(v.len())]}
fn special(registry:&IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&SpecialShell> {
 let entry=registry.get(id).ok_or("ConfigureGWorld stale registered identity")?;
 match &entry.value {
  RestoredRecord::Kingdom(v) if v.source.serialized_id==id.raw()=>v.runtime.as_ref().and_then(|v|v.derived.as_ref()),
  RestoredRecord::Team(v) if v.source.serialized_id==id.raw()=>v.runtime.as_ref().and_then(|v|v.derived.as_ref()),
  _=>None,
 }.ok_or_else(||"ConfigureGWorld actual special allocation is absent or has a different identity".into())
}
fn special_mut(registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&mut SpecialShell> {
 let entry=registry.value_mut(id).ok_or("ConfigureGWorld stale mutable registered identity")?;
 match entry {
  RestoredRecord::Kingdom(v) if v.source.serialized_id==id.raw()=>v.runtime.as_mut().and_then(|v|v.derived.as_mut()),
  RestoredRecord::Team(v) if v.source.serialized_id==id.raw()=>v.runtime.as_mut().and_then(|v|v.derived.as_mut()),
  _=>None,
 }.ok_or_else(||"ConfigureGWorld actual mutable special allocation is absent or has a different identity".into())
}
fn name(registry:&IdentityRegistry<RestoredRecord>,id:ActorId,kingdom:bool)->Result<&[u16]> {
 let entry=registry.get(id).ok_or("ConfigureGWorld name GTID is absent")?;
 let value=match (&entry.value,kingdom) {
  (RestoredRecord::Kingdom(v),true)=>v.runtime.as_ref(),
  (RestoredRecord::Team(v),false)=>v.runtime.as_ref(),_=>None,
 }.ok_or("ConfigureGWorld name receiver has the wrong native class")?;
 Ok(&value.base.thing.name_utf16)
}
fn word(s:&SpecialShell,offset:u32)->Result<u32> {s.words.get(&offset).copied().ok_or_else(||format!("ConfigureGWorld actual+{offset:X} word unbound"))}
fn reference(s:&SpecialShell,offset:u32)->Result<Option<&NamedDefinition>> {
 match s.references.get(&offset) {Some(v)=>Ok(v.as_ref()),None if s.words.get(&offset)==Some(&0)=>Ok(None),_=>Err(format!("ConfigureGWorld actual+{offset:X} canonical reference unbound"))}
}
fn team(s:&SpecialShell)->Result<Option<ActorId>> {
 match s.identity_references.get(&0x1ec) {Some(v)=>Ok(*v),None if s.words.get(&0x1ec)==Some(&0)=>Ok(None),_=>Err("ConfigureGWorld actual Team reference unbound".into())}
}
fn store_reference(s:&mut SpecialShell,offset:u32,value:Option<NamedDefinition>) {
 s.words.remove(&offset);s.references.insert(offset,value);
}
fn byte(s:&mut SpecialShell,offset:u32,value:u8)->Result<()> {
 if let Some(w)=s.words.get_mut(&(offset&!3)) {let shift=(offset&3)*8;*w=(*w&!(255<<shift))|((value as u32)<<shift);return Ok(())}
 *s.bytes.get_mut(&offset).ok_or_else(||format!("ConfigureGWorld actual+{offset:X} byte unbound"))?=value;Ok(())
}
fn increment_team(s:&mut SpecialShell)->Result<()> {
 let v=s.words.get_mut(&0x68).ok_or("4E3A50 actual Team+68 absent")?;*v=v.wrapping_add(1);Ok(())
}
pub(super) fn remove_from_team(s:&mut SpecialShell,id:ActorId)->Result<()> {
 let list=s.team_members.as_mut().ok_or("4F1430 actual Team member list unbound")?;
 if let Some(i)=list.iter().position(|&v|v==id){list.remove(i);}
 increment_team(s)
}
pub(super) fn add_to_team(s:&mut SpecialShell,id:ActorId)->Result<()> {
 let list=s.team_members.as_mut().ok_or("4E3A50 actual new Team member list unbound")?;
 list.try_reserve(1).map_err(|e|e.to_string())?;list.insert(0,id);increment_team(s)
}
pub(super) fn set_team(registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId,new:Option<ActorId>)->Result<()> {
 if let Some(old)=team(special(registry,id)?)? {
  let s=special_mut(registry,old)?;
  //The original increments its mutation counter even if removal found none.
  remove_from_team(s,id)?;
 }
 let s=special_mut(registry,id)?;s.words.remove(&0x1ec);s.identity_references.insert(0x1ec,new);
 if let Some(new)=new {
  add_to_team(special_mut(registry,new)?,id)?;
 }
 Ok(())
}
fn find_team(registry:&IdentityRegistry<RestoredRecord>,teams:&live_registration::State,needle:&[u16])->Result<Option<ActorId>> {
 //Both4F49C2 and50DC20 call96CB83 (_wcscmp), not the case-folding CRT.
 let target=prefix(needle);
 for &id in teams.teams().iter().flatten() {if prefix(name(registry,id,false)?)==target{return Ok(Some(id))}}
 Ok(None)
}
fn named(catalog:&Catalog<'_>,manager:&str,ids:&[u16],pc:Precision)->Result<Option<NamedDefinition>> {
 catalog.resolve_named_definition(manager,prefix(ids),pc)
}
fn color_index(catalog:&Catalog<'_>,color:&NamedDefinition)->Result<u32> {
 let entries=&catalog.actors.native_data_registry("KingdomColor").ok_or("actual Color registry absent")?.entries;
 let mut found=entries.iter().enumerate().filter(|(_,e)|e.ids.eq_ignore_ascii_case(&color.id));
 let (index,e)=found.next().ok_or("actual Color identity absent")?;
 if found.next().is_some()||!e.reloads.is_empty(){return Err("Color ordinal needs reloaded/duplicate source storage".into())}
 let index=u32::try_from(index).map_err(|e|e.to_string())?;
 if color.index.is_some_and(|v|v!=index){return Err("Color canonical ordinal differs".into())}Ok(index)
}
fn set_color(catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,kingdoms:&mut KingdomRegistryState,id:ActorId,new:Option<NamedDefinition>)->Result<()> {
 let old=reference(special(registry,id)?,0x1e8)?.map(|v|color_index(catalog,v)).transpose()?;
 let next=new.as_ref().map(|v|color_index(catalog,v)).transpose()?;
 kingdoms.transition_color(old,next)?;
 store_reference(special_mut(registry,id)?,0x1e8,new);Ok(())
}
fn commander_null(s:&SpecialShell)->Result<bool> {
 if s.commander.is_some(){Ok(false)}else if s.words.get(&0x230)==Some(&0){Ok(true)}else{Err("ConfigureGWorld actual Commander pointer/owner unbound".into())}
}
fn set_commander(catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId,ids:&[u16],pc:Precision)->Result<()> {
 let definition=if let Some(faction)=named(catalog,"Faction",ids,pc)? {
  catalog.resolve_commander(&faction.id.encode_utf16().collect::<Vec<_>>(),pc)?
 }else{None};
 let shell=special_mut(registry,id)?;
 aa_combat::commander::replace_with_maximum(&mut shell.commander,definition.as_ref().map(|v|(&v.definition,v.maximum_experience)),id.raw()).map_err(|e|e.to_string())?;
 //Positive Commander has one authoritative owner, not a made-up raw pointer.
 if shell.commander.is_some(){shell.words.remove(&0x230);}else{shell.words.insert(0x230,0);}Ok(())
}
fn diagnostic(messages:&mut Vec<NativeMessage>,line:u32,function:u32,id:ActorId,text:&str)->Result<()> {
 messages.try_reserve(1).map_err(|e|e.to_string())?;
 messages.push(NativeMessage{source_ordinal:0,function,line,text:format!("ConfigureGWorld {}: {text}",id.raw())});Ok(())
}
pub(super) fn parameter_tail(shell:&mut SpecialShell,p:&aa_mapworld::header::KingdomParameter,
 handicap:Option<NamedDefinition>,mut grant:impl FnMut(&mut SpecialShell,&[u16])->Result<()>)->Result<()> {
 store_reference(shell,0x244,handicap);
 *shell.strings_utf16.get_mut(&0x240).ok_or("actual Kingdom240 CString unbound")?=p.string_2c.clone();
 byte(shell,0x23d,p.byte_32)?;byte(shell,0x23b,1)?;byte(shell,0x23c,p.byte_28)?;
 for definition in &p.definitions {grant(shell,definition)?;}
 *shell.strings_utf16.get_mut(&0x248).ok_or("actual Kingdom248 merge CString unbound")?=p.string_58.clone();
 shell.vectors.get_mut(&0x258).ok_or("actual Kingdom258 starting array unbound")?
  .replace_values(&p.resources.iter().map(|v|v.1).collect::<Vec<_>>()).map_err(|e|e.to_string())?;
 Ok(())
}
pub(super) fn append_start_property(shell:&mut SpecialShell,token:u32)->Result<()> {
 let array=shell.vectors.get_mut(&0x24c).ok_or("4E6640 actual Kingdom24C start-Property array unbound")?;
 if !array.values.contains(&token){array.push(token).map_err(|e|e.to_string())?;}Ok(())
}
fn property_token(catalog:&Catalog<'_>,ids:&[u16],pc:Precision)->Result<u32> {
 if ids.is_empty(){return Ok(0)}
 let p=named(catalog,"Property",ids,pc)?.ok_or("nonempty start-Property IDS requires its actual native unknown-reference diagnostic")?;
 let bank=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?;
 let token=bank.token("Property",&p.id).ok_or("ConfigureGWorld current Property is not owned by the actual completed source Bank")?;
 let entry=bank.entry(token)?;
 if entry.key.domain!="property"||!entry.key.name.eq_ignore_ascii_case(&p.id){return Err("ConfigureGWorld current Property token canonical identity differs".into())}
 Ok(token)
}
pub(super) fn select_color(pool:&mut Vec<u32>,count:u32,draw:impl FnOnce()->Result<u32>)->Result<u32> {
 if pool.is_empty(){pool.extend(0..count);}
 let draw=draw()?;
 if pool.is_empty(){return Err("50E370 consumed real RNG then reached native zero Color divisor".into())}
 Ok(pool.remove(draw as usize%pool.len()))
}
///Actual full first pass, followed by native color selection on the same RNG.
///Final successful completion still requires random-Nation/Faction sources,
///queued destruction and the two live relation-mask passes when reached.
pub(super) fn run(catalog:&Catalog<'_>,creator:&Creator,registry:&mut IdentityRegistry<RestoredRecord>,kingdoms:&mut KingdomRegistryState,teams:&mut live_registration::State,rng:&mut NativeRng,messages:&mut Vec<NativeMessage>,live_pc:Precision,removal:&mut impl super::KingdomRemoval)->Result<()> {
 let pc=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?.combat_source()?.source_precision();
 let colors=catalog.actors.native_data_registry("KingdomColor").ok_or("50E1A0 actual Color registry absent")?;
 let mut pool:Vec<u32>=(0..u32::try_from(colors.entries.len()).map_err(|e|e.to_string())?).collect();
 let mut pending_destroy=Vec::new();let mut i=0;
 while i<kingdoms.majors().len() {
  let Some(id)=kingdoms.majors()[i]else{i+=1;continue};
  let native_name=name(registry,id,true)?;
  diagnostic(messages,0x1f6,0x404f60,id,&String::from_utf16_lossy(native_name))?;
  let parameter=creator.parameters().kingdoms.iter().enumerate().find(|(_,v)|prefix(&v.id)==prefix(native_name));
  let Some((index,p))=parameter else {
   diagnostic(messages,0x1fd,0x404ed0,id,"Destroying kingdom because cannot find KingdomInfo in WorldParams")?;
   pending_destroy.push(id);i+=1;continue
  };
  if p.byte_31!=0 {pending_destroy.push(id);i+=1;continue}
  let new_team=find_team(registry,teams,&p.string_14)?;
  set_team(registry,id,new_team)?;
  let text=creator.labels().kingdom_pair(creator.parameters(),index)?.text().to_vec();
  let shell=special_mut(registry,id)?;
  let key=shell.strings_utf16.get(&0x194).ok_or("50E0D0 actual Kingdom194 key unbound")?.clone();
  let dest=shell.strings_utf16.get_mut(&0x198).ok_or("50E0D0 actual Kingdom198 text unbound")?;
  //The first pair store is independent of whether its table can be loaned.
  dest.clear();dest.extend_from_slice(&text);
  catalog.with_current_text_mut(|table|crate::live_map_strings::set_existing_pair(table,&key,dest,&text))?;
  if reference(special(registry,id)?,0x234)?.is_none() {let v=named(catalog,"Nation",&p.nation,pc)?;crate::configure_world_nations::set_nation(catalog,registry,id,v,rng)?;}
  if commander_null(special(registry,id)?)? {set_commander(catalog,registry,id,&p.faction,pc)?;}
  if reference(special(registry,id)?,0x1e8)?.is_none() {let v=named(catalog,"KingdomColor",&p.color,pc)?;set_color(catalog,registry,kingdoms,id,v)?;}
  if let Some(color)=reference(special(registry,id)?,0x1e8)? {
   let index=color_index(catalog,color)?;if let Some(at)=pool.iter().position(|&v|v==index){pool.remove(at);}
  }
  let handicap=named(catalog,"Handicap",&p.handicap,pc)?;
  let resource_registry=catalog.actors.native_resource_registry(pc).map_err(|e|e.to_string())?;
  if p.resources.len()!=resource_registry.ids.len()||p.resources.iter().zip(&resource_registry.ids).any(|((a,_),b)|*a!=b.encode_utf16().collect::<Vec<_>>()) {
   return Err("ConfigureGWorld parameter resources differ from the actual native source array".into())
  }
  parameter_tail(special_mut(registry,id)?,p,handicap,|shell,ids|append_start_property(shell,property_token(catalog,ids,pc)?))?;
  i+=1;
 }
 i=0;
 while i<kingdoms.majors().len() {
  let Some(id)=kingdoms.majors()[i]else{i+=1;continue};
  diagnostic(messages,0x245,0x404f60,id,"resolving random settings")?;
  if reference(special(registry,id)?,0x1e8)?.is_none() {
   let ordinal=select_color(&mut pool,u32::try_from(colors.entries.len()).map_err(|e|e.to_string())?,||super::random_word(catalog,rng))?;
   let color=&colors.entries[ordinal as usize];
   set_color(catalog,registry,kingdoms,id,Some(NamedDefinition{id:color.ids.clone(),index:Some(ordinal)}))?;
  }
  crate::configure_world_nations::resolve_random(catalog,registry,id,rng)?;
  i+=1;
 }
 super::drain_pending(&pending_destroy,|id|removal.remove(catalog,registry,kingdoms,teams,id,live_pc))?;
 for id in kingdoms.majors().iter().flatten().copied() {crate::configure_world_relations::refresh(registry,kingdoms,id)?;}
 Ok(())
}
