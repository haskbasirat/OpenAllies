//!Source-created named AVars read by878F30. Their current values live only
//!in the existing Ui.avars.values map; this receipt owns type/source metadata.
use crate::{fs::DataRoot,view_variables::{SourceFile,SourceOverrides}};
use aa_rules::{native::index::native_enumerate,tgi::{Document,Item,Limits}};
use std::collections::HashMap;
pub(crate) const NAMES:[&str;2]=["PreprocessedDataMissingWarning","PreprocessedDataWrongVersionWarning"];
#[derive(Clone,Debug,Default)]pub(crate) struct Receipt{files:Vec<SourceFile>,bound:bool,failure:Option<String>}
impl Receipt{
 pub(crate) fn bind(&mut self,root:&DataRoot,scope:SourceOverrides,values:&mut HashMap<String,String>)->Result<(),String>{
  if self.bound||!self.files.is_empty(){return Err("Model AVar source receipt already attempted".into())}
  if scope!=SourceOverrides::ControlledNone{return Err("Model AVar profile/console source scope is unbound".into())}
  let files=native_enumerate("AVars*.tgi",&root.list_dir("/"));let mut declared=[false;2];
  if !files.iter().any(|name|name.eq_ignore_ascii_case("AVars.tgi")){return Err("Model named AVars require actual AVars.tgi source".into())}
  for file in files{let bytes=root.read(&file).map_err(|e|e.to_string())?;self.files.push(SourceFile{path:file.clone(),sha256:aa_rules::sources::sha256(&bytes)});
   let doc=Document::parse_native(bytes,Limits::default()).map_err(|e|e.to_string())?;let mut seen=[false;2];
   for item in &doc.items{let Item::Node(node)=item else{continue};if !doc.header_name(&node.header).is_some_and(|name|name.eq_ignore_ascii_case("Vars")){continue}if node.header.tokens.len()!=1{return Err("Model AVar source inheritance requires actual lookup owner".into())}
    for item in &node.items{let Item::Property(field)=item else{continue};let Some(name)=doc.property_name(field)else{continue};let Some(index)=NAMES.iter().position(|n|n.eq_ignore_ascii_case(&name))else{continue};
     if seen[index]{return Err(format!("{name} duplicate source field requires native hash traversal"))}seen[index]=true;
     let typed=field.left.len()==2&&doc.text(field.left[0].span).is_some_and(|kind|kind.eq_ignore_ascii_case("bool")||kind.eq_ignore_ascii_case("flag"));
     if !typed&&!(field.left.len()==1&&declared[index]){return Err(format!("{name} requires actual source-created Bool type"))}
     let raw=doc.property_value(field).ok_or("Model AVar source value absent")?.trim();if !raw.eq_ignore_ascii_case("true")&&!raw.eq_ignore_ascii_case("false"){return Err("Model AVar native Bool source conversion is required".into())}
     values.insert(NAMES[index].to_ascii_lowercase(),raw.to_ascii_lowercase());declared[index]=true;
    }
   }
  }
  if declared!=[true;2]{return Err("Model named AVar Bool allocations are missing".into())}self.bound=true;Ok(())
 }
 pub(crate) fn fail(&mut self,error:String){self.failure=Some(error)}
 pub(crate) fn current(&self,values:&HashMap<String,String>,index:usize)->Result<u8,String>{
  if let Some(error)=&self.failure{return Err(error.clone())}if !self.bound{return Err("Model current named AVar source owner is unbound".into())}
  let name=NAMES.get(index).ok_or("Model AVar index outside its named receipt")?;let value=values.get(&name.to_ascii_lowercase()).ok_or("same current named Model AVar allocation absent")?;
  if value.eq_ignore_ascii_case("true"){Ok(1)}else if value.eq_ignore_ascii_case("false"){Ok(0)}else{Err("current Model AVar Bool requires actual raw-byte setter representation".into())}
 }
}
#[cfg(test)]mod tests{
 use super::*;
 #[test]fn source_created_model_bool_reads_same_current_map_and_has_no_default(){
  let path=std::env::temp_dir().join(format!("aa-model-avars-{}",std::process::id()));std::fs::create_dir_all(&path).unwrap();std::fs::write(path.join("AVars.tgi"),"[Vars]\n{\nbool PreprocessedDataMissingWarning = true\nbool PreprocessedDataWrongVersionWarning = false\n}\n").unwrap();
  let root=DataRoot::new(path.clone());let mut receipt=Receipt::default();let mut values=HashMap::new();assert!(receipt.current(&values,0).is_err());receipt.bind(&root,SourceOverrides::ControlledNone,&mut values).unwrap();assert_eq!(receipt.current(&values,0).unwrap(),1);assert_eq!(receipt.current(&values,1).unwrap(),0);
  values.insert(NAMES[1].to_ascii_lowercase(),"true".into());assert_eq!(receipt.current(&values,1).unwrap(),1);values.remove(&NAMES[1].to_ascii_lowercase());assert!(receipt.current(&values,1).is_err());
  std::fs::remove_file(path.join("AVars.tgi")).unwrap();std::fs::remove_dir(path).unwrap();
 }
 #[test]
 #[ignore="requires actual original/v14 AVars source files; no profile override scope"]
 fn actual_model_warning_avars_bind_without_fabricated_current_defaults(){
  let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());let root=DataRoot::with_overlay(game.join("reconstruction/assets/original/Data"),game.join("reconstruction/native-game/runtime-expanded-v14/Data"));
  let mut values=HashMap::new();let mut receipt=Receipt::default();receipt.bind(&root,SourceOverrides::ControlledNone,&mut values).unwrap();assert_eq!(receipt.current(&values,0).unwrap(),1);assert_eq!(receipt.current(&values,1).unwrap(),0);assert!(!receipt.files.is_empty());assert!(receipt.files.iter().all(|file|file.sha256.len()==64));
 }
}
