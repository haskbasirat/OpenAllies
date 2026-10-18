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
use std::{collections::{BTreeSet,BTreeMap},path::PathBuf,sync::Arc};
#[path="model_property_metadata.rs"]
mod property_metadata;
#[path="model_graph_fields.rs"]
mod graph_fields;
#[path="model_hidden_tree.rs"]
mod hidden_tree;
#[path="model_property_state.rs"]
mod property_state;
type Result<T>=std::result::Result<T,String>;
const INDEX:&str=include_str!("../../../assets-cache/model-native-graphs/index.json");
const IDENTITY:[u32;13]=[0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000];
const MERGE_EPSILON:u32=897988541; //nativeB32D94 image-backed Ni math constant
const FOOTPRINT_THRESHOLD:u32=0x3dcccccd; //native9FE0C0
fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn finite(words:impl IntoIterator<Item=u32>)->Result<()>{if words.into_iter().any(|w|!f32::from_bits(w).is_finite()){return Err("selected NIF nonfinite source field requires native exceptional numeric path".into())}Ok(())}
#[derive(Debug,Deserialize)]struct Document{schema:u32,source:String,source_sha256:String,converted_sha256:String,material_audit_sha256:String,version:u32,roots:Vec<Option<usize>>,nodes:Vec<Node>,textures:Vec<Texture>,mesh_mapping:serde_json::Value}
#[derive(Debug,Deserialize)]struct Node{
 kind:String,name:String,parent:Option<usize>,hierarchy:Vec<String>,local:[u32;13],flags:u32,children:Vec<Option<usize>>,
 vertices:Vec<[u32;3]>,#[serde(default)]normals:Vec<[u32;3]>,sphere:Option<[u32;4]>,skin:bool,shadow:bool,team:bool,tags:Vec<String>,
 properties:Vec<Property>,controllers:Vec<Controller>,extras:Vec<serde_json::Value>,
 #[serde(default)]has_normals:bool,
 data_block:Option<u32>,#[serde(skip)]property_blocks:Vec<u32>,#[serde(skip)]generated:BTreeMap<String,Extra>,
 #[serde(skip)]world:Option<[u32;13]>,#[serde(skip)]world_sphere:Option<[u32;4]>,
 #[serde(skip)]mode_c0:Option<u32>,#[serde(skip)]detached:bool,
 #[serde(skip)]source_block:Option<u32>,#[serde(skip)]effect_blocks:Option<Vec<Option<u32>>>,
 #[serde(skip)]property_state:Option<Arc<[PropertySource;12]>>,
 #[serde(skip)]effects_updated:bool,
}
#[derive(Debug,Deserialize,PartialEq,Eq)]struct Property{kind:String,block:u32,#[serde(default)]flags:u32,#[serde(default)]slots:Vec<Option<Descriptor>>,#[serde(default)]shader_textures:u32,#[serde(default)]extras:Vec<serde_json::Value>,#[serde(skip)]generated:BTreeMap<String,Extra>,#[serde(skip)]source_fields:Option<graph_fields::Property>}
#[derive(Debug,Deserialize,PartialEq,Eq)]struct Descriptor{texture:Option<String>,source:Option<u32>,filter:u32}
#[derive(Debug,Deserialize)]struct Controller{kind:String,flags:u32,stop:u32}
#[derive(Debug,Deserialize)]struct Texture{block:u32,filename:Option<String>,extras:Vec<serde_json::Value>,#[serde(skip)]generated:BTreeMap<String,Extra>}
#[derive(Debug,PartialEq,Eq)]struct Geometry{vertices:Vec<[u32;3]>,sphere:Option<[u32;4]>,normals:Option<Vec<[u32;3]>>}
#[derive(Debug,PartialEq,Eq)]enum Extra{Integer(u32),Boolean(u32),Text(Vec<u16>),Processed(Arc<Metadata>),TextureNames(Arc<Vec<Vec<u16>>>),Material([u32;12]),Opacity(property_metadata::Opacity) }
#[derive(Clone,Debug)]enum PropertySource{Default(DefaultProperty),Source(u32)}
#[derive(Debug)]struct Selected{
 source_sha:String,filename:Vec<u16>,source:Option<SourceContext>,graph:Document,root:usize,pc:Precision,
 render:PreparedModel,metadata:Option<Arc<Metadata>>,textures:Vec<Vec<u16>>,processing:bool,wrapped:bool,updated:bool,
 data:BTreeMap<u32,Geometry>,properties:BTreeMap<u32,Property>,texture_objects:BTreeMap<u32,Texture>,
}
///The input was minted by the actual Resource file read. The material loader
///reads and verifies converted bytes once; this graph projection is separately
///bound to the exact input SHA and that SAME paired source-node audit.
pub(crate) fn prepare(input:&Input,store:&AssetStore,pc:Precision)->Result<Payload>{
 let filename=input.resource_ids().to_vec();let source=input.source_context().cloned();
 input.prepare_with(|raw,selected|{
  if digest(raw)!=input.sha256(){return Err("selected NIF byte allocation digest differs".into())}
  let index:serde_json::Value=serde_json::from_str(INDEX).map_err(|e|e.to_string())?;
  let selected_key=selected.logical_path().replace('\\',"/").to_ascii_lowercase();
  let record=&index["by_source"][&selected_key];let expected=record["sha256"].as_str().ok_or("selected NIF has no paired raw graph projection; regenerate the source artifact")?;
  if record["source_sha256"]!=input.sha256(){return Err("selected NIF source bytes differ from its canonical graph record".into())}
  let canonical=record["source"].as_str().ok_or("raw graph index source is absent")?;
  if !canonical.eq_ignore_ascii_case(&selected.logical_path().replace('\\',"/")){return Err("selected NIF physical source differs from graph index".into())}
  let artifact=record["artifact"].as_str().ok_or("selected NIF graph artifact is absent")?;
  let path=discover_graph(store,input.sha256(),artifact)?;let bytes=std::fs::read(&path).map_err(|e|e.to_string())?;
  if digest(&bytes)!=expected{return Err("raw graph projection digest differs from this build's paired index".into())}
  let mut graph:Document=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
  if graph.schema!=2||graph.source_sha256!=input.sha256()||graph.source!=canonical{return Err("raw graph/source identity differs".into())}
  let fixes=crate::model_fixes::ModelFixes::discover(store).map_err(|e|e.to_string())?;
  fixes.verify_native_graph_pair(&graph.source,input.sha256(),&graph.converted_sha256,&graph.material_audit_sha256,&graph.mesh_mapping).map_err(|e|e.to_string())?;
  let converted=fixes.load_selected_model(store,&graph.source,input.sha256()).map_err(|e|e.to_string())?;
  let mut render=converted.prepare().map_err(|e|e.to_string())?;fixes.apply(store,&mut render).map_err(|e|e.to_string())?;
  let root=validate(&graph)?;
  //Materialize each serialized Data/Property/Texture block ONCE. Aliased
  //node references must agree before the projection's duplicate input rows
  //are consumed; later mutations address only this sole typed owner.
  let mut data=BTreeMap::new();let mut properties=BTreeMap::new();let mut texture_objects=BTreeMap::new();
  for texture in std::mem::take(&mut graph.textures){if texture_objects.insert(texture.block,texture).is_some(){return Err("duplicate NIF SourceTexture block identity".into())}}
  for node in &mut graph.nodes{
   if let Some(block)=node.data_block{if node.has_normals&&node.normals.len()!=node.vertices.len(){return Err("selected NIF normal array differs from its GeometryData vertex count".into())}if !node.has_normals&&!node.normals.is_empty(){return Err("selected NIF normal pointer/array source differs".into())}let fields=Geometry{vertices:std::mem::take(&mut node.vertices),sphere:node.sphere.take(),normals:node.has_normals.then(||std::mem::take(&mut node.normals))};if let Some(old)=data.get(&block){if old!=&fields{return Err("aliased NIF GeometryData fields differ".into())}}else{data.insert(block,fields);}}
   for property in std::mem::take(&mut node.properties){node.property_blocks.push(property.block);for descriptor in property.slots.iter().flatten(){if let Some(source)=descriptor.source{let actual=texture_objects.get(&source).ok_or("NIF descriptor SourceTexture allocation absent")?;if actual.filename!=descriptor.texture{return Err("NIF descriptor filename differs from its shared SourceTexture owner".into())}}}
    if let Some(old)=properties.get(&property.block){if old!=&property{return Err("aliased NIF Property fields differ".into())}}else{properties.insert(property.block,property);}}
  }
  let mut additional=graph_fields::load(store,canonical,input.sha256(),expected)?;
  if additional.nodes.len()!=graph.nodes.len(){return Err("source graph node allocation census differs".into())}
  let mut node_blocks=BTreeSet::new();for(node,fields)in graph.nodes.iter_mut().zip(std::mem::take(&mut additional.nodes)){
   if node.kind!=fields.kind||node.name!=fields.name||node.property_blocks!=fields.property_blocks||!node_blocks.insert(fields.block)
      ||node.controllers.len()!=fields.controllers.len()||node.controllers.iter().zip(&fields.controllers).any(|(a,b)|a.kind!=b.kind||a.flags!=b.flags||a.stop!=b.stop){return Err("source graph node/Property/controller identity differs".into())}
   node.source_block=Some(fields.block);node.effect_blocks=Some(fields.effect_blocks);
  }
  let mut source_properties=additional.take_properties()?;
  for(block,property)in &mut properties{let fields=source_properties.remove(block).ok_or("same NIF Property source fields absent")?;if property.kind!=fields.kind{return Err("same NIF Property source class differs".into())}property.source_fields=Some(fields);}
  Ok(Box::new(Selected{source_sha:input.sha256().into(),filename,source,graph,root,pc,render,metadata:None,textures:vec![],processing:false,wrapped:false,updated:false,data,properties,texture_objects})as Box<dyn Model>)
 })
}
fn discover_graph(store:&AssetStore,sha:&str,artifact:&str)->Result<PathBuf>{discover_artifact(store,"model-native-graphs",sha,artifact)}
fn discover_artifact(store:&AssetStore,folder:&str,sha:&str,artifact:&str)->Result<PathBuf>{
 if sha.len()!=64||!sha.bytes().all(|b|b.is_ascii_hexdigit()){return Err("invalid selected NIF digest".into())}
 let stem=artifact.strip_suffix(".json").ok_or("raw model graph artifact extension differs")?;
 if stem!=sha&&!stem.strip_prefix(sha).and_then(|s|s.strip_prefix('-')).is_some_and(|s|s.len()==16&&s.bytes().all(|b|b.is_ascii_hexdigit())){return Err("raw model graph artifact is not a plain source-bound filename".into())}
 let relative=PathBuf::from("assets-cache").join(folder).join(artifact);
 let mut roots=Vec::new();if let Ok(exe)=std::env::current_exe(){if let Some(folder)=exe.parent(){roots.extend(folder.ancestors().map(|p|p.to_owned()));}}
 if let Some(game)=store.root().parent(){roots.push(game.join("rust-rebuild"));}
 for root in roots{let p=root.join(&relative);if p.try_exists().map_err(|e|e.to_string())?{if !p.is_file(){return Err("raw model graph path is not a file".into())}return Ok(p)}}
 Err(format!("selected NIF raw graph artifact is missing: {}",relative.display()))
}
fn validate(d:&Document)->Result<usize>{
 if d.version!=0x0a010000{return Err(format!("selected NIF version{:X} raw graph loader requires implementation",d.version))}
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
struct Propagating<'a>(&'a mut Selected);
impl property_state::Tree<PropertySource> for Propagating<'_>{
 fn children(&self,i:usize)->&[Option<usize>]{&self.0.graph.nodes[i].children}
 fn own_count(&self,i:usize)->usize{self.0.graph.nodes[i].property_blocks.len()}
 fn own(&self,i:usize,j:usize)->Result<(usize,PropertySource)>{let block=self.0.graph.nodes[i].property_blocks[j];let property=self.0.properties.get(&block).ok_or("actual own Property allocation absent")?;Ok((PropertyKind::from_name(&property.kind)?.index(),PropertySource::Source(block)))}
 fn geometry(&self,i:usize)->bool{Selected::geometry(&self.0.graph.nodes[i])}
 fn retain(&mut self,i:usize,state:Arc<[PropertySource;12]>)->Result<()>{self.0.graph.nodes[i].property_state=Some(state);Ok(())}
}
impl Selected{
 fn prepare_native_metadata_graph(&mut self)->Result<()>{
  self.update()?;
  for i in self.order(){self.supported(i)?;if self.graph.nodes[i].effect_blocks.as_ref().ok_or("actual source effect list is unbound")?.iter().any(Option::is_some){return Err("actual source dynamic-effect processing is required".into())}}
  hidden_tree::remove_hidden(&mut Hidden(&mut self.graph.nodes),self.root)?;
  self.update()?;
  let mut defaults=Vec::with_capacity(12);for kind in [PropertyKind::Alpha,PropertyKind::Dither,PropertyKind::Fog,PropertyKind::Material,PropertyKind::RendererSpecific,PropertyKind::Shade,PropertyKind::Specular,PropertyKind::Stencil,PropertyKind::Texturing,PropertyKind::VertexColor,PropertyKind::Wireframe,PropertyKind::ZBuffer]{defaults.push(PropertySource::Default(self.source()?.default_property(kind)?));}
  let defaults:[PropertySource;12]=defaults.try_into().map_err(|_|"actual default Property census differs")?;let root=self.root;
  property_state::propagate(&mut Propagating(self),root,defaults)?;
  //86F090 starts at this actual parentless selected root. Its scratch is
  //cleared before traversal; every admitted source node has an explicitly
  //empty effect list. Thus the inherited effect state is NULL at each node.
  //A positive source effect is refused above, never replaced by emptiness.
  for i in self.order(){self.graph.nodes[i].effects_updated=true;}
  self.update()?;
  Err("87AA10 property/effect propagation is retained; actual879A90 own-property/material/tag processing remains required".into())
 }
 fn source(&self)->Result<&SourceContext>{self.source.as_ref().ok_or_else(||"model requires same preWorld canonical Bone source context".into())}
 fn geometry(n:&Node)->bool{matches!(n.kind.as_str(),"NiTriShape"|"NiTriStrips")}
 fn supported(&self,i:usize)->Result<()>{let n=&self.graph.nodes[i];if n.skin{return Err(format!("selected NIF actual {} skin graph requires skin transforms",n.name))}if !n.controllers.is_empty(){return Err(format!("selected NIF actual {} controller update requires its real key/controller graph",n.name))}if !matches!(n.kind.as_str(),"NiNode"|"NiTriShape"|"NiTriStrips"){return Err(format!("selected NIF actual {} {} graph processing is required",n.kind,n.name))}Ok(())}
 fn order(&self)->Vec<usize>{let mut out=vec![];let mut work=vec![self.root];while let Some(i)=work.pop(){if self.graph.nodes[i].detached{continue}out.push(i);work.extend(self.graph.nodes[i].children.iter().rev().flatten().copied());}out}
 fn geometry_data(&self,i:usize)->Result<&Geometry>{let block=self.graph.nodes[i].data_block.ok_or("actual GeometryData source pointer is NULL")?;self.data.get(&block).ok_or_else(||"actual GeometryData allocation is absent".into())}
 //86EA70 reads THIS object's A4 list. It does not search its parents.
 fn property(&self,i:usize,kind:&str)->Option<&Property>{self.graph.nodes[i].property_blocks.iter().filter_map(|b|self.properties.get(b)).find(|p|p.kind==kind)}
 fn update(&mut self)->Result<()>{let order=self.order();for &i in &order{self.supported(i)?;let n=&self.graph.nodes[i];let world=match n.parent{Some(p)=>math::compose_world(self.graph.nodes[p].world.ok_or("NIF parent transform is not current")?,n.local,self.pc)?,None=>n.local};let sphere=if Self::geometry(n){self.geometry_data(i)?.sphere.map(|sphere|math::transform_sphere(&world,sphere,self.pc)).transpose()?}else{None};self.graph.nodes[i].world=Some(world);self.graph.nodes[i].world_sphere=sphere;}
  for &i in order.iter().rev(){if Self::geometry(&self.graph.nodes[i]){continue}let children=self.graph.nodes[i].children.clone();let mut sphere=None;for child in children.into_iter().flatten(){let c=&self.graph.nodes[child];if c.detached{continue}if let Some(s)=c.world_sphere{if f32::from_bits(s[3])==0.0{continue}if let Some(current)=sphere.as_mut(){math::merge_sphere(current,s,MERGE_EPSILON,self.pc)?}else{sphere=Some(s)}}}self.graph.nodes[i].world_sphere=sphere;}Ok(())
 }
 fn find_name(&self,name:&[u16])->Option<usize>{self.order().into_iter().find(|&i|self.graph.nodes[i].name.encode_utf16().eq(name.iter().copied()))}
 fn collect_textures(&mut self)->Result<Vec<Vec<u16>>>{let mut out:Vec<Vec<u16>>=vec![];for i in self.order(){let Some(p)=self.property(i,"NiTexturingProperty")else{continue};if p.shader_textures!=0{return Err("actual NiTexturingProperty shader descriptor vector requires processing".into())}if p.slots.len()!=7{return Err("actual NiTexturingProperty descriptor layout differs".into())}let sources=[0,5,1,2,3,4,6].into_iter().filter_map(|slot|p.slots[slot].as_ref().and_then(|d|d.source)).collect::<Vec<_>>();
   for source in sources{let texture=self.texture_objects.get_mut(&source).ok_or("actual SourceTexture owner is absent")?;let name=texture.filename.as_ref().ok_or("8792F0 actual internal-texture diagnostic is required")?.encode_utf16().collect::<Vec<_>>();
    if !out.iter().any(|x|ascii_equal(x,&name)){out.push(name.clone())}
    if !texture.extras.iter().any(|v|v["extra_name"]=="otn")&&!texture.generated.contains_key("otn"){texture.generated.insert("otn".into(),Extra::Text(name));}
   }
  }Ok(out)}
 fn attach_base_texture(&mut self)->Result<()>{let selected=select_base_texture(&self.filename,&self.textures,&vec![true;self.textures.len()])?;
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
  if self.graph.nodes[self.root].extras.iter().any(|e|e["extra_name"]=="ppdv"&&e["kind"]=="NiIntegerExtraData"&&e["integer"]==7){return Err("87AA10 valid source ppdv7 metadata reuse requires its actual serialized payload branch".into())}
  self.processing=true;
  if has_skin{return Err("87AA10 has-skin unique texture/clone path requires concrete skin graph".into())}
  self.prepare_native_metadata_graph()?;
  self.update()?;let mut bounds=math::Bounds::empty_processing();let mut stop=0;let mut blend=false;
  for i in self.order(){self.supported(i)?;
   //Only these source-normalized material tags are admitted by the existing
   //paired audit. Other positive process tags must gain their real operation.
   if self.graph.nodes[i].tags.iter().any(|t|!matches!(t.as_str(),"RepColor"|"Sg"|"Sg2"|"Eq")){return Err("87AA10 positive non-material graph tag requires its actual processing child".into())}
   for controller in &mut self.graph.nodes[i].controllers{finite([controller.stop])?;if controller.flags&1!=0{return Err("8785D0 APP_TIME warning service is required before controller flag mutation".into())}if f32::from_bits(controller.stop)>f32::from_bits(stop){stop=controller.stop;}}
   if Self::geometry(&self.graph.nodes[i]){
    let remove=policy.renderer_remove_normals_current()?;if remove!=0{let block=self.graph.nodes[i].data_block.ok_or("87A1E5 actual GeometryData pointer is NULL")?;let data=self.data.get_mut(&block).ok_or("87A1E5 actual GeometryData allocation absent")?;data.normals=None;if data.vertices.is_empty(){return Err("8ABEC0 empty GeometryData sphere needs actual vertex-pointer/global-center owners".into())}data.sphere=Some(math::sphere_from_vertices(&data.vertices,None,self.pc)?);}
    if let Some(p)=self.property(i,"NiAlphaProperty"){blend|=p.flags&1!=0;}
    let n=&self.graph.nodes[i];if !n.shadow{bounds.include_geometry(&n.world.ok_or("raw NIF world transform absent")?,&self.geometry_data(i)?.vertices,n.flags&1!=0,self.pc)?;}
   }
  }
  if let Some(height)=bounds.begin_footprint(){
   for i in self.order(){let n=&self.graph.nodes[i];if !Self::geometry(n){continue}let mut ancestor=Some(i);let mut equipment=false;while let Some(j)=ancestor{let a=&self.graph.nodes[j];equipment|=a.tags.iter().any(|t|t=="Eq");ancestor=a.parent;}if !equipment{bounds.include_footprint_at_height(&n.world.ok_or("raw NIF world transform absent")?,&self.geometry_data(i)?.vertices,n.flags&1!=0,height,FOOTPRINT_THRESHOLD,self.pc)?;}}
  }
  self.textures=self.collect_textures()?;
  self.attach_base_texture()?;
  let m=Metadata{byte1:0,word4:stop,radial8:bounds.radial,max_z0c:bounds.max_z,min_z10:bounds.min_z,footprint14:bounds.footprint[0],footprint18:bounds.footprint[1],footprint1c:bounds.footprint[2],texture_count20:u32::try_from(self.textures.len()).map_err(|e|e.to_string())?,byte24:u8::from(blend)};
  let metadata=Arc::new(m);let n=&mut self.graph.nodes[self.root];
  for name in ["ppd","ppdv","pptn"]{if n.extras.iter().filter(|v|v["extra_name"]==name).count()>1{return Err("8780A0 duplicate processed extra needs actual remove-first ownership".into())}n.extras.retain(|v|v["extra_name"]!=name);}
  n.generated.insert("ppdv".into(),Extra::Integer(7));n.generated.insert("ppd".into(),Extra::Processed(Arc::clone(&metadata)));
  if !self.textures.is_empty(){n.generated.insert("pptn".into(),Extra::TextureNames(Arc::new(self.textures.clone())));}
  self.metadata=Some(Arc::clone(&metadata));self.processing=false;Ok((*metadata).clone())
 }
 fn texture_names(&self)->Result<Vec<Vec<u16>>>{self.metadata.as_ref().ok_or("native model metadata has not completed")?;Ok(self.textures.clone())}
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
 fn wrap_root(&mut self)->Result<()>{if self.wrapped{return Err("867690 model already wrapped".into())}let old=self.root;let new=self.graph.nodes.len();let name=self.graph.nodes[old].name.clone();let dcss=self.graph.nodes[old].extras.iter().any(|v|v["extra_name"]=="dcss");
  self.graph.nodes.try_reserve(1).map_err(|e|e.to_string())?;
  let extras=if dcss{vec![serde_json::json!({"extra_name":"dcss","is_string":false,"value":""})]}else{vec![]};
  self.graph.nodes.push(Node{kind:"NiNode".into(),name:name.clone(),parent:None,hierarchy:vec![name],local:IDENTITY,flags:0x30,children:vec![Some(old)],vertices:vec![],normals:vec![],sphere:None,skin:false,shadow:false,team:false,tags:vec![],properties:vec![],controllers:vec![],extras,has_normals:false,data_block:None,property_blocks:vec![],generated:BTreeMap::new(),world:None,world_sphere:None,mode_c0:Some(0),detached:false,source_block:None,effect_blocks:Some(vec![]),property_state:None,effects_updated:false});
  self.graph.nodes[old].name="base".into();self.graph.nodes[old].parent=Some(new);self.root=new;self.wrapped=true;Ok(())
 }
 fn update_root(&mut self)->Result<()>{if !self.wrapped{return Err("model root wrapper is not installed".into())}self.update()?;self.updated=true;Ok(())}
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
 struct HeadlessBackend<'a>{capability:crate::live_scene::model_policy::Capabilities,variables:&'a aa_ui::uvars::UVars,store:&'a AssetStore,pc:Precision,timer:Option<crate::windows_reset_os::PerformanceTimer>}
 impl aa_runtime::resource_objects::ModelPolicy for HeadlessBackend<'_>{fn use_hardware_model_73a560(&mut self)->Result<bool>{self.capability.use_hardware_model_current(self.variables)}}
 impl MetadataPolicy for HeadlessBackend<'_>{fn renderer_remove_normals_current(&mut self)->Result<u8>{self.variables.renderer_remove_normals_current()}}
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
 #[test]fn raw_graph_index_never_accepts_unpaired_source_or_aliased_graph(){let index:serde_json::Value=serde_json::from_str(INDEX).unwrap();let records=index["by_source"].as_object().unwrap();for source in ["buildings/gbr/gbrcorps/gbrcorps.nif","buildings/gbr/gbrcorps/gbrcorpspack.nif","buildings/gbr/gbrdepotoil/gbrdepotoil.nif"]{assert!(records.contains_key(source));}for(key,record)in records{assert_eq!(record["source"].as_str().unwrap().to_ascii_lowercase(),*key);assert_eq!(record["sha256"].as_str().unwrap().len(),64);assert_eq!(record["source_sha256"].as_str().unwrap().len(),64);}}
 #[test]fn base_texture_selector_and_same_property_mutation_match_native(){
  let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-model-resource-load/native-model-base-texture.json")).unwrap();
  let wide=|v:&serde_json::Value|v.as_str().unwrap().encode_utf16().collect::<Vec<_>>();
  let property=|initial:&serde_json::Value|Property{kind:"NiTexturingProperty".into(),block:17,flags:0,slots:vec![],shader_textures:0,extras:initial.as_array().unwrap().iter().map(|v|serde_json::json!({"extra_name":v[0],"kind":"NiBooleanExtraData","value":v[1]})).collect(),generated:BTreeMap::new(),source_fields:None};
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
    if !slots.is_null(){n.property_blocks.push(block);properties.insert(block,Property{kind:"NiTexturingProperty".into(),block,flags:0,slots:slots.as_array().unwrap().iter().map(|v|if v.as_u64().unwrap()!=0{Some(Descriptor{source:None,texture:None,filter:0xdeadbeef})}else{None}).collect(),shader_textures:0,extras:vec![],generated:BTreeMap::new(),source_fields:None});}nodes.push(n);index};
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
  let renderer=renderer();let mut backend=HeadlessBackend{capability:renderer.model_variant_capabilities(),variables:&variables,store:&store,pc:Precision::Double53,timer:None};
  let ids="Buildings/GBR/GBRDepotOil/GBRDepotOil.nif".encode_utf16().collect::<Vec<_>>();
  let handle=resources.model_5a8520(&catalog,&bank,&ids,&mut backend).unwrap();let input=resources.model_input_4c(&handle).unwrap();
  assert_eq!(input.source_context().unwrap().first_controller_bone_name().unwrap(),"Scene Root".encode_utf16().collect::<Vec<_>>());
  assert_eq!(input.source_context().unwrap().selection_box_name().unwrap().unwrap(),"SelectionBox".encode_utf16().collect::<Vec<_>>());
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
    let Some(Extra::TextureNames(names))=root.generated.get("pptn")else{panic!("actual root texture names extra absent")};assert_eq!(names.as_ref(),&m.textures);
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
  assert!(resources.model_payload_40(&handle).unwrap().is_none());
  assert!(resources.load_model_4c(&handle,Precision::Double53,&mut backend).unwrap());
  let retained=resources.model_payload_40(&handle).unwrap().unwrap();retained.require_resource(&handle).unwrap();assert_eq!(retained.source_sha256(),input.sha256());
  assert!(f32::from_bits(resources.model_radius_64(&handle).unwrap())>1.0);
  assert_eq!(resources.flags_08(&handle).unwrap()&6,0,"vf4C does not run ResourceCallbacks OR6 or task completion");
  assert!(backend.timer.as_ref().unwrap().milliseconds().is_ok());
  println!("{}",serde_json::json!({"scope":"actual selected NIF/static typed graph and explicit policy lab; no native Task/flags/cache/GPU-ready completion","rows":reports}));
 }
}

