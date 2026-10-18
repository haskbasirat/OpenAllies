//! Immutable source reads reached by4E6DB0, using the SAME moved Bank.
use crate::{catalog::Catalog,definitions::Bank};
use aa_mapworld::restore::NamedDefinition;
use aa_rules::{actors::ResolvedNode,native::{TypedValue,layout_data::{PropertyRef,RootPosition}}};
type Result<T>=std::result::Result<T,String>;
fn precision(catalog:&Catalog<'_>,bank:&Bank)->Result<aa_numeric::Precision> {
 catalog.require_source_owner()?;let source=bank.combat_source()?;source.completed_means()?;
 if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("Kingdom mission source universe differs from moved Bank".into())}
 Ok(source.source_precision())
}
fn faction<'a>(catalog:&'a Catalog<'_>,bank:&Bank,token:u32)->Result<ResolvedNode<'a>> {
 let pc=precision(catalog,bank)?;let key=bank.entry(token)?;
 if key.key.domain!="faction"{return Err("Commander definition is not the actual Bank Faction".into())}
 let registry=catalog.actors.native_data_registry("Faction").ok_or("Faction manager absent")?;
 let entry=registry.find(&key.key.name).ok_or("Commander canonical Faction absent")?;
 if !entry.ids.eq_ignore_ascii_case(&key.key.name)||!entry.reloads.is_empty(){return Err("Commander Faction identity/reloaded fields are unbound".into())}
 let node=catalog.actors.native_data_node(entry).map_err(|e|e.to_string())?;
 match node.field("IDS",pc).map_err(|e|e.to_string())? {
  Some(TypedValue::String{value,..})if value.eq_ignore_ascii_case(&key.key.name)=>Ok(node),
  _=>Err("Commander Faction stored IDS differs from canonical Bank identity".into()),
 }
}
///48BD1C..48BDE3 reads the typed property string, resolves Property names at
///this source load position, and prepends each node. Its resulting list is
///therefore reverse authored order, including duplicates.
pub(crate) fn properties(catalog:&Catalog<'_>,bank:&Bank,token:u32)->Result<Vec<u32>> {
 let pc=precision(catalog,bank)?;let node=faction(catalog,bank,token)?;
 let Some(TypedValue::String{value,..})=node.field("properties",pc).map_err(|e|e.to_string())? else{return Err("Faction properties typed string absent".into())};
 let source=node.sources().into_iter().next().ok_or("Faction source position absent")?;
 let at=RootPosition{file:catalog.actors.sources.iter().position(|v|v.path.eq_ignore_ascii_case(&source.path)).ok_or("Faction source is not a retained file")?,offset:source.span.start};
 let data=catalog.layout_data(pc)?;let mut result=Vec::new();
 for name in aa_rules::abilities::required_property_tokens(&value).map_err(|e|e.to_string())? {
  let Some(PropertyRef::Registered(index))=data.find_property(&name,Some(at))else{return Err(format!("48BDB8 unknown Property '{name}' at Faction load boundary"))};
  let property=data.properties.get(index as usize).ok_or("Faction Property canonical ordinal absent")?;
  let token=bank.token("Property",&property.ids).ok_or("Faction Property lacks the same Bank canonical token")?;
  let key=bank.entry(token)?;if key.key.domain!="property"||!key.key.name.eq_ignore_ascii_case(&property.ids){return Err("Faction Property token differs from canonical source".into())}
  result.try_reserve(1).map_err(|e|e.to_string())?;result.insert(0,token);
 }
 Ok(result)
}
///4730D0 null Faction succeeds immediately. A positive Faction is compared by
///canonical identity against the actual Nation's loaded pointer vector.
pub(crate) fn allowed(catalog:&Catalog<'_>,bank:&Bank,nation:Option<&NamedDefinition>,faction_token:Option<u32>)->Result<bool> {
 let Some(token)=faction_token else{return Ok(true)};
 let Some(nation)=nation else{return Ok(true)};
 let pc=precision(catalog,bank)?;
 let registry=catalog.actors.native_data_registry("Nation").ok_or("Nation manager absent")?;
 let entry=registry.find(&nation.id).ok_or("Kingdom Nation canonical identity absent")?;
 if !entry.ids.eq_ignore_ascii_case(&nation.id)||!entry.reloads.is_empty(){return Err("Nation identity/reloaded fields are unbound".into())}
 let node=catalog.actors.native_data_node(entry).map_err(|e|e.to_string())?;
 let Some(TypedValue::String{value,..})=node.field("factions",pc).map_err(|e|e.to_string())? else{return Err("Nation factions typed string absent".into())};
 let names=aa_rules::abilities::required_property_tokens(&value).map_err(|e|e.to_string())?;
 if names.is_empty(){return Ok(true)}
 let key=bank.entry(token)?;if key.key.domain!="faction"{return Err("current Commander has a foreign Faction token".into())}
 let factions=catalog.actors.native_data_registry("Faction").ok_or("Faction manager absent")?;
 for name in names {
  let f=factions.find(&name).ok_or_else(||format!("Nation unknown Faction '{name}' requires actual source failure"))?;
  if f.ids.eq_ignore_ascii_case(&key.key.name){return Ok(true)}
 }
 Ok(false)
}
