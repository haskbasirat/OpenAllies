//!878F30: extra removal precedes the reached current warning-policy lookup.
type Result<T>=std::result::Result<T,String>;
#[derive(Clone,Copy,Debug,PartialEq,Eq)]pub(super) enum Mode{Fresh,Reused}
pub(super) trait Root{
 fn integer_version(&self)->Result<Option<u32>>;
 fn reuse_binary_metadata(&mut self)->Result<bool>;
 fn remove_version(&mut self)->Result<()>;
 fn metadata_named(&self)->Result<bool>;
 fn remove_metadata(&mut self)->Result<()>;
 fn warning_current(&mut self,old:Option<u32>)->Result<u8>;
 fn warning(&mut self,old:Option<u32>)->Result<()>;
}
pub(super) fn lookup(root:&mut impl Root)->Result<Mode>{
 let old=root.integer_version()?.filter(|version|*version!=u32::MAX);
 if let Some(version)=old{
  if version==7{return Ok(if root.reuse_binary_metadata()?{Mode::Reused}else{Mode::Fresh})}
  root.remove_version()?;if root.metadata_named()?{root.remove_metadata()?;}
 }
 if root.warning_current(old)?!=0{root.warning(old)?;}Ok(Mode::Fresh)
}
#[cfg(test)]mod tests{
 use super::*;
 struct Fields{version:Option<u32>,version_present:bool,binary:bool,metadata_present:bool,warning:u8,events:Vec<serde_json::Value>,fail:bool}
 impl Root for Fields{
  fn integer_version(&self)->Result<Option<u32>>{Ok(self.version)}
  fn reuse_binary_metadata(&mut self)->Result<bool>{Ok(self.binary)}
  fn remove_version(&mut self)->Result<()>{self.version_present=false;Ok(())}
  fn metadata_named(&self)->Result<bool>{Ok(self.metadata_present)}
  fn remove_metadata(&mut self)->Result<()>{self.metadata_present=false;Ok(())}
  fn warning_current(&mut self,old:Option<u32>)->Result<u8>{self.events.push(serde_json::json!({"read":if old.is_some(){"PreprocessedDataWrongVersionWarning"}else{"PreprocessedDataMissingWarning"},"value":self.warning}));if self.fail{return Err("actual current policy unbound".into())}Ok(self.warning)}
  fn warning(&mut self,_:Option<u32>)->Result<()>{self.events.push(serde_json::json!({"diagnostic":true}));Ok(())}
 }
 #[test]fn all_native_versions_types_and_lazy_warning_order_match(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-model-preprocessed.json")).unwrap();
  for row in fixture["rows"].as_array().unwrap(){let version=row["version"].as_i64().map(|v|v as u32);let mut fields=Fields{version,version_present:!row["version"].is_null(),binary:row["binary"].as_bool().unwrap(),metadata_present:row["binary"].as_bool().unwrap(),warning:row["warning"].as_u64().unwrap()as u8,events:vec![],fail:false};
   assert_eq!(lookup(&mut fields).unwrap()==Mode::Reused,row["reused_payload"].as_bool().unwrap());let mut present=vec![];if fields.version_present{present.push("ppdv")}if fields.metadata_present{present.push("ppd")}assert_eq!(serde_json::json!(present),row["present"]);assert_eq!(serde_json::json!(fields.events),row["events"]);
  }
 }
 #[test]fn missing_current_policy_keeps_the_native_removal_prefix(){let mut fields=Fields{version:Some(6),version_present:true,binary:true,metadata_present:true,warning:0,events:vec![],fail:true};assert!(lookup(&mut fields).is_err());assert!(!fields.version_present&&!fields.metadata_present);}
}
