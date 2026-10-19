//!Native source fields of the selected NIF, computed at load time from the
//!actual bytes by the aa-formats port of AA.exe's NiStream reader (stream
//!reads, version branches and LinkObject list order). There is no
//!precomputed sidecar: every model the scene selects gets the same projection.
//!This is source projection only. No native processing/default-state receipt.
use aa_formats::nif::{self,is_kind_of,NifFile,Value};
type Result<T>=std::result::Result<T,String>;
#[derive(Debug)]
pub(super) struct Fields{pub nodes:Vec<Node>,properties:Vec<Property>,pub effects:Vec<Effect>}
#[derive(Debug)]
pub(super) struct Node{pub block:u32,pub kind:String,pub name:String,pub property_blocks:Vec<u32>,pub effect_blocks:Vec<Option<u32>>,pub controllers:Vec<Controller>,pub geometry_data:Option<GeometryData>}
#[derive(Debug,PartialEq,Eq)]
pub(super) struct GeometryData{pub block:u32,pub kind:String,pub count:u16,pub vertices_present:bool,pub normals_present:bool,pub flags_word:u16,pub consistency_flags:u16,pub triangle_count:Option<u16>,pub colors:Option<Vec<[u32;4]>>,pub uv_sets:Vec<Vec<[u32;2]>>}
///`flags` is the word NiTimeController::LoadBinary 89F070 stores; `flags_read`
///is the file word (they differ for versions below 10.0.1.3).
#[derive(Debug,PartialEq,Eq)]
pub(super) struct Controller{pub block:u32,pub kind:String,pub flags:u32,pub flags_read:u32,pub stop:u32}
///`alpha` holds the flags NiAlphaProperty::LoadBinary 87CFD0 stores and the
///threshold; `alpha_flags_read` is the file word for the same flags.
#[derive(Debug,PartialEq,Eq)]
pub(super) struct Property{pub block:u32,pub kind:String,pub controllers:Vec<Controller>,pub material_words:Option<[u32;14]>,pub alpha:Option<(u16,u8)>,pub alpha_flags_read:Option<u16>,pub vertex_color:Option<[u32;2]>}
#[derive(Debug)]
pub(super) struct Effect{pub block:u32,pub kind:String,pub controllers:Vec<Controller>}
impl Fields{
 pub(super) fn take_properties(&mut self)->Result<std::collections::BTreeMap<u32,Property>>{
  let mut fields=std::collections::BTreeMap::new();for property in std::mem::take(&mut self.properties){if fields.insert(property.block,property).is_some(){return Err("source graph property allocation identity repeats".into())}}Ok(fields)
 }
}
fn err(e:nif::NifError)->String{e.to_string()}
fn text(bytes:Option<&[u8]>)->Result<String>{
 //The native name is the C string LoadCString 880EE0 stored (to its first NUL).
 let bytes=bytes.unwrap_or(&[]);let bytes=bytes.split(|&b|b==0).next().unwrap_or(&[]);
 if !bytes.is_ascii(){return Err("actual native non-ASCII name conversion is required".into())}
 Ok(String::from_utf8(bytes.to_vec()).expect("ASCII"))
}
fn block(i:usize)->Result<u32>{u32::try_from(i).map_err(|_|"NIF object index exceeds the native 32-bit link ID".into())}
fn controllers(nif:&NifFile,object:usize)->Result<Vec<Controller>>{
 nif.controllers(object).map_err(err)?.into_iter().map(|c|{let o=&nif.objects[c];let field=o.fields.field("flags").ok_or("89F070 controller flags absent")?;
  let flags=match field.value{Value::U16(v)=>v as u32,_=>return Err("89F070 controller flags type".into())};
  let flags_read=match &field.read{Some(Value::U16(v))=>*v as u32,Some(_)=>return Err("89F070 controller file flags type".into()),None=>flags};
  Ok(Controller{block:block(c)?,kind:o.class.into(),flags,flags_read,stop:o.fields.f32("stop").ok_or("89F070 stop time absent")?})}).collect()
}
fn pairs<const N:usize>(words:&[u32])->Result<Vec<[u32;N]>>{if words.len()%N!=0{return Err("NIF word array length differs from its element width".into())}Ok(words.chunks_exact(N).map(|c|c.try_into().expect("chunk")).collect())}
///NiGeometryData fields 8AC0B0 (+0xC count, +0x30 data flags, +0x32 consistency).
fn geometry_data(nif:&NifFile,data:usize)->Result<GeometryData>{
 let o=&nif.objects[data];let f=&o.fields;if !is_kind_of(o.class,"NiGeometryData"){return Err("NiGeometry data link is not NiGeometryData".into())}
 let count=f.u16("vertex_count").ok_or("8AC0D1 vertex count absent")?;
 let present=|name:&str|->Result<bool>{Ok(f.u32(name).ok_or("8AC0B0 presence word absent")?!=0)};
 //+0x30 is 0 in the constructor 8ABAC0 until a version branch stores it.
 let flags_word=f.u16("data_flags").unwrap_or(0);
 //+0x32: read from 5.0.0.10; otherwise the constructor's 0, which the
 //NiParticlesData constructor 8C3130 and loader 8C35F4 force to 0x8000.
 let consistency=match(f.u16("consistency_flags"),f.u16("consistency_flags_native")){(Some(v),_)=>v,(None,Some(v))=>v,(None,None)=>0};
 let colors=if present("has_colors")?{Some(pairs::<4>(f.f32s("colors").ok_or("8AC29C colours absent")?)?)}else{None};
 let sets=(flags_word&0x3f)as usize;let uv=match f.f32s("uv_sets"){Some(words)=>{let per=count as usize*2;if words.len()!=sets*per{return Err("8AC3CC UV array length differs".into())}(0..sets).map(|s|pairs::<2>(&words[s*per..(s+1)*per])).collect::<Result<Vec<_>>>()?},None=>vec![]};
 Ok(GeometryData{block:block(data)?,kind:o.class.into(),count,vertices_present:present("has_vertices")?,normals_present:present("has_normals")?,flags_word,consistency_flags:consistency,
  triangle_count:if is_kind_of(o.class,"NiTriBasedGeomData"){Some(f.u16("triangle_count").ok_or("87FA6C triangle count absent")?)}else{None},colors,uv_sets:uv})
}
///Nodes in the source graph walk order: the model loader's root (5A7175:
///the first top-level object, cast to NiAVObject), then the linked NiNode
///child slots in preorder. An object reached twice is a cyclic graph.
pub(super) fn native(raw:&[u8])->Result<(NifFile,Fields)>{
 let nif=nif::load(raw).map_err(err)?;
 if !nif.requires_native_post_process.is_empty(){return Err("selected NIF old particle system requires the native 8E93B0 post-load conversion".into())}
 let mut nodes=Vec::new();let mut seen=std::collections::BTreeSet::new();
 let root=nif.top_level.first().copied().flatten().ok_or("5A71A7 selected NIF first top-level object is absent or NULL (diagnostic path)")?;
 let mut work:Vec<usize>=vec![root];
 while let Some(i)=work.pop(){
  if !seen.insert(i){return Err("NiAVObject reached twice: the linked native graph is cyclic".into())}
  let o=&nif.objects[i];if !is_kind_of(o.class,"NiAVObject"){return Err(format!("NonAV root/child {}",o.class))}
  let mut property_blocks=Vec::new();for p in nif.av_properties(i).map_err(err)?{property_blocks.push(block(p.ok_or("86F3A0 NULL Property link: the native list keeps a NULL entry")?)?)}
  let effect_blocks=nif.node_effects(i).map_err(err)?.into_iter().map(|e|block(e).map(Some)).collect::<Result<Vec<_>>>()?;
  let geometry=if is_kind_of(o.class,"NiGeometry"){nif.linked(i,"data").map(|d|geometry_data(&nif,d)).transpose()?}else{None};
  nodes.push(Node{block:block(i)?,kind:o.class.into(),name:text(o.fields.string("name").ok_or("86B090 name absent")?)?,property_blocks,effect_blocks,controllers:controllers(&nif,i)?,geometry_data:geometry});
  work.extend(nif.node_children(i).map_err(err)?.into_iter().rev().flatten());
 }
 let mut properties=Vec::new();let mut effects=Vec::new();
 for(i,o)in nif.objects.iter().enumerate(){
  if is_kind_of(o.class,"NiProperty"){let f=&o.fields;
   let material_words=if o.class=="NiMaterialProperty"{let mut w=Vec::with_capacity(14);for name in ["ambient","diffuse","specular","emissive"]{w.extend_from_slice(f.f32s(name).ok_or("87D3C0 colour absent")?)}w.push(f.f32("glossiness").ok_or("87D400 glossiness absent")?);w.push(f.f32("alpha").ok_or("87D411 alpha absent")?);Some(<[u32;14]>::try_from(w).map_err(|_|"87D3C0 material word census differs")?)}else{None};
   let(alpha,alpha_flags_read)=if o.class=="NiAlphaProperty"{let field=f.field("flags").ok_or("87CFD0 alpha flags absent")?;let flags=match field.value{Value::U16(v)=>v,_=>return Err("87CFD0 alpha flags type".into())};let read=match &field.read{Some(Value::U16(v))=>*v,_=>flags};(Some((flags,f.u8("threshold").ok_or("87D016 threshold absent")?)),Some(read))}else{(None,None)};
   let vertex_color=if o.class=="NiVertexColorProperty"{Some([f.u32("vertex_mode").ok_or("87B75B vertex mode absent")?,f.u32("lighting_mode").ok_or("87B774 lighting mode absent")?])}else{None};
   properties.push(Property{block:block(i)?,kind:o.class.into(),controllers:controllers(&nif,i)?,material_words,alpha,alpha_flags_read,vertex_color});
  }
  if is_kind_of(o.class,"NiDynamicEffect"){effects.push(Effect{block:block(i)?,kind:o.class.into(),controllers:controllers(&nif,i)?})}
 }
 Ok((nif,Fields{nodes,properties,effects}))
}
#[cfg(test)]mod tests{
 use super::*;
 ///The three PyFFI-projected sidecars this replaced (GBR corps models) must
 ///agree with the native projection in every field, with the property list in
 ///the native head-first order (the reverse of the link order).
 #[test]fn native_projection_matches_the_former_sidecars(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
  let game=root.join("..");let index:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("assets-cache/model-native-fields/index.json")).unwrap()).unwrap();
  let records=index["by_source"].as_object().unwrap();assert_eq!(records.len(),3);
  for record in records.values(){
   let source=record["source"].as_str().unwrap();let path=game.join("reconstruction/assets/original/Data").join(source);
   let Ok(raw)=std::fs::read(&path)else{eprintln!("skipping {}: original data absent",path.display());continue};
   let doc:serde_json::Value=serde_json::from_slice(&std::fs::read(root.join("assets-cache/model-native-fields").join(record["artifact"].as_str().unwrap())).unwrap()).unwrap();
   let(_,fields)=native(&raw).unwrap();let old=doc["nodes"].as_array().unwrap();assert_eq!(old.len(),fields.nodes.len(),"{source}");
   let ctl=|c:&Controller|serde_json::json!({"block":c.block,"kind":c.kind,"flags":c.flags_read,"stop":c.stop});
   for(a,b)in old.iter().zip(&fields.nodes){
    let mut reversed:Vec<u32>=serde_json::from_value(a["property_blocks"].clone()).unwrap();reversed.reverse();
    assert_eq!((a["block"].as_u64().unwrap() as u32,a["kind"].as_str().unwrap(),a["name"].as_str().unwrap()),(b.block,b.kind.as_str(),b.name.as_str()),"{source}");
    assert_eq!(reversed,b.property_blocks,"{source} {}",b.name);
    assert!(a["effect_blocks"].as_array().unwrap().iter().all(|e|e.is_null())&&b.effect_blocks.is_empty(),"{source}");
    assert_eq!(a["controllers"],serde_json::json!(b.controllers.iter().map(ctl).collect::<Vec<_>>()),"{source}");
    match(&a["geometry_data"],&b.geometry_data){(serde_json::Value::Null,None)=>{},(g,Some(n))=>{
     assert_eq!(g["block"].as_u64().unwrap() as u32,n.block);assert_eq!(g["kind"].as_str().unwrap(),n.kind);assert_eq!(g["count"].as_u64().unwrap() as u16,n.count);
     assert_eq!(g["vertices_present"].as_bool().unwrap(),n.vertices_present);assert_eq!(g["normals_present"].as_bool().unwrap(),n.normals_present);
     assert_eq!(g["flags_word"].as_u64().unwrap() as u16,n.flags_word);assert_eq!(g["consistency_flags"].as_u64().unwrap() as u16,n.consistency_flags);
     assert_eq!(g["triangle_count"].as_u64().map(|v|v as u16),n.triangle_count);
     assert_eq!(serde_json::from_value::<Option<Vec<[u32;4]>>>(g["colors"].clone()).unwrap(),n.colors);
     assert_eq!(serde_json::from_value::<Vec<Vec<[u32;2]>>>(g["uv_sets"].clone()).unwrap(),n.uv_sets);
    },other=>panic!("{source} geometry presence differs {other:?}")}
   }
   let mut native_properties=fields.properties;native_properties.sort_by_key(|p|p.block);
   let old=doc["properties"].as_array().unwrap();assert_eq!(old.len(),native_properties.len(),"{source}");
   for(a,b)in old.iter().zip(&native_properties){
    assert_eq!((a["block"].as_u64().unwrap() as u32,a["kind"].as_str().unwrap()),(b.block,b.kind.as_str()));
    assert_eq!(a["controllers"],serde_json::json!(b.controllers.iter().map(ctl).collect::<Vec<_>>()));
    assert_eq!(serde_json::from_value::<Option<[u32;14]>>(a["material_words"].clone()).unwrap(),b.material_words);
    assert_eq!(serde_json::from_value::<Option<(u16,u8)>>(a["alpha"].clone()).unwrap(),b.alpha_flags_read.zip(b.alpha.map(|a|a.1)));
    assert_eq!(serde_json::from_value::<Option<[u32;2]>>(a["vertex_color"].clone()).unwrap(),b.vertex_color);
   }
   assert_eq!(doc["effects"].as_array().unwrap().len(),fields.effects.len());
  }
 }
}
