//!The selected raw NIF and the paired modern render artifact share one owned
//!static graph. Native scalar processing uses the raw binary32 source words;
//!converted meshes are retained only as the modern rendering representation.
//!This does not implement the Gamebryo allocation/refcount ABI or GPU readiness.
use aa_assets::{AssetStore,PreparedModel};
use aa_numeric::Precision;
use aa_runtime::{resource_model_payload::{Input,Payload,Model,Metadata,MetadataPolicy},resource_model_bounds as math,resource_model_source::Context as SourceContext};
use aa_runtime::resource_model_defaults::{Kind as PropertyKind,Handle as DefaultProperty};
use serde::Deserialize;
use sha2::{Digest,Sha256};
use std::{collections::{BTreeSet,BTreeMap},sync::Arc};
#[path="model_property_metadata.rs"]
mod property_metadata;
#[path="model_graph_fields.rs"]
mod graph_fields;
#[path="model_hidden_tree.rs"]
mod hidden_tree;
#[path="model_property_state.rs"]
mod property_state;
#[path="model_preprocessed.rs"]
mod preprocessed;
#[path="model_metadata_flow.rs"]
mod metadata_flow;
#[path="model_controllers.rs"]
mod controllers;
#[path="model_native_graph.rs"]
mod native_graph;
#[path="model_selection.rs"]
mod selection;
type Result<T>=std::result::Result<T,String>;
const IDENTITY:[u32;13]=[0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000];
const MERGE_EPSILON:u32=897988541; //nativeB32D94 image-backed Ni math constant
const FOOTPRINT_THRESHOLD:u32=0x3dcccccd; //native9FE0C0
fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn finite(words:impl IntoIterator<Item=u32>)->Result<()>{if words.into_iter().any(|w|!f32::from_bits(w).is_finite()){return Err("selected NIF nonfinite source field requires native exceptional numeric path".into())}Ok(())}
#[derive(Debug,Deserialize)]struct Document{schema:u32,source:String,source_sha256:String,converted_sha256:String,material_audit_sha256:String,version:u32,roots:Vec<Option<usize>>,nodes:Vec<Node>,textures:Vec<Texture>,mesh_mapping:serde_json::Value}
#[derive(Debug,Deserialize)]struct Node{
 kind:String,name:String,parent:Option<usize>,hierarchy:Vec<String>,local:[u32;13],flags:u32,children:Vec<Option<usize>>,
 vertices:Vec<[u32;3]>,#[serde(default)]normals:Vec<[u32;3]>,sphere:Option<[u32;4]>,skin:bool,tags:Vec<String>,
 properties:Vec<Property>,controllers:Vec<Controller>,extras:Vec<serde_json::Value>,
 #[serde(default)]has_normals:bool,
 data_block:Option<u32>,#[serde(skip)]property_blocks:Vec<u32>,#[serde(skip)]generated:BTreeMap<String,Extra>,
 #[serde(skip)]world:Option<[u32;13]>,#[serde(skip)]world_sphere:Option<[u32;4]>,
 #[serde(skip)]mode_c0:Option<u32>,#[serde(skip)]detached:bool,
 #[serde(skip)]source_block:Option<u32>,#[serde(skip)]effect_blocks:Option<Vec<Option<u32>>>,
 #[serde(skip)]property_state:Option<Arc<[PropertySource;12]>>,
 #[serde(skip)]effects_updated:bool,
}
#[derive(Debug,Deserialize,PartialEq,Eq)]struct Property{kind:String,block:u32,#[serde(default)]flags:u32,#[serde(default)]slots:Vec<Option<Descriptor>>,#[serde(default)]shader_textures:u32,#[serde(default)]shader_maps:Vec<Option<Descriptor>>,#[serde(default)]extras:Vec<serde_json::Value>,#[serde(skip)]generated:BTreeMap<String,Extra>,#[serde(skip)]material:Option<[u32;14]>,#[serde(skip)]alpha_threshold:Option<u8>,#[serde(skip)]vertex_color:Option<[u32;2]>,#[serde(skip)]controllers:Option<Vec<graph_fields::Controller>>}
#[derive(Clone,Debug,Deserialize,PartialEq,Eq)]struct Descriptor{texture:Option<String>,source:Option<u32>,filter:u32}
#[derive(Debug,Deserialize)]struct Controller{kind:String,flags:u32,stop:u32}
#[derive(Debug,Deserialize)]struct Texture{block:u32,filename:Option<String>,extras:Vec<serde_json::Value>,#[serde(skip)]generated:BTreeMap<String,Extra>}
#[derive(Debug,PartialEq,Eq)]struct Geometry{vertices:Vec<[u32;3]>,sphere:Option<[u32;4]>,normals:Option<Vec<[u32;3]>>,fields:Option<GeometryFields>}
#[derive(Debug,PartialEq,Eq)]struct GeometryFields{kind:String,count:u16,vertices_present:bool,flags:u16,consistency:u16,triangles:Option<u16>,colors:Option<Vec<[u32;4]>>,uv_sets:Vec<Vec<[u32;2]>>}
#[derive(Debug,PartialEq,Eq)]enum Extra{Integer(u32),Boolean(u32),Text(Vec<u16>),Processed(Arc<Metadata>),TextureNames(Arc<Vec<Vec<u16>>>),Material([u32;12]),Opacity(property_metadata::Opacity) }
#[derive(Clone,Debug)]enum PropertySource{Default(DefaultProperty),Source(u32)}
#[derive(Debug)]struct MetadataWork{bounds:math::Bounds,stop:u32,blend:bool}
#[derive(Debug)]struct TextureUse{name:Vec<u16>,total:u32,particles:u32}
#[derive(Debug)]struct Selected{
 source_sha:String,filename:Vec<u16>,source:Option<SourceContext>,graph:Document,root:usize,pc:Precision,
 render:PreparedModel,metadata:Option<Arc<Metadata>>,work:Option<MetadataWork>,texture_rows:Vec<TextureUse>,texture_names:Option<Arc<Vec<Vec<u16>>>>,processing:bool,wrapped:bool,updated:bool,
 data:BTreeMap<u32,Geometry>,properties:BTreeMap<u32,Property>,texture_objects:BTreeMap<u32,Texture>,
 ///The native object graph of the same bytes, and the time-controller state
 ///of its 86E8D0 updates with controllers (87ABC0, 5AA0DD).
 nif:aa_formats::nif::NifFile,controllers:Option<controllers::Runtime>,
}
///The input was minted by the actual Resource file read. The material loader
///reads and verifies converted bytes once; this graph projection is separately
///bound to the exact input SHA and that SAME paired source-node audit.
pub(crate) fn prepare(input:&Input,store:&AssetStore,pc:Precision)->Result<Payload>{
 let filename=input.resource_ids().to_vec();let source=input.source_context().cloned();
 input.prepare_with(|raw,selected|{
  if digest(raw)!=input.sha256(){return Err("selected NIF byte allocation digest differs".into())}
  //The actual bytes through the aa-formats port of AA.exe's NiStream reader
  //(every shipped NIF version, native LinkObject list order and child
  //re-parenting). The source graph is built from that native object graph
  //at load time; the paired material audit binds the converted render
  //artifact and its meshes' source-node rows to the same bytes.
  let(nif,mut additional)=graph_fields::native(raw)?;
  let fixes=crate::model_fixes::ModelFixes::discover(store).map_err(|e|e.to_string())?;
  let(canonical,converted_sha,audit_sha,mapping)=fixes.native_pair(selected.logical_path(),input.sha256()).map_err(|e|e.to_string())?;
  if !canonical.eq_ignore_ascii_case(&selected.logical_path().replace('\\',"/")){return Err("selected NIF physical source differs from its paired audit".into())}
  let mut graph=native_graph::document(&nif,&canonical,input.sha256(),converted_sha,audit_sha,mapping)?;
  let converted=fixes.load_selected_model(store,&graph.source,input.sha256()).map_err(|e|e.to_string())?;
  let mut render=converted.prepare().map_err(|e|e.to_string())?;fixes.apply(store,&mut render).map_err(|e|e.to_string())?;
  let root=validate(&graph,nif.version)?;
  //Materialize each serialized Data/Property/Texture block ONCE. Aliased
  //node references must agree before the projection's duplicate input rows
  //are consumed; later mutations address only this sole typed owner.
  let mut data=BTreeMap::new();let mut properties=BTreeMap::new();let mut texture_objects=BTreeMap::new();
  for texture in std::mem::take(&mut graph.textures){if texture_objects.insert(texture.block,texture).is_some(){return Err("duplicate NIF SourceTexture block identity".into())}}
  for node in &mut graph.nodes{
   if let Some(block)=node.data_block{if node.has_normals&&node.normals.len()!=node.vertices.len(){return Err("selected NIF normal array differs from its GeometryData vertex count".into())}if !node.has_normals&&!node.normals.is_empty(){return Err("selected NIF normal pointer/array source differs".into())}let fields=Geometry{vertices:std::mem::take(&mut node.vertices),sphere:node.sphere.take(),normals:node.has_normals.then(||std::mem::take(&mut node.normals)),fields:None};if let Some(old)=data.get(&block){if old!=&fields{return Err("aliased NIF GeometryData fields differ".into())}}else{data.insert(block,fields);}}
   for property in std::mem::take(&mut node.properties){node.property_blocks.push(property.block);for descriptor in property.slots.iter().flatten(){if let Some(source)=descriptor.source{let actual=texture_objects.get(&source).ok_or("NIF descriptor SourceTexture allocation absent")?;if actual.filename!=descriptor.texture{return Err("NIF descriptor filename differs from its shared SourceTexture owner".into())}}}
    if let Some(old)=properties.get(&property.block){if old!=&property{return Err("aliased NIF Property fields differ".into())}}else{properties.insert(property.block,property);}}
  }
  if additional.nodes.len()!=graph.nodes.len(){return Err("source graph node allocation census differs".into())}
  let mut node_blocks=BTreeSet::new();for(node,fields)in graph.nodes.iter_mut().zip(std::mem::take(&mut additional.nodes)){
   //The projection lists Property links in link order; NiAVObject::LinkObject
   //86F3A0 inserts each at the list head from 4.1.0.8.
   let mut link_order=node.property_blocks.clone();if nif.version>=0x0401_0008{link_order.reverse()}
   if node.kind!=fields.kind||node.name!=fields.name||link_order!=fields.property_blocks||!node_blocks.insert(fields.block)
      ||node.controllers.len()!=fields.controllers.len()||node.controllers.iter().zip(&fields.controllers).any(|(a,b)|a.kind!=b.kind||a.flags!=b.flags_read||a.stop!=b.stop){return Err("source graph node/Property/controller identity differs".into())}
   native_node_fields(&nif,fields.block,node)?;node.property_blocks=fields.property_blocks;
   for(a,b)in node.controllers.iter_mut().zip(&fields.controllers){a.flags=b.flags}
   if let Some(g)=fields.geometry_data{
    if node.data_block!=Some(g.block){return Err("same GeometryData source allocation differs".into())}let actual=data.get_mut(&g.block).ok_or("GeometryData source fields lack actual block owner")?;
    if (g.vertices_present&&g.count as usize!=actual.vertices.len())||(!g.vertices_present&&!actual.vertices.is_empty())||g.normals_present!=actual.normals.is_some(){return Err("GeometryData source pointer/count fields differ from actual arrays".into())}
    native_geometry_words(&nif,g.block,actual)?;
    let fields=GeometryFields{kind:g.kind,count:g.count,vertices_present:g.vertices_present,flags:g.flags_word,consistency:g.consistency_flags,triangles:g.triangle_count,colors:g.colors,uv_sets:g.uv_sets};if let Some(old)=&actual.fields{if old!=&fields{return Err("aliased GeometryData additional source fields differ".into())}}else{actual.fields=Some(fields)}
   }else if node.data_block.is_some(){return Err("actual GeometryData additional source fields absent".into())}
   node.source_block=Some(fields.block);node.effect_blocks=Some(fields.effect_blocks);
  }
  let mut source_properties=additional.take_properties()?;
  for(block,property)in &mut properties{let fields=source_properties.remove(block).ok_or("same NIF Property source fields absent")?;if property.kind!=fields.kind{return Err("same NIF Property source class differs".into())}if let Some((flags,threshold))=fields.alpha{if Some(property.flags)!=fields.alpha_flags_read.map(u32::from){return Err("same AlphaProperty source flags differ".into())}property.flags=flags as u32;property.alpha_threshold=Some(threshold)}property.material=fields.material_words;property.vertex_color=fields.vertex_color;property.controllers=Some(fields.controllers);}
  Ok(Box::new(Selected{source_sha:input.sha256().into(),filename,source,graph,root,pc,render,metadata:None,work:None,texture_rows:vec![],texture_names:None,processing:false,wrapped:false,updated:false,data,properties,texture_objects,nif,controllers:None})as Box<dyn Model>)
 })
}
///NiAVObject flags and local transform (86F150) from the native reader: the
///projection carries the file words; the node keeps the stored flags, which
///versions below 10.0.1.2 reduce to their low six bits.
fn native_node_fields(nif:&aa_formats::nif::NifFile,block:u32,node:&mut Node)->Result<()>{
 let f=&nif.objects[block as usize].fields;let flags=f.field("flags").ok_or("86F174 NiAVObject flags absent")?;
 let stored=match flags.value{aa_formats::nif::Value::U16(v)=>v as u32,_=>return Err("86F174 flags type".into())};
 let read=match &flags.read{Some(aa_formats::nif::Value::U16(v))=>*v as u32,_=>stored};
 if node.flags!=read{return Err("selected NIF node flags differ from the native file word".into())}
 let mut local=Vec::with_capacity(13);local.extend_from_slice(f.f32s("rotation").ok_or("870C10 rotation absent")?);local.extend_from_slice(f.f32s("translation").ok_or("869420 translation absent")?);local.push(f.f32("scale").ok_or("86F206 scale absent")?);
 if local!=node.local{return Err("selected NIF local transform words differ from the native reader".into())}
 node.flags=stored;Ok(())
}
///Vertex, normal and bound words of the same GeometryData (8AC0B0).
fn native_geometry_words(nif:&aa_formats::nif::NifFile,block:u32,actual:&Geometry)->Result<()>{
 let f=&nif.objects[block as usize].fields;
 let vertices=f.f32s("vertices").unwrap_or(&[]);if vertices.chunks_exact(3).map(|c|[c[0],c[1],c[2]]).ne(actual.vertices.iter().copied()){return Err("selected NIF vertex words differ from the native reader".into())}
 if let Some(normals)=&actual.normals{let words=f.f32s("normals").ok_or("8AC25B normals absent")?;if words.len()<normals.len()*3||words.chunks_exact(3).take(normals.len()).map(|c|[c[0],c[1],c[2]]).ne(normals.iter().copied()){return Err("selected NIF normal words differ from the native reader".into())}}
 let center=f.f32s("center").ok_or("8C5470 bound centre absent")?;let sphere=[center[0],center[1],center[2],f.f32("radius").ok_or("8C548C bound radius absent")?];
 if actual.sphere.is_some_and(|s|s!=sphere){return Err("selected NIF bound words differ from the native reader".into())}Ok(())
}
fn validate(d:&Document,native_version:u32)->Result<usize>{
 if d.version!=native_version{return Err(format!("selected NIF projection version {:X} differs from the native header {:X}",d.version,native_version))}
 if d.roots.len()!=1{return Err("selected NIF multi-root linking requires actual NiStream root operation".into())}let root=d.roots[0].ok_or("selected NIF root is NULL")?;
 if root>=d.nodes.len()||d.nodes[root].parent.is_some(){return Err("raw NIF root identity differs".into())}
 let mut seen=BTreeSet::new();let mut work=vec![root];while let Some(i)=work.pop(){
  if !seen.insert(i){return Err("raw NIF graph has aliased/cyclic node ownership".into())}let n=d.nodes.get(i).ok_or("raw NIF child index escapes allocation")?;
  finite(n.local)?;if let Some(s)=n.sphere{finite(s)?}for v in &n.vertices{finite(*v)?;}
  for child in n.children.iter().flatten(){if d.nodes.get(*child).is_none_or(|c|c.parent!=Some(i)){return Err("raw NIF parent/child identity differs".into())}work.push(*child)}
 }
 if seen.len()!=d.nodes.len(){return Err("raw NIF graph contains unreachable allocations".into())}
 for row in d.mesh_mapping.as_array().ok_or("raw model mapping absent")?{let i=row["source_node_ordinal"].as_u64().ok_or("raw model ordinal absent")?as usize;let n=d.nodes.get(i).ok_or("paired mesh source node absent")?;
  if row["name"]!=n.name||row["hierarchy"]!=serde_json::json!(n.hierarchy)||row["vertex_count"]!=n.vertices.len(){return Err("raw NIF geometry differs from paired native-node mapping".into())}}
 Ok(root)
}
struct Hidden<'a>(&'a mut[Node]);
impl hidden_tree::Tree for Hidden<'_>{
 fn parent(&self,i:usize)->Option<usize>{self.0[i].parent}
 fn children(&self,i:usize)->&[Option<usize>]{&self.0[i].children}
 fn is_node(&self,i:usize)->bool{self.0[i].kind=="NiNode"}
 fn hidden(&self,i:usize)->bool{self.0[i].flags&1!=0}
 fn selection_box(&self,i:usize)->bool{self.0[i].name.eq_ignore_ascii_case("SelectionBox")}
 fn keep_marker(&self,i:usize)->bool{self.0[i].extras.iter().any(|e|e["is_string"].as_bool()==Some(true)&&e["value"].as_str().is_some_and(|s|["fiducial","sgoKeep","NiOptimizeKeep","NiBoneLOD#"].iter().any(|m|s.contains(m))))}
 fn detach(&mut self,i:usize)->Result<()>{let parent=self.0[i].parent.ok_or("8C6F10 actual parent is NULL")?;let slot=self.0[parent].children.iter().position(|c|*c==Some(i)).ok_or("8C6F10 child is absent from actual parent")?;self.0[parent].children[slot]=None;self.0[i].parent=None;self.0[i].detached=true;Ok(())}
}
///Controller targets: the source objects' local transforms and material words.
struct ControllerTargets<'a>{nodes:&'a mut [Node],blocks:&'a BTreeMap<usize,usize>,properties:&'a mut BTreeMap<u32,Property>}
impl controllers::Targets for ControllerTargets<'_>{
 fn local(&mut self,block:usize)->Result<&mut [u32;13]>{let i=*self.blocks.get(&block).ok_or("controller target is not a source NiAVObject of this model")?;Ok(&mut self.nodes[i].local)}
 fn material(&mut self,block:usize)->Result<&mut [u32;14]>{let p=self.properties.get_mut(&(block as u32)).ok_or("controller target Property is not owned by this model")?;p.material.as_mut().ok_or_else(||"controller target is not a MaterialProperty".into())}
}
struct Propagating<'a>(&'a mut Selected);
impl property_state::Tree<PropertySource> for Propagating<'_>{
 fn children(&self,i:usize)->&[Option<usize>]{&self.0.graph.nodes[i].children}
 fn own_count(&self,i:usize)->usize{self.0.graph.nodes[i].property_blocks.len()}
 fn own(&self,i:usize,j:usize)->Result<(usize,PropertySource)>{let block=self.0.graph.nodes[i].property_blocks[j];let property=self.0.properties.get(&block).ok_or("actual own Property allocation absent")?;Ok((PropertyKind::from_name(&property.kind)?.index(),PropertySource::Source(block)))}
 fn geometry(&self,i:usize)->bool{Selected::geometry(&self.0.graph.nodes[i])}
 fn retain(&mut self,i:usize,state:Arc<[PropertySource;12]>)->Result<()>{self.0.graph.nodes[i].property_state=Some(state);Ok(())}
}
struct Preprocessed<'a,'p>{model:&'a mut Selected,policy:&'p mut dyn MetadataPolicy}
impl preprocessed::Root for Preprocessed<'_,'_>{
 fn integer_version(&self)->Result<Option<u32>>{let extras=&self.model.graph.nodes[self.model.root].extras;if extras.iter().filter(|e|e["extra_name"]=="ppdv").count()>1{return Err("878F30 duplicate ppdv lookup requires actual sorted extra ownership".into())}let Some(extra)=extras.iter().find(|e|e["extra_name"]=="ppdv"&&e["kind"]=="NiIntegerExtraData")else{return Ok(None)};let raw=extra["integer"].as_i64().ok_or("actual ppdv integer source field absent")?;if raw<i32::MIN as i64||raw>u32::MAX as i64{return Err("actual ppdv integer escapes native source word".into())}Ok(Some(raw as u32))}
 fn reuse_binary_metadata(&mut self)->Result<bool>{Err("878F30 valid source ppdv7 requires actual serialized binary metadata reuse".into())}
 fn remove_version(&mut self)->Result<()>{self.model.graph.nodes[self.model.root].extras.retain(|e|e["extra_name"]!="ppdv");Ok(())}
 fn metadata_named(&self)->Result<bool>{let extras=&self.model.graph.nodes[self.model.root].extras;if extras.iter().filter(|e|e["extra_name"]=="ppd").count()>1{return Err("878F30 duplicate ppd removal requires actual sorted extra ownership".into())}Ok(extras.iter().any(|e|e["extra_name"]=="ppd"))}
 fn remove_metadata(&mut self)->Result<()>{self.model.graph.nodes[self.model.root].extras.retain(|e|e["extra_name"]!="ppd");Ok(())}
 fn warning_current(&mut self,old:Option<u32>)->Result<u8>{if old.is_some(){self.policy.preprocessed_data_wrong_version_warning_current()}else{self.policy.preprocessed_data_missing_warning_current()}}
 fn warning(&mut self,old:Option<u32>)->Result<()>{self.policy.preprocessed_data_warning(&self.model.filename,old)}
}
impl Selected{
 fn prepare_native_metadata_graph(&mut self)->Result<()>{
  self.update()?;
  for i in self.order(){self.supported(i)?;if self.graph.nodes[i].effect_blocks.as_ref().ok_or("actual source effect list is unbound")?.iter().any(Option::is_some){return Err("actual source dynamic-effect processing is required".into())}}
  hidden_tree::remove_hidden(&mut Hidden(&mut self.graph.nodes),self.root)?;
  //87ABC0: NiAVObject::Update(0.0, 1), the first update with controllers.
  self.update_with(true)?;
  let mut defaults=Vec::with_capacity(12);for kind in [PropertyKind::Alpha,PropertyKind::Dither,PropertyKind::Fog,PropertyKind::Material,PropertyKind::RendererSpecific,PropertyKind::Shade,PropertyKind::Specular,PropertyKind::Stencil,PropertyKind::Texturing,PropertyKind::VertexColor,PropertyKind::Wireframe,PropertyKind::ZBuffer]{defaults.push(PropertySource::Default(self.source()?.default_property(kind)?));}
  let defaults:[PropertySource;12]=defaults.try_into().map_err(|_|"actual default Property census differs")?;let root=self.root;
  property_state::propagate(&mut Propagating(self),root,defaults)?;
  //86F090 starts at this actual parentless selected root. Its scratch is
  //cleared before traversal; every admitted source node has an explicitly
  //empty effect list. Thus the inherited effect state is NULL at each node.
  //A positive source effect is refused above, never replaced by emptiness.
  for i in self.order(){self.graph.nodes[i].effects_updated=true;}
  self.update()?;
  Ok(())
 }
 fn source(&self)->Result<&SourceContext>{self.source.as_ref().ok_or_else(||"model requires same preWorld canonical Bone source context".into())}
 fn geometry(n:&Node)->bool{matches!(n.kind.as_str(),"NiTriShape"|"NiTriStrips")}
 fn supported(&self,i:usize)->Result<()>{let n=&self.graph.nodes[i];if n.skin{return Err(format!("selected NIF actual {} skin graph requires skin transforms",n.name))}if !matches!(n.kind.as_str(),"NiNode"|"NiTriShape"|"NiTriStrips"){return Err(format!("selected NIF actual {} {} graph processing is required",n.kind,n.name))}Ok(())}
 fn order(&self)->Vec<usize>{let mut out=vec![];let mut work=vec![self.root];while let Some(i)=work.pop(){if self.graph.nodes[i].detached{continue}out.push(i);work.extend(self.graph.nodes[i].children.iter().rev().flatten().copied());}out}
 fn geometry_data(&self,i:usize)->Result<&Geometry>{let block=self.graph.nodes[i].data_block.ok_or("actual GeometryData source pointer is NULL")?;self.data.get(&block).ok_or_else(||"actual GeometryData allocation is absent".into())}
 //86EA70 reads THIS object's A4 list. It does not search its parents.
 fn property(&self,i:usize,kind:&str)->Option<&Property>{self.graph.nodes[i].property_blocks.iter().filter_map(|b|self.properties.get(b)).find(|p|p.kind==kind)}
 fn tagged(&self,i:usize,name:&str)->bool{let n=&self.graph.nodes[i];n.generated.contains_key(name)||n.extras.iter().any(|e|e["extra_name"]==name||(e["is_string"]==true&&e["value"].as_str().is_some_and(|value|value.eq_ignore_ascii_case(name))))}
 fn process_own_material_metadata(&mut self,i:usize)->Result<()>{
  //8790B0 does nothing for the admitted empty StringExtraData values. A
  //positive value needs its real line/tag conversion before property work.
  if self.graph.nodes[i].extras.iter().any(|e|e["is_string"]==true&&e["value"].as_str().is_none_or(|value|!value.is_empty())){return Err("8790B0 nonempty source StringExtraData conversion is required".into())}
  let name=&self.graph.nodes[i].name;
  if self.graph.nodes[i].parent.is_some()&&(["Camera","Biped Object","Cull"].iter().any(|prefix|name.get(..prefix.len()).is_some_and(|text|text.eq_ignore_ascii_case(prefix)))||name.get(name.len().saturating_sub(9)..).is_some_and(|text|text.eq_ignore_ascii_case("Footsteps"))){return Err("879A90 named source helper requires actual878540 reference-safe removal".into())}
  let material=self.property(i,"NiMaterialProperty").map(|p|p.block);let words=material.map(|block|self.properties[&block].material.ok_or("same MaterialProperty typed source words are absent")).transpose()?;
  let alpha=self.property(i,"NiAlphaProperty").map(|p|->Result<(u16,u8)>{Ok((p.flags as u16,p.alpha_threshold.ok_or("same AlphaProperty typed threshold absent")?))}).transpose()?;
  if alpha.is_some_and(|(flags,_)|flags&1!=0){self.work.as_mut().ok_or("actual metadata allocation is absent")?.blend=true;}
  if let Some(block)=material{let p=self.properties.get_mut(&block).ok_or("same MaterialProperty allocation absent")?;if !p.extras.iter().any(|e|e["extra_name"]=="omi")&&!p.generated.contains_key("omi"){p.generated.insert("omi".into(),Extra::Material(property_metadata::material(words.ok_or("Material words disappeared")?)));}}
  let node=&mut self.graph.nodes[i];if !node.extras.iter().any(|e|e["extra_name"]=="opi")&&!node.generated.contains_key("opi"){node.generated.insert("opi".into(),Extra::Opacity(property_metadata::opacity(words.as_ref(),alpha)));}Ok(())
 }
 fn process_vertex_color(&mut self,i:usize,policy:&mut dyn MetadataPolicy)->Result<()>{
  let Some(property)=self.property(i,"NiVertexColorProperty")else{return Ok(())};let block=property.block;let[mode,lighting]=property.vertex_color.ok_or("same VertexColorProperty typed source fields absent")?;
  match metadata_flow::vertex_color(mode,lighting,self.graph.nodes[i].parent.is_some()){
   metadata_flow::VertexColor::Keep=>Ok(()),metadata_flow::VertexColor::KeepDiagnostic=>policy.graph_diagnostic(&self.filename,&self.graph.nodes[i].name,0x410),
   metadata_flow::VertexColor::RemoveDiagnostic=>{let at=self.graph.nodes[i].property_blocks.iter().position(|actual|*actual==block).ok_or("same VertexColorProperty list entry absent")?;self.graph.nodes[i].property_blocks.remove(at);policy.graph_diagnostic(&self.filename,&self.graph.nodes[i].name,0x41d)}
  }
 }
 ///872650(object, 8785D0, metadata) of 879A90 (KKC_NiProcessing::
 ///ProcessTimeControllers): the own chain, each Property chain, then each
 ///effect chain; an APP_INIT controller is reported (line 0x64E) and made an
 ///active APP_TIME controller; metadata +4 becomes the largest stop time.
 fn visit_controllers(&mut self,i:usize,policy:&mut dyn MetadataPolicy)->Result<()>{
  let Some(block)=self.graph.nodes[i].source_block else{if self.graph.nodes[i].controllers.is_empty(){return Ok(())}return Err("872650 controllers of a node without a source object".into())};
  let properties:Vec<usize>=self.graph.nodes[i].property_blocks.iter().map(|&b|b as usize).collect();
  let effects:Vec<usize>=self.graph.nodes[i].effect_blocks.as_ref().ok_or("872650 actual effect list unbound")?.iter().flatten().map(|&b|b as usize).collect();
  let mut runtime=match self.controllers.take(){Some(r)=>r,None=>controllers::Runtime::new(self.pc)?};
  let mut stop=self.work.as_ref().ok_or("actual metadata allocation is absent")?.stop;let filename=self.filename.clone();
  let result=runtime.visit_metadata(&self.nif,block as usize,&properties,&effects,&mut stop,&mut |text|policy.graph_diagnostic(&filename,text,0x64e));
  //The source projection's controller flags follow the native words.
  if let Ok(chain)=self.nif.controllers(block as usize){for(c,k)in self.graph.nodes[i].controllers.iter_mut().zip(chain){if let Some(s)=runtime.state(k){c.flags=s.flags as u32}}}
  for &p in &properties{if let Some(property)=self.properties.get_mut(&(p as u32)){for c in property.controllers.iter_mut().flatten(){if let Some(s)=runtime.state(c.block as usize){c.flags=s.flags as u32}}}}
  self.controllers=Some(runtime);result?;
  self.work.as_mut().ok_or("actual metadata allocation is absent")?.stop=stop;Ok(())
 }
 fn update(&mut self)->Result<()>{self.update_with(false)}
 ///86E8D0(0.0, controllers): in preorder, 866BE0 (when `controllers`) runs
 ///the Property chains and the own chain of each source object before its
 ///world transform 8A2C70; then the bounds.
 fn update_with(&mut self,controllers:bool)->Result<()>{
  let mut runtime=if controllers{Some(match self.controllers.take(){Some(r)=>r,None=>controllers::Runtime::new(self.pc)?})}else{None};
  let result=self.update_inner(runtime.as_mut());
  if controllers{self.controllers=runtime;}
  result
 }
 fn update_inner(&mut self,mut runtime:Option<&mut controllers::Runtime>)->Result<()>{
  let order=self.order();
  let blocks:BTreeMap<usize,usize>=if runtime.is_some(){self.graph.nodes.iter().enumerate().filter_map(|(i,n)|n.source_block.map(|b|(b as usize,i))).collect()}else{BTreeMap::new()};
  for &i in &order{self.supported(i)?;
   if let(Some(runtime),Some(block))=(runtime.as_deref_mut(),self.graph.nodes[i].source_block){
    let properties:Vec<usize>=self.graph.nodes[i].property_blocks.iter().map(|&b|b as usize).collect();
    let mut targets=ControllerTargets{nodes:&mut self.graph.nodes,blocks:&blocks,properties:&mut self.properties};
    runtime.update_object(&self.nif,block as usize,&properties,0,&mut targets)?;
   }
   let n=&self.graph.nodes[i];let world=match n.parent{Some(p)=>math::compose_world(self.graph.nodes[p].world.ok_or("NIF parent transform is not current")?,n.local,self.pc)?,None=>n.local};let sphere=if Self::geometry(n){self.geometry_data(i)?.sphere.map(|sphere|math::transform_sphere(&world,sphere,self.pc)).transpose()?}else{None};self.graph.nodes[i].world=Some(world);self.graph.nodes[i].world_sphere=sphere;}
  for &i in order.iter().rev(){if Self::geometry(&self.graph.nodes[i]){continue}let children=self.graph.nodes[i].children.clone();let mut sphere=None;for child in children.into_iter().flatten(){let c=&self.graph.nodes[child];if c.detached{continue}if let Some(s)=c.world_sphere{if f32::from_bits(s[3])==0.0{continue}if let Some(current)=sphere.as_mut(){math::merge_sphere(current,s,MERGE_EPSILON,self.pc)?}else{sphere=Some(s)}}}self.graph.nodes[i].world_sphere=sphere;}Ok(())
 }
 fn find_name(&self,name:&[u16])->Option<usize>{self.order().into_iter().find(|&i|self.graph.nodes[i].name.encode_utf16().eq(name.iter().copied()))}
 ///879A90 texture maps of THIS object's NiTexturingProperty (86EA70(8)):
 ///the base map; then, when +0x24 is set (86CF20: any map after the base
 ///is present), maps 5 (bump), 1, 2, 3, 4, the decal maps 6.. of the map
 ///array (+0x2C/+0x34) and every shader map (+0x44), each through 8792F0.
 fn collect_node_textures(&mut self,i:usize)->Result<()>{
  let Some(p)=self.property(i,"NiTexturingProperty")else{return Ok(())};
  let mut maps:Vec<Option<Descriptor>>=vec![p.slots.first().cloned().flatten()];
  if p.slots.iter().skip(1).any(Option::is_some){
   //87A63E..87A6C5 read the map data at indices 1..=5 without a size test.
   if p.slots.len()<6{return Err("87A63E reads maps past a native map array shorter than 6".into())}
   for k in [5,1,2,3,4]{maps.push(p.slots[k].clone());}
   maps.extend(p.slots[6..].iter().cloned());
   maps.extend(p.shader_maps.iter().cloned());
  }
  for map in maps{self.collect_texture(map.as_ref())?;}
  Ok(())
 }
 ///8792F0 for one map: a NULL map, a NULL texture or a texture that is not
 ///an NiSourceTexture (711540 B43F24) is skipped; the file name is counted in
 ///the model's texture-name vector (_stricmp, 970817) and the SourceTexture
 ///gets its "otn" NiStringExtraData once. This object is never NiParticles
 ///(B43CE0) in the processed domain, so the particle count stays 0.
 fn collect_texture(&mut self,map:Option<&Descriptor>)->Result<()>{
  let Some(source)=map.and_then(|d|d.source)else{return Ok(())};
  if self.nif.objects.get(source as usize).ok_or("8792F0 texture link escapes the object array")?.class!="NiSourceTexture"{return Ok(())}
  let texture=self.texture_objects.get_mut(&source).ok_or("actual SourceTexture owner is absent")?;let name=texture.filename.as_ref().ok_or("8792F0 actual internal-texture diagnostic is required")?;if !name.is_ascii(){return Err("8792F0 actual narrow texture filename conversion is required".into())}let name=name.encode_utf16().collect::<Vec<_>>();
  if let Some(row)=self.texture_rows.iter_mut().find(|row|ascii_equal(&row.name,&name)){row.total=row.total.wrapping_add(1)}else{self.texture_rows.try_reserve(1).map_err(|e|e.to_string())?;self.texture_rows.push(TextureUse{name:name.clone(),total:1,particles:0});}
  if !texture.extras.iter().any(|v|v["extra_name"]=="otn"||(v["is_string"]==true&&v["value"].as_str().is_some_and(|value|value.eq_ignore_ascii_case("otn"))))&&!texture.generated.contains_key("otn"){texture.generated.insert("otn".into(),Extra::Text(name));}
  Ok(())
 }
 fn attach_base_texture(&mut self)->Result<()>{let names=self.texture_rows.iter().map(|row|row.name.clone()).collect::<Vec<_>>();let used=self.texture_rows.iter().map(|row|row.total!=row.particles).collect::<Vec<_>>();let selected=select_base_texture(&self.filename,&names,&used)?;
  for i in self.order(){let Some(p)=self.property(i,"NiTexturingProperty")else{continue};let source=p.slots.first().and_then(Option::as_ref).and_then(|d|d.source);let name=source.map(|b|self.texture_objects.get(&b).ok_or("base descriptor SourceTexture allocation absent")).transpose()?.and_then(|t|t.filename.as_ref());
   let block=p.block;
   let operand=match(source,name){(None,_)=>TextureOperand::Absent,(Some(_),None)=>TextureOperand::Internal,(_,Some(name))=>TextureOperand::External(name.encode_utf16().collect())};
   let property=self.properties.get_mut(&block).ok_or("878170 same Property allocation was retired")?;apply_base_texture(property,&operand,selected.as_deref(),false)?;
  }Ok(())
 }
}
fn ascii_equal(a:&[u16],b:&[u16])->bool{a.len()==b.len()&&a.iter().zip(b).all(|(a,b)|a==b||(*a<=127&&*b<=127&&(*a as u8).eq_ignore_ascii_case(&(*b as u8))))}
enum TextureOperand{Absent,Internal,External(Vec<u16>)}
fn apply_base_texture(property:&mut Property,texture:&TextureOperand,selected:Option<&[u16]>,particles:bool)->Result<()>{
 if particles{return Ok(())}
 if property.extras.iter().filter(|v|v["extra_name"]=="bt").count()>1{return Err("878170 duplicate named bt removal order requires actual alias owner".into())}
 property.extras.retain(|v|v["extra_name"]!="bt");property.generated.remove("bt");
 let qualifies=match texture{TextureOperand::Absent=>selected.is_none(),TextureOperand::Internal=>true,TextureOperand::External(name)=>selected.is_none_or(|s|ascii_equal(s,name))};
 if qualifies{property.generated.insert("bt".into(),Extra::Boolean(1));}Ok(())
}
//87A820 static-unskinned domain: every collected texture has ordinary uses
//and no NiParticles uses. The native filename, never selected disk spelling,
//determines the preferred basename; last matching *base fallback is retained.
fn select_base_texture(filename:&[u16],textures:&[Vec<u16>],used:&[bool])->Result<Option<Vec<u16>>>{
 if used.len()!=textures.len(){return Err("actual native texture-use vector differs".into())}
 if textures.is_empty(){return Ok(None)}if textures.len()==1&&used[0]{return Ok(Some(textures[0].clone()))}
 let filename=String::from_utf16(filename).map_err(|e|e.to_string())?;if !filename.is_ascii(){return Err("87A820 native wide-to-narrow filename conversion is required".into())}
 let name=filename.rsplit(['\\','/',':']).next().ok_or("actual Model filename is absent")?;let stem=name.rsplit_once('.').map_or(name,|v|v.0);let mut fallback=None;
 for (texture,used) in textures.iter().zip(used){if !used{continue}let name=String::from_utf16(texture).map_err(|e|e.to_string())?;if !name.is_ascii(){return Err("87A820 non-ASCII texture-name comparison is required".into())}
  if name.len()>=stem.len()&&name[..stem.len()].eq_ignore_ascii_case(stem)&&((name.len()==stem.len()+8&&name[stem.len()..stem.len()+5].eq_ignore_ascii_case("base."))||(name.len()==stem.len()+4&&name.as_bytes()[stem.len()]==b'.')){return Ok(Some(texture.clone()))}
  if let Some(dot)=name.rfind('.') {if dot>4&&name[dot-4..dot].eq_ignore_ascii_case("base"){fallback=Some(texture.clone())}}
 }Ok(fallback)
}
impl Model for Selected{
 fn source_sha256(&self)->&str{&self.source_sha}
 fn backend_data(&self)->Result<&(dyn std::any::Any+Send+Sync)>{Ok(self)}
 fn process_metadata_with_policy(&mut self,has_skin:bool,policy:&mut dyn MetadataPolicy)->Result<Metadata>{
  if self.processing||self.metadata.is_some(){return Err("selected NIF retains a prior metadata processing call; actual retry/cleanup is required".into())}
  self.processing=true;
  preprocessed::lookup(&mut Preprocessed{model:self,policy})?;
  self.work=Some(MetadataWork{bounds:math::Bounds::empty_processing(),stop:0,blend:false});
  if has_skin{return Err("87AA10 has-skin unique texture/clone path requires concrete skin graph".into())}
  self.prepare_native_metadata_graph()?;
  self.update()?;let mut contexts:Vec<Option<metadata_flow::Context>>=vec![None;self.graph.nodes.len()];
  for i in self.order(){self.supported(i)?;
   //Only these source-normalized material tags are admitted by the existing
   //paired audit. Other positive process tags must gain their real operation.
   if self.graph.nodes[i].tags.iter().any(|t|!matches!(t.as_str(),"RepColor"|"Sg"|"Sg2"|"Eq")){return Err("87AA10 positive non-material graph tag requires its actual processing child".into())}
   self.process_own_material_metadata(i)?;self.process_vertex_color(i,policy)?;
   let own_sg=self.tagged(i,"Sg");let own_sg_two=self.tagged(i,"Sg2");let replacement=self.tagged(i,"RepColor");
   let inherited=self.graph.nodes[i].parent.map(|parent|contexts[parent].ok_or("native metadata parent context is absent")).transpose()?.unwrap_or_default();
   let(context,add_replacement)=metadata_flow::enter(inherited,i,Self::geometry(&self.graph.nodes[i]),own_sg,own_sg_two,replacement);contexts[i]=Some(context);
   if context.shadow&&Self::geometry(&self.graph.nodes[i]){let version=self.graph.nodes[i].extras.iter().find(|e|e["extra_name"]=="Sgv"&&e["kind"]=="NiIntegerExtraData").and_then(|e|e["integer"].as_i64());if version!=Some(1){return Err("879A90 positive shadow geometry needs actual8CF490 conversion/Sgv diagnostic".into())}}
   if Self::geometry(&self.graph.nodes[i]){
    let remove=policy.renderer_remove_normals_current()?;if remove!=0{let block=self.graph.nodes[i].data_block.ok_or("87A1E5 actual GeometryData pointer is NULL")?;let data=self.data.get_mut(&block).ok_or("87A1E5 actual GeometryData allocation absent")?;data.normals=None;let fields=data.fields.as_mut().ok_or("8ABEC0 actual GeometryData fields unbound")?;if fields.vertices_present{if data.vertices.is_empty(){return Err("8ABEC0 empty GeometryData sphere needs actual vertex-pointer/global-center owners".into())}data.sphere=Some(math::sphere_from_vertices(&data.vertices,None,self.pc)?);}fields.flags=metadata_flow::replacement_flags(fields.flags,1,0);}
    if !context.shadow&&!self.source()?.remove_base_mapping_current()?{let fields=self.geometry_data(i)?.fields.as_ref().ok_or("actual TriBasedGeometryData source fields absent")?;let triangles=fields.triangles.ok_or("actual TriBasedGeometryData triangle count unbound")?;if fields.count as u32>2*triangles as u32{policy.graph_diagnostic(&self.filename,&self.graph.nodes[i].name,0x4ab)?;}}
    let n=&self.graph.nodes[i];if !context.shadow{let block=n.data_block.ok_or("878630 actual GeometryData pointer NULL")?;let data=self.data.get(&block).ok_or("878630 actual GeometryData allocation absent")?;let fields=data.fields.as_ref().ok_or("878630 actual GeometryData fields absent")?;if !fields.vertices_present&&fields.count!=0{return Err("878630 positive count with actual NULL vertex pointer requires native invalid-memory boundary".into())}self.work.as_mut().ok_or("actual metadata allocation absent")?.bounds.include_geometry(&n.world.ok_or("raw NIF world transform absent")?,&data.vertices,n.flags&1!=0,self.pc)?;}
   }
   if context.shadow{if replacement{policy.graph_diagnostic(&self.filename,&self.graph.nodes[i].name,0x4ca)?;}}else if context.replacement_owner.is_some(){
    if add_replacement{self.graph.nodes[i].generated.insert("RepColor".into(),Extra::Boolean(1));}
    if let Some(block)=self.property(i,"NiAlphaProperty").map(|p|p.block){for(bit,line)in[(1,0x4e4),(0x200,0x4ea)]{if self.properties[&block].flags&bit!=0{policy.graph_diagnostic(&self.filename,&self.graph.nodes[i].name,line)?;self.properties.get_mut(&block).ok_or("actual AlphaProperty disappeared")?.flags&=!bit;}}}
   }
   if self.tagged(i,"tred"){return Err("879A90 tred inheritance requires actual73CD60 geometry predicate".into())}
   self.visit_controllers(i,policy)?;
   if self.tagged(i,"Eq"){return Err("879A90 positive Eq inheritance requires actual871B40 child".into())}
   self.collect_node_textures(i)?;
  }
  if let Some(height)=self.work.as_mut().ok_or("actual metadata allocation absent")?.bounds.begin_footprint(){
   for i in self.order(){let n=&self.graph.nodes[i];if !Self::geometry(n){continue}let mut ancestor=Some(i);let mut equipment=false;while let Some(j)=ancestor{equipment|=self.tagged(j,"Eq");ancestor=self.graph.nodes[j].parent;}if !equipment{let block=n.data_block.ok_or("878790 actual GeometryData pointer NULL")?;let data=self.data.get(&block).ok_or("878790 actual GeometryData allocation absent")?;self.work.as_mut().ok_or("actual metadata allocation absent")?.bounds.include_footprint_at_height(&n.world.ok_or("raw NIF world transform absent")?,&data.vertices,n.flags&1!=0,height,FOOTPRINT_THRESHOLD,self.pc)?;}}
  }
  self.attach_base_texture()?;
  let names=Arc::new(self.texture_rows.iter().map(|row|row.name.clone()).collect::<Vec<_>>());
  let work=self.work.as_ref().ok_or("actual metadata allocation absent")?;let bounds=&work.bounds;
  let m=Metadata{byte1:0,word4:work.stop,radial8:bounds.radial,max_z0c:bounds.max_z,min_z10:bounds.min_z,footprint14:bounds.footprint[0],footprint18:bounds.footprint[1],footprint1c:bounds.footprint[2],texture_count20:u32::try_from(names.len()).map_err(|e|e.to_string())?,byte24:u8::from(work.blend)};
  let metadata=Arc::new(m);let n=&mut self.graph.nodes[self.root];
  for name in ["ppd","ppdv","pptn"]{if n.extras.iter().filter(|v|v["extra_name"]==name).count()>1{return Err("8780A0 duplicate processed extra needs actual remove-first ownership".into())}n.extras.retain(|v|v["extra_name"]!=name);}
  n.generated.insert("ppdv".into(),Extra::Integer(7));n.generated.insert("ppd".into(),Extra::Processed(Arc::clone(&metadata)));
  if !names.is_empty(){n.generated.insert("pptn".into(),Extra::TextureNames(Arc::clone(&names)));self.texture_names=Some(names);}
  self.metadata=Some(Arc::clone(&metadata));self.work=None;self.processing=false;Ok((*metadata).clone())
 }
 fn texture_names(&self)->Result<Vec<Vec<u16>>>{self.metadata.as_ref().ok_or("native model metadata has not completed")?;Ok(self.texture_names.as_ref().map(|names|names.as_ref().clone()).unwrap_or_default())}
 fn prepare_effect(&mut self,code:u32)->Result<()>{match code{0x871bc0=>{for i in self.order(){if Self::geometry(&self.graph.nodes[i])&&!self.graph.nodes[i].tags.iter().any(|s|s=="Mod"){self.graph.nodes[i].tags.push("Mod".into())}}},4|5=>{for i in self.order(){self.graph.nodes[i].mode_c0=Some(code)}},0x871b40=>{self.graph.nodes[self.root].tags.push("Eq".into())},_=>return Err("unsupported actual model graph preparation code".into())}Ok(())}
 fn controllers(&self)->Result<bool>{let name=self.source()?.first_controller_bone_name()?;Ok(descriptor_slot_three(&self.graph.nodes,&self.properties,self.find_name(name)))}
 fn selection_cuboid(&mut self)->Result<[u32;6]>{
  let name=self.source()?.selection_box_name()?.ok_or("5A94F0 missing Bone selection_box fallback CString owner is required")?.to_vec();self.update()?;
  let Some(i)=self.find_name(&name)else{return Err("5A94F0 actual BoundingSphere selection fallback/diagnostic child is required".into())};
  if !Self::geometry(&self.graph.nodes[i]){return Err("5A94F0 non-Geometry SelectionBox requires native sphere fallback".into())}
  let n=&self.graph.nodes[i];let out=math::selection_geometry(&n.world.ok_or("selection world transform absent")?,&self.geometry_data(i)?.vertices,self.pc)?;
  let parent=n.parent.ok_or("6FF850 actual selection parent is NULL")?;let slot=self.graph.nodes[parent].children.iter().position(|c|*c==Some(i)).ok_or("selection child is not in same parent")?;
  self.graph.nodes[parent].children[slot]=None;self.graph.nodes[i].parent=None;self.graph.nodes[i].detached=true;Ok(out)
 }
 fn selection_5a94f0(&mut self)->Result<aa_runtime::resource_model_payload::Selection>{selection::compute_5a94f0(self)}
 fn wrap_root(&mut self)->Result<()>{if self.wrapped{return Err("867690 model already wrapped".into())}let old=self.root;let new=self.graph.nodes.len();let name=self.graph.nodes[old].name.clone();let dcss=self.graph.nodes[old].extras.iter().any(|v|v["extra_name"]=="dcss");
  self.graph.nodes.try_reserve(1).map_err(|e|e.to_string())?;
  let extras=if dcss{vec![serde_json::json!({"extra_name":"dcss","is_string":false,"value":""})]}else{vec![]};
  self.graph.nodes.push(Node{kind:"NiNode".into(),name:name.clone(),parent:None,hierarchy:vec![name],local:IDENTITY,flags:0x30,children:vec![Some(old)],vertices:vec![],normals:vec![],sphere:None,skin:false,tags:vec![],properties:vec![],controllers:vec![],extras,has_normals:false,data_block:None,property_blocks:vec![],generated:BTreeMap::new(),world:None,world_sphere:None,mode_c0:Some(0),detached:false,source_block:None,effect_blocks:Some(vec![]),property_state:None,effects_updated:false});
  self.graph.nodes[old].name="base".into();self.graph.nodes[old].parent=Some(new);self.root=new;self.wrapped=true;Ok(())
 }
 fn update_root(&mut self)->Result<()>{if !self.wrapped{return Err("model root wrapper is not installed".into())}self.update_with(true)?;self.updated=true;Ok(())}
}
///A Send rendering packet is derived from the SAME source40 allocation after
///its graph processing/wrapper/update. This never rereads converted geometry.
pub(crate) fn render_model(payload:&Payload)->Result<PreparedModel>{payload.with_backend_data::<Selected,_>(|model|{
 if model.metadata.is_none()||!model.wrapped||!model.updated{return Err("render packet requires the same completed Model graph processing/root update".into())}Ok(model.render.clone())
})}
pub(crate) fn render_scene(payload:&Payload,store:&AssetStore,source:&str,settings:crate::loading::LoadSettings)->anyhow::Result<(crate::gpu::CpuScene,aa_assets::Bounds)>{
 payload.with_backend_data::<Selected,_>(|model|{
  if model.metadata.is_none()||!model.wrapped||!model.updated{return Err("render packet requires the same completed Model graph processing/root update".into())}
  if !model.graph.source.eq_ignore_ascii_case(&source.replace('\\',"/")){return Err("render packet source differs from its actual selected Model allocation".into())}
  crate::loading::load_retained_model(store,source,&model.render,settings).map_err(|e|e.to_string())
 }).map_err(anyhow::Error::msg)
}
fn descriptor_slot_three(nodes:&[Node],properties:&BTreeMap<u32,Property>,selected:Option<usize>)->bool{
 let Some(i)=selected else{return false};
 std::iter::once(i).chain(nodes[i].children.iter().flatten().copied()).any(|i|nodes[i].property_blocks.iter().filter_map(|b|properties.get(b)).find(|p|p.kind=="NiTexturingProperty").is_some_and(|p|p.slots.get(3).is_some_and(Option::is_some)))
}
#[cfg(test)]mod tests{
 use super::*;
 use std::path::PathBuf;
 struct HeadlessBackend<'a>{capability:crate::live_scene::model_policy::Capabilities,variables:&'a aa_ui::uvars::UVars,avars:&'a aa_ui::resources::AVars,store:&'a AssetStore,pc:Precision,timer:Option<crate::windows_reset_os::PerformanceTimer>}
 impl aa_runtime::resource_objects::ModelPolicy for HeadlessBackend<'_>{fn use_hardware_model_73a560(&mut self)->Result<bool>{self.capability.use_hardware_model_current(self.variables)}}
 impl MetadataPolicy for HeadlessBackend<'_>{fn renderer_remove_normals_current(&mut self)->Result<u8>{self.variables.renderer_remove_normals_current()}fn preprocessed_data_missing_warning_current(&mut self)->Result<u8>{self.avars.preprocessed_data_missing_warning_current()}fn preprocessed_data_wrong_version_warning_current(&mut self)->Result<u8>{self.avars.preprocessed_data_wrong_version_warning_current()}}
 impl aa_runtime::model_resource_load::Backend for HeadlessBackend<'_>{
  fn resource_no_ni_objects_current(&mut self)->Result<u8>{self.variables.resource_no_ni_objects_current()}
  fn begin_model_load(&mut self,_:&aa_runtime::resource_objects::Handle)->Result<()>{crate::windows_reset_os::PerformanceTimer::initialize_frequency()?;self.timer=Some(crate::windows_reset_os::PerformanceTimer::from_native_constructor().map_err(|e|e.to_string())?);Ok(())}
  fn decode_selected_model(&mut self,input:&Input)->Result<Payload>{prepare(input,self.store,self.pc)}
  fn process_model_metadata(&mut self,payload:&Payload,skin:bool)->Result<Metadata>{payload.process_metadata_with_policy(skin,self)}
  fn skip_loaded_model_bounds(&mut self)->Result<u8>{self.variables.resource_caching_current()}
 }
 fn renderer()->crate::gpu::SceneRenderer{
  let instance=wgpu::Instance::new(&wgpu::InstanceDescriptor{backends:wgpu::Backends::DX12|wgpu::Backends::VULKAN,..Default::default()});
  let adapter=pollster::block_on(instance.request_adapter(&Default::default())).unwrap();
  let(device,queue)=pollster::block_on(adapter.request_device(&wgpu::DeviceDescriptor{required_limits:adapter.limits(),..Default::default()})).unwrap();
  let egui_renderer=egui_wgpu::Renderer::new(&device,wgpu::TextureFormat::Rgba8UnormSrgb,Default::default());
  crate::gpu::SceneRenderer::new(egui_wgpu::RenderState{adapter,available_adapters:Vec::new(),device,queue,target_format:wgpu::TextureFormat::Rgba8UnormSrgb,renderer:std::sync::Arc::new(eframe::egui::mutex::RwLock::new(egui_renderer))}).unwrap()
 }
 ///The load-time source graph against every PyFFI graph projection that
 ///used to be shipped beside the models (assets-cache/model-native-graphs):
 ///same nodes, preorder ordinals, hierarchy, words, child slots, properties,
 ///descriptors, controllers, extra data and source textures.
 #[test]fn native_graph_matches_every_former_projection(){
  let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");let folder=root.join("assets-cache/model-native-graphs");
  let Ok(index)=std::fs::read(folder.join("index.json"))else{eprintln!("skipping: former projections absent");return};
  let index:serde_json::Value=serde_json::from_slice(&index).unwrap();let data=root.join("../reconstruction/assets/original/Data");
  let(mut compared,mut kind_names,mut differ)=(0,0,Vec::new());
  for record in index["by_source"].as_object().unwrap().values(){
   let source=record["source"].as_str().unwrap();let Ok(raw)=std::fs::read(data.join(source))else{continue};
   let mut old:Document=serde_json::from_slice(&std::fs::read(folder.join(record["artifact"].as_str().unwrap())).unwrap()).unwrap();
   //PyFFI wrote "" where LoadCString 880EE0 stores NULL (a zero-length name).
   for n in &mut old.nodes{for p in &mut n.properties{for d in p.slots.iter_mut().flatten(){if d.texture.as_deref()==Some(""){d.texture=None}}}}
   let nif=aa_formats::nif::load(&raw).unwrap();
   let new=native_graph::document(&nif,source,&old.source_sha256,old.converted_sha256.clone(),old.material_audit_sha256.clone(),old.mesh_mapping.clone()).unwrap();
   //The projections kept only the seven named slots and no shader maps.
   let mut new=new;for n in &mut new.nodes{for p in &mut n.properties{if p.kind=="NiTexturingProperty"{if p.slots[7.min(p.slots.len())..].iter().any(Option::is_some){p.block=u32::MAX}p.slots.resize(7,None);p.shader_maps.clear();}}}
   let mut why=Vec::new();
   if old.schema!=2||new.version!=old.version||new.roots!=old.roots{why.push("schema/version/roots".to_string())}
   if new.nodes.len()!=old.nodes.len(){why.push(format!("{} nodes, projection {}",new.nodes.len(),old.nodes.len()))}
   for(k,(a,b))in new.nodes.iter().zip(&old.nodes).enumerate(){
    if a.kind!=b.kind{kind_names+=1;}
    let fields=[("name",a.name==b.name),("parent",a.parent==b.parent),("hierarchy",a.hierarchy==b.hierarchy),("local",a.local==b.local),("flags",a.flags==b.flags),("children",a.children==b.children),("vertices",a.vertices==b.vertices),("normals",a.normals==b.normals),("sphere",a.sphere==b.sphere),("skin",a.skin==b.skin),("tags",a.tags==b.tags),("properties",a.properties==b.properties),("extras",a.extras==b.extras),("has_normals",a.has_normals==b.has_normals),("data_block",a.data_block==b.data_block),
     ("controllers",a.controllers.len()==b.controllers.len()&&a.controllers.iter().zip(&b.controllers).all(|(x,y)|x.kind==y.kind&&x.flags==y.flags&&x.stop==y.stop))];
    let bad:Vec<&str>=fields.iter().filter(|f|!f.1).map(|f|f.0).collect();
    if !bad.is_empty(){why.push(format!("node {k} {}: {}",a.name,bad.join(",")));break}
   }
   if new.textures.len()!=old.textures.len()||new.textures.iter().zip(&old.textures).any(|(a,b)|a.block!=b.block||a.filename!=b.filename||a.extras!=b.extras){why.push("textures".into())}
   if why.is_empty(){compared+=1}else{differ.push(format!("{source}: {}",why.join("; ")))}
  }
  eprintln!("{compared} projections reproduced; {kind_names} nodes named by stream class in the projection");
  assert!(differ.is_empty(),"{} differ:
{}",differ.len(),differ.iter().take(30).cloned().collect::<Vec<_>>().join("
"));
 }
 #[test]fn base_texture_selector_and_same_property_mutation_match_native(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-model-resource-load/native-model-base-texture.json")).unwrap();
  let wide=|v:&serde_json::Value|v.as_str().unwrap().encode_utf16().collect::<Vec<_>>();
  let property=|initial:&serde_json::Value|Property{kind:"NiTexturingProperty".into(),block:17,flags:0,slots:vec![],shader_textures:0,shader_maps:vec![],extras:initial.as_array().unwrap().iter().map(|v|serde_json::json!({"extra_name":v[0],"kind":"NiBooleanExtraData","value":v[1]})).collect(),generated:BTreeMap::new(),material:None,alpha_threshold:None,vertex_color:None,controllers:None};
  let projection=|p:&Property|{let mut rows=p.extras.iter().map(|e|serde_json::json!({"name":e["extra_name"],"value":e["value"],"boolean_vtable":e["kind"]=="NiBooleanExtraData"})).collect::<Vec<_>>();rows.extend(p.generated.iter().map(|(name,v)|match v{Extra::Boolean(value)=>serde_json::json!({"name":name,"value":value,"boolean_vtable":true}),_=>panic!("unexpected base-texture extra")}));rows.sort_by(|a,b|a["name"].as_str().cmp(&b["name"].as_str()));serde_json::Value::Array(rows)};
  for row in fixture["rows"].as_array().unwrap(){match row["kind"].as_str().unwrap(){
   "choice"=>{let input=&row["input"];let names=input["names"].as_array().unwrap().iter().map(wide).collect::<Vec<_>>();let used=input["used"].as_array().unwrap().iter().map(|v|v.as_bool().unwrap()).collect::<Vec<_>>();let selected=select_base_texture(&wide(&input["file"]),&names,&used).unwrap();let expected=row["selected_index"].as_u64().map(|i|names[i as usize].clone());assert_eq!(selected,expected,"{input}");},
   "mutation"=>{let input=&row["input"];if !input["property"].as_bool().unwrap(){assert_eq!(row["node_before"],row["node_after"]);continue}let mut p=property(&input["initial"]);let texture=match &input["texture"]{serde_json::Value::Null=>TextureOperand::Internal,serde_json::Value::Bool(false)=>TextureOperand::Absent,v if v=="wrong-class"=>TextureOperand::Absent,v=>TextureOperand::External(wide(v))};let selected=input["selected"].as_str().map(|v|v.encode_utf16().collect::<Vec<_>>());apply_base_texture(&mut p,&texture,selected.as_deref(),input["kind"]=="particles").unwrap();assert_eq!(projection(&p),row["after"],"{input}");assert_eq!(row["node_before"],row["node_after"]);},
   "recursive"=>{let mut child=property(&serde_json::json!([]));let mut particles=property(&serde_json::json!([["bt",0]]));let selected="A.DDS".encode_utf16().collect::<Vec<_>>();let texture=TextureOperand::External("a.dds".encode_utf16().collect());apply_base_texture(&mut child,&texture,Some(&selected),false).unwrap();apply_base_texture(&mut particles,&texture,Some(&selected),true).unwrap();assert_eq!(projection(&child),row["child_property"]);assert_eq!(projection(&particles),row["particle_property"]);},
   "shared_property"=>{let mut p=property(&serde_json::json!([]));let selected="A.DDS".encode_utf16().collect::<Vec<_>>();let texture=TextureOperand::External("a.dds".encode_utf16().collect());for _ in 0..2{apply_base_texture(&mut p,&texture,Some(&selected),false).unwrap();}assert_eq!(projection(&p),row["left"]);assert_eq!(projection(&p),row["right"]);assert_eq!(p.generated.len(),1);},
   other=>panic!("unreplayed native base-texture case {other}")
  }}
 }
 #[test]fn first_bone_query_uses_own_property_array_slot_three_not_filter_or_controller(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-model-texturing-query.json")).unwrap();
  for row in fixture["rows"].as_array().unwrap(){let case=&row["input"];let mut properties=BTreeMap::new();let mut nodes=vec![];
   let mut append=|slots:&serde_json::Value|{let index=nodes.len();let block=index as u32;let mut n:Node=serde_json::from_value(serde_json::json!({"kind":"NiNode","name":"explicit native query operand","parent":null,"hierarchy":[],"local":IDENTITY,"flags":0,"children":[],"vertices":[],"sphere":null,"skin":false,"shadow":false,"team":false,"tags":[],"properties":[],"controllers":[],"extras":[],"data_block":null})).unwrap();
    if !slots.is_null(){n.property_blocks.push(block);properties.insert(block,Property{kind:"NiTexturingProperty".into(),block,flags:0,slots:slots.as_array().unwrap().iter().map(|v|if v.as_u64().unwrap()!=0{Some(Descriptor{source:None,texture:None,filter:0xdeadbeef})}else{None}).collect(),shader_textures:0,shader_maps:vec![],extras:vec![],generated:BTreeMap::new(),material:None,alpha_threshold:None,vertex_color:None,controllers:None});}nodes.push(n);index};
   append(&case["own"]);let children=case["children"].as_array().unwrap().iter().map(|slots|if slots.is_null(){None}else{Some(append(slots))}).collect();nodes[0].children=children;
   assert_eq!(descriptor_slot_three(&nodes,&properties,case["found"].as_bool().unwrap().then_some(0)),row["output"].as_bool().unwrap());
  }
 }
 #[test]
 #[ignore="requires actual original/v14 source files and completed Bank; explicit factory/policy lab, not a whole ResourceTask or GPU-ready claim"]
 fn actual_depot_oil_selected_source_graph_metadata_selection_and_root_all_precisions(){
  use aa_runtime::{catalog::Catalog,combat_source_receipt::VerifiedStartupReceipt,world_ready_services::Application};
  use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
  let game=PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
  let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
  let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
  catalog.settings=Some(aa_runtime::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
  let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
  aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();aa_runtime::combat_source_completion::compute(&catalog).unwrap();
  let startup=aa_runtime::actor_creation_startup::StartupInputs::from_verified_startup_reference(&receipt).unwrap();
  let output=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/development10/view-object-init/model-payload-lab");
  std::fs::create_dir_all(&output).unwrap(); //real portable work-depot precondition
  let mut app=Application::from_fresh_sources(&catalog,files,aa_runtime::application_mode::GameAppState::from_native_constructor(),startup,&output,aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
  let bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
  let store=AssetStore::open(game.join("reconstruction")).unwrap();let mut resources=app.depot_mut().resources();
  let root=aa_ui::fs::DataRoot::with_overlay(game.join("reconstruction/assets/original/Data"),game.join("reconstruction/native-game/runtime-expanded-v14/Data"));
  let mut tgi=aa_ui::tgi::TgiStore::new(root);let variables=aa_ui::uvars::UVars::load_with_view_scope(&mut tgi,aa_ui::view_variables::SourceOverrides::ControlledNone);
  let avars=aa_ui::resources::AVars::load(&mut tgi);
  let renderer=renderer();let mut backend=HeadlessBackend{capability:renderer.model_variant_capabilities(),variables:&variables,avars:&avars,store:&store,pc:Precision::Double53,timer:None};
  let ids="Buildings/GBR/GBRDepotOil/GBRDepotOil.nif".encode_utf16().collect::<Vec<_>>();
  let handle=resources.model_5a8520(&catalog,&bank,&ids,&mut backend).unwrap();let input=resources.model_input_4c(&handle).unwrap();
  assert_eq!(input.source_context().unwrap().first_controller_bone_name().unwrap(),"Scene Root".encode_utf16().collect::<Vec<_>>());
  assert_eq!(input.source_context().unwrap().selection_box_name().unwrap().unwrap(),"SelectionBox".encode_utf16().collect::<Vec<_>>());
  //The real same-Entry loader publishes B43E88 before metadata processing.
  //The following independent numeric payload calls read that retained owner;
  //they do not manufacture a caller-provided remove-base flag.
  assert!(resources.model_payload_40(&handle).unwrap().is_none());
  assert!(resources.load_model_4c(&handle,Precision::Double53,&mut backend).unwrap());
  let retained=resources.model_payload_40(&handle).unwrap().unwrap();retained.require_resource(&handle).unwrap();assert_eq!(retained.source_sha256(),input.sha256());
  let mut reports=vec![];
  for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
   let payload=prepare(&input,&store,pc).unwrap();payload.require_resource(&handle).unwrap();
   let metadata=payload.process_metadata_with_policy(false,&mut backend).unwrap();
   assert!(f32::from_bits(metadata.radial8)>0.0);assert_eq!(metadata.texture_count20,2);assert_eq!(metadata.word4,0);
   assert!(!payload.controllers().unwrap());let cuboid=payload.selection_cuboid().unwrap();
   let native:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-selected-depot-oil-v3.json")).unwrap();
   assert_eq!(native["source_sha256"],input.sha256());
   let native_pc=match pc{Precision::Single24=>"Single",Precision::Double53=>"Double",Precision::Extended64=>"Extended"};
   let expected=&native["rows"].as_array().unwrap().iter().find(|r|r["image"]=="original"&&r["precision"]==native_pc).unwrap()["output"];
   assert_eq!(serde_json::json!({"radial":metadata.radial8,"min_z":metadata.min_z10,"max_z":metadata.max_z0c,"footprint":[metadata.footprint14,metadata.footprint18,metadata.footprint1c],"cuboid":cuboid}),*expected,"actual selected source words must match original numeric children");
   payload.with_backend_data::<Selected,_>(|m|{
    let root=&m.graph.nodes[m.root];assert!(matches!(root.generated.get("ppdv"),Some(Extra::Integer(7))));
    let Some(Extra::Processed(data))=root.generated.get("ppd")else{panic!("actual root processed metadata extra absent")};assert!(Arc::ptr_eq(data,m.metadata.as_ref().unwrap()));
    let Some(Extra::TextureNames(names))=root.generated.get("pptn")else{panic!("actual root texture names extra absent")};assert!(Arc::ptr_eq(names,m.texture_names.as_ref().unwrap()));
    let tagged=m.properties.values().filter(|p|matches!(p.generated.get("bt"),Some(Extra::Boolean(1)))).map(|p|p.block).collect::<Vec<_>>();assert_eq!(tagged,[17]);
    //Two nodes share the SAME serialized Property17 and SourceTexture19;
    //the normal-removal field lives only in its actual GeometryData owner.
    assert_eq!(m.property(3,"NiTexturingProperty").unwrap().block,m.property(12,"NiTexturingProperty").unwrap().block);
    assert_eq!(m.texture_objects.len(),2);assert!(m.data.values().all(|data|data.normals.is_none()));
    Ok(())
   }).unwrap();
   assert!((0..3).all(|i|f32::from_bits(cuboid[i])<f32::from_bits(cuboid[3+i])));
   assert!(payload.selection_cuboid().is_err()); //same source selection child was really unlinked
   payload.prepare_effect(0x871bc0).unwrap();payload.prepare_effect(4).unwrap();payload.prepare_effect(0x871b40).unwrap();
   payload.wrap_root().unwrap();assert!(payload.wrap_root().is_err());payload.update_root().unwrap();
   assert!(!render_model(&payload).unwrap().meshes.is_empty());
   reports.push(serde_json::json!({"pc":format!("{pc:?}"),"radius":metadata.radial8,"max_z":metadata.max_z0c,"min_z":metadata.min_z10,"footprint":[metadata.footprint14,metadata.footprint18,metadata.footprint1c],"textures":payload.texture_names().unwrap(),"cuboid":cuboid,"source_sha256":payload.source_sha256()}));
  }
  assert!(f32::from_bits(resources.model_radius_64(&handle).unwrap())>1.0);
  assert_eq!(resources.flags_08(&handle).unwrap()&6,0,"vf4C does not run ResourceCallbacks OR6 or task completion");
  assert!(backend.timer.as_ref().unwrap().milliseconds().is_ok());
  println!("{}",serde_json::json!({"scope":"actual selected NIF/static typed graph and explicit policy lab; no native Task/flags/cache/GPU-ready completion","rows":reports}));
 }
}

