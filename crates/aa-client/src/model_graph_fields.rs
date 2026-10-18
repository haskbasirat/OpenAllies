//!Additional immutable source fields paired to a frozen raw graph artifact.
//!This is source projection only. No native processing/default-state receipt.
use serde::Deserialize;
use std::collections::BTreeMap;
type Result<T>=std::result::Result<T,String>;
const INDEX:&str=include_str!("../../../assets-cache/model-native-fields/index.json");
#[derive(Debug,Deserialize)]
pub(super) struct Fields{schema:u32,source:String,source_sha256:String,graph_sha256:String,pub nodes:Vec<Node>,properties:Vec<Property>,pub effects:Vec<Effect>}
#[derive(Debug,Deserialize)]
pub(super) struct Node{pub block:u32,pub kind:String,pub name:String,pub property_blocks:Vec<u32>,pub effect_blocks:Vec<Option<u32>>,pub controllers:Vec<Controller>,pub geometry_data:Option<GeometryData>}
#[derive(Debug,Deserialize,PartialEq,Eq)]
pub(super) struct GeometryData{pub block:u32,pub kind:String,pub count:u16,pub vertices_present:bool,pub normals_present:bool,pub flags_word:u16,pub consistency_flags:u16,pub triangle_count:Option<u16>,pub colors:Option<Vec<[u32;4]>>,pub uv_sets:Vec<Vec<[u32;2]>>}
#[derive(Debug,Deserialize,PartialEq,Eq)]
pub(super) struct Controller{pub block:u32,pub kind:String,pub flags:u32,pub stop:u32}
#[derive(Debug,Deserialize,PartialEq,Eq)]
pub(super) struct Property{pub block:u32,pub kind:String,pub controllers:Vec<Controller>,pub material_words:Option<[u32;14]>,pub alpha:Option<(u16,u8)>,pub vertex_color:Option<[u32;2]>}
#[derive(Debug,Deserialize)]
pub(super) struct Effect{pub block:u32,pub kind:String,pub controllers:Vec<Controller>}
impl Fields{
 pub(super) fn take_properties(&mut self)->Result<BTreeMap<u32,Property>>{
  let mut fields=BTreeMap::new();for property in std::mem::take(&mut self.properties){if fields.insert(property.block,property).is_some(){return Err("source graph property allocation identity repeats".into())}}Ok(fields)
 }
}
pub(super) fn load(store:&aa_assets::AssetStore,source:&str,sha:&str,graph_sha:&str)->Result<Fields>{
 let index:serde_json::Value=serde_json::from_str(INDEX).map_err(|e|e.to_string())?;
 let record=&index["by_source"][source.to_ascii_lowercase()];
 if record["source"]!=source||record["source_sha256"]!=sha||record["graph_sha256"]!=graph_sha{return Err("selected Model requires exact source-bound property/effect field projection".into())}
 let artifact=record["artifact"].as_str().ok_or("model graph fields artifact absent")?;
 let path=super::discover_artifact(store,"model-native-fields",sha,artifact)?;let bytes=std::fs::read(path).map_err(|e|e.to_string())?;
 if record["sha256"]!=super::digest(&bytes){return Err("selected Model graph fields artifact hash differs".into())}
 let fields:Fields=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
 if fields.schema!=2||fields.source!=source||fields.source_sha256!=sha||fields.graph_sha256!=graph_sha{return Err("selected Model graph fields source identity differs".into())}Ok(fields)
}
