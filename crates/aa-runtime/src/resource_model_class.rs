//!9D06E0 canonical NiAVObj descriptor and59CB00's current RAM weight in the
//!same ResourceManager. The shared fresh policy is source/PC provenance only;
//!this class is created lazily after the actual Model Entry publication.
use crate::{catalog::Catalog,definitions::Bank,resource_audio_class::FreshCacheVariableScope};
use aa_rules::{native::index::native_enumerate,resources::{Provenance,SourceIdentity},sources::V14Sources,tgi::{Document,Item,Limits}};
type Result<T>=std::result::Result<T,String>;
pub struct ModelClass{ram_weight:i32,vram_weight:i32,declarations:Vec<Provenance>,files:Vec<SourceIdentity>}
impl ModelClass{
 pub fn name(&self)->&'static str{"KKC_NiAVObjResource"}
 pub fn ram_weight(&self)->i32{self.ram_weight}pub fn vram_weight(&self)->i32{self.vram_weight}
 pub fn declarations(&self)->&[Provenance]{&self.declarations}pub fn source_files(&self)->&[SourceIdentity]{&self.files}
 pub(crate) fn from_current(catalog:&Catalog<'_>,bank:&Bank,sources:&mut V14Sources,scope:&FreshCacheVariableScope)->Result<Self>{
  scope.require(catalog,bank)?;
  let paths=sources.catalog().into_iter().map(|p|p.logical_path).collect::<Vec<_>>();
  let ordered=["AVars*.tgi","UVars*.tgi"].into_iter().flat_map(|p|native_enumerate(p,&paths)).collect::<Vec<_>>();
  if ordered!=scope.variable_files().iter().map(|s|s.path.clone()).collect::<Vec<_>>(){return Err("Model cache current variable enumeration differs".into())}
  let mut values=Vec::new();let mut files=Vec::new();
  for expected in scope.variable_files(){
   let file=sources.read(&expected.path).map_err(|e|e.to_string())?;
   let actual=SourceIdentity{path:file.logical_path.clone(),sha256:file.sha256.clone(),origin:file.origin.clone()};
   if &actual!=expected||aa_rules::sources::sha256(&file.bytes)!=expected.sha256{return Err("Model cache current variable source changed".into())}
   let doc=Document::parse_native(file.bytes,Limits::default()).map_err(|e|e.to_string())?;
   for item in &doc.items{let Item::Node(node)=item else{continue};if !doc.header_name(&node.header).is_some_and(|n|n.eq_ignore_ascii_case("Vars")){continue}
    for item in &node.items{let Item::Property(p)=item else{continue};if !doc.property_name(p).is_some_and(|n|n.eq_ignore_ascii_case("ResourceCacheNiAVObjRAMWeight")){continue}
     if p.left.len()!=2||!doc.text(p.left[0].span).is_some_and(|t|t.eq_ignore_ascii_case("int")){return Err("actual Model cache weight Int assignment owner is required".into())}
     let value=doc.property_value(p).ok_or("Model cache weight source unassigned")?.trim().parse::<i32>().map_err(|_|"Model cache weight outside literalInt domain")?;
     values.push((value,Provenance{path:actual.path.clone(),sha256:actual.sha256.clone(),span:p.span}));
    }
   }files.push(actual);
  }
  let current=match values.as_slice(){[]=>None,[(v,_)]=>Some(*v),_=>return Err("Model cache reloaded/duplicate current weight owner is required".into())};
  let(ram_weight,vram_weight)=weights(current)?;
  Ok(Self{ram_weight,vram_weight,declarations:values.into_iter().map(|(_,p)|p).collect(),files})
 }
}
pub(crate) fn weights(current:Option<i32>)->Result<(i32,i32)>{crate::ni_cache_math::class_weights(current)}
