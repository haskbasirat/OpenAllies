//! Native4E15D0 Nation store/Commander filter/HQ refresh and50EC66..50ECFA
//! random Nation/Faction resolution. Every owner is the registered allocation;
//! source fields use the same completed Bank and its definition-time precision.
use crate::catalog::Catalog;
use aa_actors::{ActorId,IdentityRegistry,components::ComponentSlot,owned::Component};
use aa_mapworld::{restore::{RestoredRecord,RestoreDefinitions,NamedDefinition},special_state::SpecialShell};
use aa_numeric::Precision;
use aa_rules::{actors::ResolvedNode,native::TypedValue};
use aa_simulation::NativeRng;
type Result<T>=std::result::Result<T,String>;

fn shell(registry:&IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&SpecialShell> {
 match &registry.get(id).ok_or("Nation receiver GTID is stale")?.value {
  RestoredRecord::Kingdom(v) if v.source.serialized_id==id.raw()=>v.runtime.as_ref().and_then(|v|v.derived.as_ref()),
  _=>None,
 }.ok_or_else(||"Nation receiver is not the same actual Kingdom allocation".into())
}
fn shell_mut(registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&mut SpecialShell> {
 match registry.value_mut(id).ok_or("Nation mutable receiver GTID is stale")? {
  RestoredRecord::Kingdom(v) if v.source.serialized_id==id.raw()=>v.runtime.as_mut().and_then(|v|v.derived.as_mut()),
  _=>None,
 }.ok_or_else(||"Nation mutable receiver is not the same actual Kingdom allocation".into())
}
fn nation(s:&SpecialShell)->Result<Option<&NamedDefinition>> {
 match s.references.get(&0x234) {Some(v)=>Ok(v.as_ref()),None if s.words.get(&0x234)==Some(&0)=>Ok(None),_=>Err("actual Kingdom234 Nation owner is unbound".into())}
}
fn commander_is_null(s:&SpecialShell)->Result<bool> {
 if s.commander.is_some(){Ok(false)}else if s.words.get(&0x230)==Some(&0){Ok(true)}else{Err("actual Kingdom230 Commander owner is unbound".into())}
}
fn source_pc(catalog:&Catalog<'_>)->Result<Precision> {
 catalog.require_source_owner()?;
 let bank=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?;
 let owner=bank.combat_source()?;
 if owner.source_identities()!=catalog.actors.sources.as_slice(){return Err("Nation source owner differs from actual completed Bank".into())}
 Ok(owner.source_precision())
}
fn source<'a>(catalog:&'a Catalog<'_>,kind:&str,id:&NamedDefinition,pc:Precision)->Result<ResolvedNode<'a>> {
 let registry=catalog.actors.native_data_registry(kind).ok_or_else(||format!("actual {kind} native registry absent"))?;
 let entry=registry.find(&id.id).ok_or_else(||format!("actual {kind} canonical identity absent"))?;
 if !entry.ids.eq_ignore_ascii_case(&id.id)||!entry.reloads.is_empty(){return Err(format!("{kind} canonical identity/reloaded field owner is unbound"))}
 if let Some(index)=id.index {
  if registry.entries.get(index as usize).map(|v|v.ids.as_str())!=Some(entry.ids.as_str()){return Err(format!("{kind} canonical ordinal differs"))}
 }
 let node=catalog.actors.native_data_node(entry).map_err(|e|e.to_string())?;
 match node.field("IDS",pc).map_err(|e|e.to_string())? {
  Some(TypedValue::String{value,..}) if value.eq_ignore_ascii_case(&entry.ids)=>Ok(node),
  _=>Err(format!("{kind} native stored IDS differs")),
 }
}
fn factions(catalog:&Catalog<'_>,id:&NamedDefinition,pc:Precision)->Result<Vec<NamedDefinition>> {
 let node=source(catalog,"Nation",id,pc)?;
 let Some(TypedValue::String{value,..})=node.field("factions",pc).map_err(|e|e.to_string())? else{return Err("48C8D5 Nation factions requires actual typed string".into())};
 let mut out=Vec::new();
 //48C8DF/48C935 are the same424FA0 flags0 comma walk as this proven helper.
 for token in aa_rules::abilities::required_property_tokens(&value).map_err(|e|e.to_string())? {
  let resolved=catalog.resolve_named_definition("Faction",&token.encode_utf16().collect::<Vec<_>>(),pc)?
   .ok_or_else(||format!("48C969 unknown faction '{token}' for nation '{}'",id.id))?;
  out.try_reserve(1).map_err(|e|e.to_string())?;out.push(resolved);
 }
 Ok(out)
}
fn selectable(catalog:&Catalog<'_>,id:&NamedDefinition,pc:Precision)->Result<bool> {
 source(catalog,"Nation",id,pc)?.field("selectable",pc).map_err(|e|e.to_string())?
  .ok_or("48CB58 actual Nation selectable field absent")?.boolean().map_err(|e|e.to_string())
}
fn enabled(catalog:&Catalog<'_>,id:&NamedDefinition,pc:Precision)->Result<bool> {
 source(catalog,"Faction",id,pc)?.field("public",pc).map_err(|e|e.to_string())?
  .ok_or("48BB3E actual Faction public field absent")?.boolean().map_err(|e|e.to_string())
}
fn pick_enabled(list:Vec<NamedDefinition>,mut enabled:impl FnMut(&NamedDefinition)->Result<bool>,mut draw:impl FnMut()->u32)->Result<Option<NamedDefinition>> {
 let mut eligible=Vec::new();eligible.try_reserve_exact(list.len()).map_err(|e|e.to_string())?;
 for faction in list {if enabled(&faction)?{eligible.push(faction);}}
 Ok(match eligible.len(){0=>None,1=>Some(eligible.remove(0)),count=>Some(eligible.remove(draw()as usize%count))})
}
fn commander_faction(catalog:&Catalog<'_>,s:&SpecialShell)->Result<Option<String>> {
 let token=s.commander.as_ref().ok_or("existing Commander allocation is absent")?.definition;
 if token==0{return Ok(None)} //4730D5 null Faction succeeds without Nation list reads.
 let bank=catalog.definition_bank.try_borrow().map_err(|e|e.to_string())?;
 let entry=bank.entry(token)?;
 if entry.key.domain!="faction" {return Err("Commander+4 does not name the same Bank Faction".into())}
 Ok(Some(entry.key.name.clone()))
}
fn replace_commander(catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId,new:Option<NamedDefinition>,pc:Precision)->Result<()> {
 //4E0F00 destroys the old allocation BEFORE constructing the replacement.
 let s=shell_mut(registry,id)?;s.commander=None;s.words.insert(0x230,0);
 let input=match new {Some(v)=>catalog.resolve_commander(&v.id.encode_utf16().collect::<Vec<_>>(),pc)?,None=>None};
 let s=shell_mut(registry,id)?;
 aa_combat::commander::replace_with_maximum(&mut s.commander,input.as_ref().map(|v|(&v.definition,v.maximum_experience)),id.raw()).map_err(|e|e.to_string())?;
 if s.commander.is_some(){s.words.remove(&0x230);}Ok(())
}
fn actor(registry:&IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&aa_actors::loading::ActorShell<aa_mapworld::restore::RestoredComponent>> {
 let e=&registry.get(id).ok_or("573650 HQ actor GTID is stale")?.value;
 let base=match e {
  RestoredRecord::Actor(v) if v.source.serialized_id==id.raw()&&v.definition.native_class==3=>Some(&v.shell),
  _=>None,
 }.ok_or("573650 owner has no proved Actor108 virtual")?;
 base.actor.as_ref().ok_or_else(||"573650 actual Actor fields absent".into())
}
fn hq(registry:&IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&aa_actors::hq_state::HqState> {
 match actor(registry,id)?.components.get(ComponentSlot::Hq) {
  Some(Component::Hq(v)) if v.header.owner==id=>Ok(v),
  _=>Err("4E1619 actual ActorD8 HQ component absent/different owner".into()),
 }
}
fn hq_mut(registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId)->Result<&mut aa_actors::hq_state::HqState> {
 let e=registry.value_mut(id).ok_or("573650 mutable HQ actor GTID is stale")?;
 let base=match e {
  RestoredRecord::Actor(v) if v.source.serialized_id==id.raw()&&v.definition.native_class==3=>Some(&mut v.shell),
  _=>None,
 }.ok_or("573650 mutable owner has no proved Actor108 virtual")?;
 match base.actor.as_mut().ok_or("573650 actual mutable Actor fields absent")?.components.get_mut(ComponentSlot::Hq) {
  Some(Component::Hq(v)) if v.header.owner==id=>Ok(v),
  _=>Err("573650 actual mutable HQ component absent/different owner".into()),
 }
}
fn actor_kingdom(registry:&IdentityRegistry<RestoredRecord>,id:ActorId)->Result<Option<ActorId>> {
 let word=*actor(registry,id)?.words.get(&0xe8).ok_or("4D1E80 actual ActorE8 owner unbound")?;
 Ok((word!=0).then(||ActorId::from_raw(word)))
}
fn allocate_badge(catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId,rng:&mut NativeRng)->Result<i32> {
 let n=nation(shell(registry,id)?)?.ok_or("4E2643 actual Kingdom234 is null")?;
 let pc=source_pc(catalog)?;
 source(catalog,"Nation",n,pc)?; //same canonical definition, no authored fallback/reload.
 let count=catalog.actors.native_nation_badges(&n.id,pc).and_then(|v|v.count()).map_err(|e|e.to_string())?;
 if (count as i32)<=0{return Ok(-1)}
 let drawn=crate::configure_world::random_word(catalog,rng)?%count;
 let s=shell_mut(registry,id)?;
 let result=choose_badge(count,&s.nation_badge_indices,drawn);
 if result!=-1 {s.nation_badge_indices.try_reserve(1).map_err(|e|e.to_string())?;s.nation_badge_indices.insert(0,result as u32);s.words.remove(&0x36c);s.words.remove(&0x370);}
 Ok(result)
}
pub(crate) fn choose_badge(count:u32,taken:&[u32],drawn:u32)->i32 {
 if taken.contains(&drawn){(0..count).find(|v|!taken.contains(v)).map_or(-1,|v|v as i32)}else{drawn as i32}
}
fn refresh_hq(catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId,rng:&mut NativeRng)->Result<()> {
 hq(registry,id)?;
 let Some(owner)=actor_kingdom(registry,id)? else{return Ok(())};
 if hq(registry,id)?.badge_index!=-1 {
  if let Some(current)=actor_kingdom(registry,id)? {
   let index=hq(registry,id)?.badge_index as u32;
   let s=shell_mut(registry,current)?;
   if let Some(at)=s.nation_badge_indices.iter().position(|&v|v==index){s.nation_badge_indices.remove(at);}
   hq_mut(registry,id)?.badge_index=-1;
  }
  if hq(registry,id)?.badge_index!=-1{return Ok(())}
 }
 if nation(shell(registry,owner)?)?.is_some(){let value=allocate_badge(catalog,registry,owner,rng)?;hq_mut(registry,id)?.badge_index=value;}
 Ok(())
}
///4E15D0: Nation store precedes the existing-Commander compatibility test;
///the captured signed HQ count precedes live per-index ActorD8 rereads.
pub(crate) fn set_nation(catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId,new:Option<NamedDefinition>,rng:&mut NativeRng)->Result<()> {
 let s=shell_mut(registry,id)?;s.words.remove(&0x234);s.references.insert(0x234,new);
 if let Some(n)=nation(shell(registry,id)?)? && !commander_is_null(shell(registry,id)?)? {
  if let Some(faction)=commander_faction(catalog,shell(registry,id)?)? {
   let pc=source_pc(catalog)?;
   if !factions(catalog,n,pc)?.iter().any(|v|v.id.eq_ignore_ascii_case(&faction)){replace_commander(catalog,registry,id,None,pc)?;}
  }
 }
 let count=shell(registry,id)?.vectors.get(&0x2dc).ok_or("4E1603 actual HQ2DC list unbound")?.values.len();
 if count>i32::MAX as usize{return Err("4E1603 HQ vector exceeds native signed domain".into())}
 for index in 0..count {
  let raw=*shell(registry,id)?.vectors.get(&0x2dc).ok_or("4E1610 current HQ list unbound")?.values.get(index).ok_or("4E1616 current HQ index absent")?;
  refresh_hq(catalog,registry,ActorId::from_raw(raw),rng)?;
 }
 Ok(())
}
///50EC66..50ECFA. A zero/one eligible Faction still logs seedA thenS at
///50ECE5, but only474C0F with >1 consumes RNG. No retry cap is invented.
pub(crate) fn resolve_random(catalog:&Catalog<'_>,registry:&mut IdentityRegistry<RestoredRecord>,id:ActorId,rng:&mut NativeRng)->Result<()> {
 if nation(shell(registry,id)?)?.is_none() {
  let entries=&catalog.actors.native_data_registry("Nation").ok_or("50EC73 actual GameData84 Nation registry absent")?.entries;
  if entries.len()>i32::MAX as usize{return Err("GameData84 exceeds native signed domain".into())}
  if !entries.is_empty(){loop {
   let word=crate::configure_world::random_word(catalog,rng)?;
   let chosen=&entries[word as usize%entries.len()];
   set_nation(catalog,registry,id,Some(NamedDefinition{id:chosen.ids.clone(),index:None}),rng)?;
   let pc=source_pc(catalog)?;
   if selectable(catalog,nation(shell(registry,id)?)?.ok_or("50ECC1 stored Nation is NULL")?,pc)?{break}
  }}
 }
 if !commander_is_null(shell(registry,id)?)?{return Ok(())}
 let Some(n)=nation(shell(registry,id)?)? else{return Ok(())};
 let pc=source_pc(catalog)?;
 let list=factions(catalog,n,pc)?;
 if list.is_empty(){return Ok(())}
 crate::configure_world::log_random_seed(catalog,rng)?;
 let chosen=pick_enabled(list,|faction|enabled(catalog,faction,pc),||rng.next_u32())?;
 replace_commander(catalog,registry,id,chosen,pc)
}
#[cfg(test)]#[path="configure_world_nations_tests.rs"]mod tests;
