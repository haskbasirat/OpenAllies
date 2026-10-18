//! The sole current Camera AVar7..23 cells. Registrations are the executed
//! 401BD0 prefix; the v14 402446 gameplay-zoom patch is deliberately retained.
//! This is a disjoint cell owner, not a second complete Settings manager.
use crate::{catalog::Catalog, definitions::Bank, combat_value_source_context::{self, SourceFpContext}, load_view_variables::NativeOverrides};
use aa_numeric::Precision;
use aa_rules::{resources::{Provenance,SourceIdentity},sources::V14Sources,tgi::{Document,Item,Limits},native::{index::native_enumerate,scalar_literal}};
type Result<T> = std::result::Result<T,String>;
pub const NAMES:[&str;17]=["CameraNearPlane","CameraFarPlane","CameraZoomMin","CameraZoomMax","CameraZoomDefault","CameraZoomMetaGameMin","CameraZoomMetaGameMax","CameraZoomMetaGameDefault","CameraZoomGameplayMin","CameraZoomGameplayMax","CameraZoomGameplayDefault","CameraYaw","CameraMetaGameYaw","CameraPitch","CameraPitchMin","CameraHeight","CameraFOV"];
const WORDS:[u32;17]=[0x3f800000,0x44000000,0x3ecccccd,0x40200000,0x40200000,0x3f000000,0x3f99999a,0x3f99999a,0x3f333333,0x40066666,0x3f800000,0xc2340000,0,0x42200000,0x41200000,0x42c80000,0x41700000];
const FLAGS:[u32;17]=[0,0,0,0,32,0,0,32,2,2,34,2,2,34,34,34,34];
fn bounds(i:usize)->[u32;2]{match i{4=>[0x3ecccccd,0x40200000],7=>[0x3f000000,0x3f99999a],10=>[0x3f333333,0x40066666],13=>[0x41200000,0x42a00000],14=>[0x40a00000,0x42a00000],15=>[0x41200000,0x43c80000],16=>[0x40a00000,0x43340000],_=>[0xff7fffff,0x7f7fffff]}}
///Move-only current allocation. There is no Default/Clone/raw-value constructor.
pub struct State{words:[u32;17],sources:Vec<SourceIdentity>,context:SourceFpContext,files:Vec<(String,String)>,assignments:Vec<Provenance>,skipped_hardcoded:Vec<Provenance>,declarations:Vec<(usize,crate::settings::Declaration)>,source_loaded:bool}
pub struct Failure{pub state:Option<State>,pub error:String}
impl std::fmt::Debug for Failure{fn fmt(&self,f:&mut std::fmt::Formatter<'_>)->std::fmt::Result{f.debug_struct("CameraVariableFailure").field("owner_retained",&self.state.is_some()).field("error",&self.error).finish()}}
impl State{
 pub fn from_fresh_sources_retained(catalog:&Catalog<'_>,bank:&Bank,files:&mut V14Sources,scope:NativeOverrides)->std::result::Result<Self,Failure>{
  let mut state=(||->Result<Self>{catalog.require_source_owner()?;if !matches!(scope,NativeOverrides::ControlledNone){return Err("camera current profile/console variable owner is unbound".into())}let source=bank.combat_source()?;source.completed_means()?;if source.source_identities()!=catalog.actors.sources.as_slice(){return Err("camera AVar source Bank differs".into())}Ok(Self{words:WORDS,sources:catalog.actors.sources.clone(),context:source.context().clone(),files:Vec::new(),assignments:Vec::new(),skipped_hardcoded:Vec::new(),declarations:Vec::new(),source_loaded:false})})().map_err(|error|Failure{state:None,error})?;
  match state.load(catalog,bank,files){Ok(())=>Ok(state),Err(error)=>Err(Failure{state:Some(state),error})}
 }
 fn load(&mut self,catalog:&Catalog<'_>,bank:&Bank,files:&mut V14Sources)->Result<()>{
  let names=files.catalog().into_iter().map(|e|e.logical_path).collect::<Vec<_>>();let ordered=native_enumerate("AVars*.tgi",&names);
  if !ordered.iter().any(|p|p.eq_ignore_ascii_case("AVars.tgi")){return Err("camera actual root AVars source absent".into())}
  for path in ordered{let file=files.read(&path).map_err(|e|e.to_string())?;if aa_rules::sources::sha256(&file.bytes)!=file.sha256{return Err("camera AVar source hash differs".into())}self.files.push((file.logical_path.clone(),file.sha256.clone()));
   let doc=Document::parse_native(file.bytes,Limits::default()).map_err(|e|e.to_string())?;
   for item in &doc.items{let Item::Node(node)=item else{continue};if !doc.header_name(&node.header).is_some_and(|n|n.eq_ignore_ascii_case("Vars")){continue}if node.header.tokens.len()!=1{return Err("camera inherited Vars source is unbound".into())}
    // The supported source pass has at most one declaration per cell per file.
    // Successful distinct-cell stores commute. This does not claim native
    // hash traversal/error-prefix equivalence for a malformed source file.
    let mut seen=[false;17];for item in &node.items{let Item::Property(p)=item else{continue};let Some(name)=doc.property_name(p)else{continue};let Some(i)=NAMES.iter().position(|n|n.eq_ignore_ascii_case(name))else{continue};if seen[i]{return Err("camera duplicate source declaration needs native field-table binding".into())}seen[i]=true;
     let kind=if p.left.len()==2{doc.text(p.left[0].span)}else{None};
     if !kind.is_some_and(|k|k.eq_ignore_ascii_case("float")){return Err("camera source assignment requires actual Float declaration".into())}
     let raw=doc.property_value(p).ok_or("camera source scalar absent")?.trim();let source=Provenance{path:file.logical_path.clone(),sha256:file.sha256.clone(),span:p.span};
     let position=self.declarations.iter().filter(|(j,_)|*j==i).count();let row=catalog.settings.as_ref().ok_or("camera Settings owner absent")?.app_variables.get(&NAMES[i].to_ascii_lowercase()).and_then(|v|v.get(position)).ok_or("camera public source declaration absent")?;
     if row.source!=source||row.raw.trim()!=raw||!row.field_type.as_deref().is_some_and(|k|k.eq_ignore_ascii_case("float")){return Err("camera public/private source span/type/raw differs".into())}
     //4B3FD4..4B403A, normal LoadData+24==0: flags2/34 plus source
     //virtual48 Float3 take the hard-coded diagnostic and continue WITHOUT
     //the source numeric getter or setter. We retain that reached diagnostic
     //request as provenance; renderer/log-level output is outside this cell
     //projection. Fixed4 comparison policy is deliberately unsupported.
     if FLAGS[i]&2!=0{self.skipped_hardcoded.push(source);self.declarations.push((i,row.clone()));continue}
     let value=scalar_literal(raw,self.context.precision()).map_err(|e|e.to_string())?;
     self.assign_index(i,value)?;self.assignments.push(source);self.declarations.push((i,row.clone()));
    }
   }
  }self.validate_owner(catalog,bank)?;self.source_loaded=true;Ok(())
 }
 fn validate_owner(&self,catalog:&Catalog<'_>,bank:&Bank)->Result<()>{catalog.require_source_owner()?;let source=bank.combat_source()?;source.completed_means()?;if self.sources!=catalog.actors.sources||source.source_identities()!=self.sources.as_slice()||source.source_precision()!=self.context.precision()||source.context().reference_startup()!=self.context.reference_startup(){return Err("camera AVar destination source owner/precision differs".into())}
  let settings=catalog.settings.as_ref().ok_or("camera Settings owner absent")?;for(i,name)in NAMES.iter().enumerate(){let rows=settings.app_variables.get(&name.to_ascii_lowercase()).map(Vec::as_slice).unwrap_or(&[]);let held=self.declarations.iter().filter(|(j,_)|*j==i).map(|(_,row)|row).collect::<Vec<_>>();if rows.len()!=held.len()||rows.iter().zip(held).any(|(a,b)|a.source!=b.source||a.raw!=b.raw||a.field_type!=b.field_type){return Err(format!("camera current Settings source declaration differs for {name}"))}}
  let numeric=combat_value_source_context::bind_numeric(catalog,&self.context)?;crate::combat_source_receipt::require_reference_numeric(&numeric)}
 pub fn validate(&self,catalog:&Catalog<'_>,bank:&Bank)->Result<()>{if !self.source_loaded{return Err("camera AVar source pass is incomplete".into())}self.validate_owner(catalog,bank)}
 ///Reached native fast-table byte offset (1C..5C), read from these same cells.
 pub fn word(&self,catalog:&Catalog<'_>,bank:&Bank,offset:u32)->Result<u32>{self.validate(catalog,bank)?;let i=index(offset)?;Ok(self.words[i])}
 ///40E0A0/4B29B0: hard-coded flags deny before accessing the proposed value.
 ///The bounded cells' constructor bounds stay fixed when other cells change.
 pub fn assign(&mut self,catalog:&Catalog<'_>,bank:&Bank,offset:u32,value:impl FnOnce()->Result<u32>)->Result<bool>{self.validate(catalog,bank)?;let i=index(offset)?;if FLAGS[i]&2!=0{return Ok(false)}self.assign_index(i,value()?)?;Ok(true)}
 fn assign_index(&mut self,i:usize,value:u32)->Result<()>{let value=aa_numeric::Binary::from_f32_bits(value).map_err(|e|e.to_string())?;let mut word=value.store_f32().map_err(|e|e.to_string())?;if FLAGS[i]&32!=0{let [lo,hi]=bounds(i);if value.compare(&aa_numeric::Binary::from_f32_bits(lo).map_err(|e|e.to_string())?)==std::cmp::Ordering::Less{word=lo}else if value.compare(&aa_numeric::Binary::from_f32_bits(hi).map_err(|e|e.to_string())?)==std::cmp::Ordering::Greater{word=hi}}self.words[i]=word;Ok(())}
 pub fn source_precision(&self)->Precision{self.context.precision()}
 pub fn source_files(&self)->&[(String,String)]{&self.files}
 pub fn source_assignments(&self)->&[Provenance]{&self.assignments}
 pub fn skipped_hardcoded_declarations(&self)->&[Provenance]{&self.skipped_hardcoded}
}
fn index(offset:u32)->Result<usize>{if !(0x1c..=0x5c).contains(&offset)||offset%4!=0{return Err("camera AVar offset outside native7..23".into())}Ok((offset/4-7)as usize)}

#[cfg(test)]mod tests{
 use super::*;
 #[test]fn native_camera_registration_v14_patch_all_pc(){let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-camera-variables.json")).unwrap();let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),6);for row in rows{for(i,v)in row["variables"].as_array().unwrap().iter().enumerate(){assert_eq!(v["name"],NAMES[i]);assert_eq!(v["kind"],2);assert_eq!(v["flags"],FLAGS[i]);let original=row["image"]=="original";let mut b=bounds(i);if original&&i==10{b[1]=0x3fb33333}assert_eq!(v["bounds"],serde_json::json!(b));assert_eq!(v["word"],if original&&i==9{0x3fb33333}else{WORDS[i]});}}}
 #[test]fn actual_hardcoded_source_diagnostic_skips_numeric_getter_and_continues(){let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-native-camera/native-camera-source-policy.json")).unwrap();assert_eq!(fixture["executions"],24);for row in fixture["rows"].as_array().unwrap(){assert_ne!(row["input"]["flags"].as_u64().unwrap()&2,0);assert_eq!(row["output"]["height"],WORDS[15]);assert_eq!(row["output"]["zoom"],1.25f32.to_bits());assert_eq!(row["output"]["height_numeric_getter_calls"],0);assert_eq!(row["output"]["diagnostics"][0]["name"],"CameraHeight");}}
 #[test]#[ignore="requires actual original/v14 sources; no native game execution"]
 fn actual_source_current_camera_cells_and_readonly_gate(){
  use aa_rules::sources::SourceLimits;let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
  let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
  let actors=aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
  let receipt=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();crate::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();crate::combat_source_completion::compute(&catalog).unwrap();let bank=catalog.definition_bank.borrow();
  let mut state=State::from_fresh_sources_retained(&catalog,&bank,&mut files,NativeOverrides::ControlledNone).unwrap();
  assert_eq!(state.word(&catalog,&bank,0x24).unwrap(),0.1f32.to_bits());assert_eq!(state.word(&catalog,&bank,0x28).unwrap(),1.5f32.to_bits());assert_eq!(state.word(&catalog,&bank,0x2c).unwrap(),1.0f32.to_bits());assert_eq!(state.word(&catalog,&bank,0x40).unwrap(),2.1f32.to_bits());
  let called=std::cell::Cell::new(false);assert!(!state.assign(&catalog,&bank,0x40,||{called.set(true);Err("must not read".into())}).unwrap());assert!(!called.get());
  state.assign(&catalog,&bank,0x2c,||Ok(9.0f32.to_bits())).unwrap();assert_eq!(state.word(&catalog,&bank,0x2c).unwrap(),2.5f32.to_bits());
  assert!(State::from_fresh_sources_retained(&catalog,&bank,&mut files,NativeOverrides::Unbound).err().unwrap().state.is_none());assert!(!state.source_files().is_empty());assert!(state.source_assignments().len()>=4);
 }
}
