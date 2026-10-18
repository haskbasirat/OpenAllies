//!The selected raw NIF and the paired modern render artifact share one owned
//!static graph. Native scalar processing uses the raw binary32 source words;
//!converted meshes are retained only as the modern rendering representation.
//!This does not implement the Gamebryo allocation/refcount ABI or GPU readiness.
use aa_assets::{AssetStore,PreparedModel};
use aa_numeric::Precision;
use aa_runtime::{resource_model_payload::{Input,Payload,Model,Metadata,MetadataPolicy},resource_model_bounds as math,resource_model_source::Context as SourceContext};
use serde::Deserialize;
use sha2::{Digest,Sha256};
use std::{collections::BTreeSet,path::PathBuf};
type Result<T>=std::result::Result<T,String>;
const INDEX:&str=include_str!("../../../assets-cache/model-native-graphs/index.json");
const IDENTITY:[u32;13]=[0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000,0,0,0,0x3f800000];
const MERGE_EPSILON:u32=897988541; //nativeB32D94 image-backed Ni math constant
const FOOTPRINT_THRESHOLD:u32=0x3dcccccd; //native9FE0C0
fn digest(bytes:&[u8])->String{format!("{:x}",Sha256::digest(bytes))}
fn finite(words:impl IntoIterator<Item=u32>)->Result<()>{if words.into_iter().any(|w|!f32::from_bits(w).is_finite()){return Err("selected NIF nonfinite source field requires native exceptional numeric path".into())}Ok(())}
#[derive(Debug,Deserialize)]struct Document{schema:u32,source:String,source_sha256:String,converted_sha256:String,material_audit_sha256:String,version:u32,roots:Vec<Option<usize>>,nodes:Vec<Node>,mesh_mapping:serde_json::Value}
#[derive(Debug,Deserialize)]struct Node{
 kind:String,name:String,parent:Option<usize>,hierarchy:Vec<String>,local:[u32;13],flags:u32,children:Vec<Option<usize>>,
 vertices:Vec<[u32;3]>,sphere:Option<[u32;4]>,skin:bool,shadow:bool,team:bool,tags:Vec<String>,
 properties:Vec<Property>,controllers:Vec<Controller>,extras:Vec<serde_json::Value>,
 #[serde(default)]has_normals:bool,
 #[serde(skip)]world:Option<[u32;13]>,#[serde(skip)]world_sphere:Option<[u32;4]>,
 #[serde(skip)]mode_c0:Option<u32>,#[serde(skip)]detached:bool,
}
#[derive(Debug,Deserialize)]struct Property{kind:String,#[serde(default)]flags:u32,#[serde(default)]slots:Vec<Option<Descriptor>>,#[serde(default)]shader_textures:u32}
#[derive(Debug,Deserialize)]struct Descriptor{texture:Option<String>,filter:u32}
#[derive(Debug,Deserialize)]struct Controller{kind:String,flags:u32,stop:u32}
#[derive(Debug)]struct Selected{
 source_sha:String,filename:Vec<u16>,source:Option<SourceContext>,graph:Document,root:usize,pc:Precision,
 render:PreparedModel,metadata:Option<Metadata>,textures:Vec<Vec<u16>>,processing:bool,wrapped:bool,
}
///The input was minted by the actual Resource file read. The material loader
///reads and verifies converted bytes once; this graph projection is separately
///bound to the exact input SHA and that SAME paired source-node audit.
pub(crate) fn prepare(input:&Input,store:&AssetStore,pc:Precision)->Result<Payload>{
 let filename=input.resource_ids().to_vec();let source=input.source_context().cloned();
 input.prepare_with(|raw,selected|{
  if digest(raw)!=input.sha256(){return Err("selected NIF byte allocation digest differs".into())}
  let index:serde_json::Value=serde_json::from_str(INDEX).map_err(|e|e.to_string())?;
  let record=&index["graphs"][input.sha256()];let expected=record["sha256"].as_str().ok_or("selected NIF has no paired raw graph projection; regenerate the source artifact")?;
  let canonical=record["source"].as_str().ok_or("raw graph index source is absent")?;
  if !canonical.eq_ignore_ascii_case(&selected.logical_path().replace('\\',"/")){return Err("selected NIF physical source differs from graph index".into())}
  let path=discover_graph(store,input.sha256())?;let bytes=std::fs::read(&path).map_err(|e|e.to_string())?;
  if digest(&bytes)!=expected{return Err("raw graph projection digest differs from this build's paired index".into())}
  let graph:Document=serde_json::from_slice(&bytes).map_err(|e|e.to_string())?;
  if graph.schema!=1||graph.source_sha256!=input.sha256()||graph.source!=canonical{return Err("raw graph/source identity differs".into())}
  let fixes=crate::model_fixes::ModelFixes::discover(store).map_err(|e|e.to_string())?;
  fixes.verify_native_graph_pair(&graph.source,input.sha256(),&graph.converted_sha256,&graph.material_audit_sha256,&graph.mesh_mapping).map_err(|e|e.to_string())?;
  let converted=fixes.load_selected_model(store,&graph.source,input.sha256()).map_err(|e|e.to_string())?;
  let mut render=converted.prepare().map_err(|e|e.to_string())?;fixes.apply(store,&mut render).map_err(|e|e.to_string())?;
  let root=validate(&graph)?;
  Ok(Box::new(Selected{source_sha:input.sha256().into(),filename,source,graph,root,pc,render,metadata:None,textures:vec![],processing:false,wrapped:false})as Box<dyn Model>)
 })
}
fn discover_graph(store:&AssetStore,sha:&str)->Result<PathBuf>{
 if sha.len()!=64||!sha.bytes().all(|b|b.is_ascii_hexdigit()){return Err("invalid selected NIF digest".into())}
 let relative=PathBuf::from("assets-cache/model-native-graphs").join(format!("{sha}.json"));
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
impl Selected{
 fn source(&self)->Result<&SourceContext>{self.source.as_ref().ok_or_else(||"model requires same preWorld canonical Bone source context".into())}
 fn geometry(n:&Node)->bool{matches!(n.kind.as_str(),"NiTriShape"|"NiTriStrips")}
 fn supported(&self,i:usize)->Result<()>{let n=&self.graph.nodes[i];if n.skin{return Err(format!("selected NIF actual {} skin graph requires skin transforms",n.name))}if !n.controllers.is_empty(){return Err(format!("selected NIF actual {} controller update requires its real key/controller graph",n.name))}if !matches!(n.kind.as_str(),"NiNode"|"NiTriShape"|"NiTriStrips"){return Err(format!("selected NIF actual {} {} graph processing is required",n.kind,n.name))}Ok(())}
 fn order(&self)->Vec<usize>{let mut out=vec![];let mut work=vec![self.root];while let Some(i)=work.pop(){if self.graph.nodes[i].detached{continue}out.push(i);work.extend(self.graph.nodes[i].children.iter().rev().flatten().copied());}out}
 fn property(&self,mut i:usize,kind:&str)->Option<&Property>{loop{let n=&self.graph.nodes[i];if let Some(p)=n.properties.iter().find(|p|p.kind==kind){return Some(p)}i=n.parent?;}}
 fn update(&mut self)->Result<()>{let order=self.order();for &i in &order{self.supported(i)?;let n=&self.graph.nodes[i];let world=match n.parent{Some(p)=>math::compose_world(self.graph.nodes[p].world.ok_or("NIF parent transform is not current")?,n.local,self.pc)?,None=>n.local};let sphere=n.sphere.map(|sphere|math::transform_sphere(&world,sphere,self.pc)).transpose()?;self.graph.nodes[i].world=Some(world);self.graph.nodes[i].world_sphere=sphere;}
  for &i in order.iter().rev(){if Self::geometry(&self.graph.nodes[i]){continue}let children=self.graph.nodes[i].children.clone();let mut sphere=None;for child in children.into_iter().flatten(){let c=&self.graph.nodes[child];if c.detached{continue}if let Some(s)=c.world_sphere{if f32::from_bits(s[3])==0.0{continue}if let Some(current)=sphere.as_mut(){math::merge_sphere(current,s,MERGE_EPSILON,self.pc)?}else{sphere=Some(s)}}}self.graph.nodes[i].world_sphere=sphere;}Ok(())
 }
 fn find_name(&self,name:&[u16])->Option<usize>{self.order().into_iter().find(|&i|self.graph.nodes[i].name.encode_utf16().eq(name.iter().copied()))}
 fn collect_textures(&self)->Result<Vec<Vec<u16>>>{let mut out:Vec<Vec<u16>>=vec![];for i in self.order(){let Some(p)=self.property(i,"NiTexturingProperty")else{continue};if p.shader_textures!=0{return Err("actual NiTexturingProperty shader descriptor vector requires processing".into())}if p.slots.len()!=7{return Err("actual NiTexturingProperty descriptor layout differs".into())}for slot in [0,5,1,2,3,4,6]{if let Some(Some(desc))=p.slots.get(slot){let Some(texture)=&desc.texture else{continue};let name=texture.encode_utf16().collect::<Vec<_>>();if !out.iter().any(|x|String::from_utf16_lossy(x).eq_ignore_ascii_case(texture)){out.push(name)}}}}Ok(out)}
}
impl Model for Selected{
 fn source_sha256(&self)->&str{&self.source_sha}
 fn process_metadata_with_policy(&mut self,has_skin:bool,policy:&mut dyn MetadataPolicy)->Result<Metadata>{
  if self.processing||self.metadata.is_some(){return Err("selected NIF retains a prior metadata processing call; actual retry/cleanup is required".into())}self.processing=true;
  if has_skin{return Err("87AA10 has-skin unique texture/clone path requires concrete skin graph".into())}
  self.update()?;let mut bounds=math::Bounds::empty_processing();let mut stop=0;let mut blend=false;
  for i in self.order(){self.supported(i)?;
   //Only these source-normalized material tags are admitted by the existing
   //paired audit. Other positive process tags must gain their real operation.
   if self.graph.nodes[i].tags.iter().any(|t|!matches!(t.as_str(),"RepColor"|"Sg"|"Sg2"|"Eq")){return Err("87AA10 positive non-material graph tag requires its actual processing child".into())}
   for controller in &mut self.graph.nodes[i].controllers{finite([controller.stop])?;if controller.flags&1!=0{return Err("8785D0 APP_TIME warning service is required before controller flag mutation".into())}if f32::from_bits(controller.stop)>f32::from_bits(stop){stop=controller.stop;}}
   if Self::geometry(&self.graph.nodes[i]){
    let remove=policy.renderer_remove_normals_current()?;if remove!=0{self.graph.nodes[i].has_normals=false;}
    if let Some(p)=self.property(i,"NiAlphaProperty"){blend|=p.flags&1!=0;}
    let n=&self.graph.nodes[i];if !n.shadow{bounds.include_geometry(&n.world.ok_or("raw NIF world transform absent")?,&n.vertices,n.flags&1!=0,self.pc)?;}
   }
  }
  for i in self.order(){let n=&self.graph.nodes[i];if !Self::geometry(n){continue}let mut ancestor=Some(i);let mut equipment=false;while let Some(j)=ancestor{let a=&self.graph.nodes[j];equipment|=a.tags.iter().any(|t|t=="Eq");ancestor=a.parent;}if !equipment{bounds.include_footprint(&n.world.ok_or("raw NIF world transform absent")?,&n.vertices,n.flags&1!=0,FOOTPRINT_THRESHOLD,self.pc)?;}}
  self.textures=self.collect_textures()?;
  let m=Metadata{byte1:0,word4:stop,radial8:bounds.radial,max_z0c:bounds.max_z,min_z10:bounds.min_z,footprint14:bounds.footprint[0],footprint18:bounds.footprint[1],footprint1c:bounds.footprint[2],texture_count20:u32::try_from(self.textures.len()).map_err(|e|e.to_string())?,byte24:u8::from(blend)};
  self.metadata=Some(m.clone());self.processing=false;Ok(m)
 }
 fn texture_names(&self)->Result<Vec<Vec<u16>>>{self.metadata.as_ref().ok_or("native model metadata has not completed")?;Ok(self.textures.clone())}
 fn prepare_effect(&mut self,code:u32)->Result<()>{match code{0x871bc0=>{for i in self.order(){if Self::geometry(&self.graph.nodes[i])&&!self.graph.nodes[i].tags.iter().any(|s|s=="Mod"){self.graph.nodes[i].tags.push("Mod".into())}}},4|5=>{for i in self.order(){self.graph.nodes[i].mode_c0=Some(code)}},0x871b40=>{self.graph.nodes[self.root].tags.push("Eq".into())},_=>return Err("unsupported actual model graph preparation code".into())}Ok(())}
 fn controllers(&self)->Result<bool>{let name=self.source()?.first_controller_bone_name()?;let Some(i)=self.find_name(name)else{return Ok(false)};for i in std::iter::once(i).chain(self.graph.nodes[i].children.iter().flatten().copied()){if self.property(i,"NiTexturingProperty").is_some_and(|p|p.slots.get(3).is_some_and(Option::is_some)){return Ok(true)}}Ok(false)}
 fn selection_cuboid(&mut self)->Result<[u32;6]>{
  let name=self.source()?.selection_box_name()?.ok_or("5A94F0 missing Bone selection_box fallback CString owner is required")?.to_vec();self.update()?;
  let Some(i)=self.find_name(&name)else{return Err("5A94F0 actual BoundingSphere selection fallback/diagnostic child is required".into())};
  if !Self::geometry(&self.graph.nodes[i]){return Err("5A94F0 non-Geometry SelectionBox requires native sphere fallback".into())}
  let n=&self.graph.nodes[i];let out=math::selection_geometry(&n.world.ok_or("selection world transform absent")?,&n.vertices,self.pc)?;
  let parent=n.parent.ok_or("6FF850 actual selection parent is NULL")?;let slot=self.graph.nodes[parent].children.iter().position(|c|*c==Some(i)).ok_or("selection child is not in same parent")?;
  self.graph.nodes[parent].children[slot]=None;self.graph.nodes[i].parent=None;self.graph.nodes[i].detached=true;Ok(out)
 }
 fn wrap_root(&mut self)->Result<()>{if self.wrapped{return Err("867690 model already wrapped".into())}let old=self.root;let new=self.graph.nodes.len();let name=self.graph.nodes[old].name.clone();let dcss=self.graph.nodes[old].extras.iter().any(|v|v["extra_name"]=="dcss");
  self.graph.nodes.try_reserve(1).map_err(|e|e.to_string())?;
  let extras=if dcss{vec![serde_json::json!({"extra_name":"dcss","is_string":false,"value":""})]}else{vec![]};
  self.graph.nodes.push(Node{kind:"NiNode".into(),name:name.clone(),parent:None,hierarchy:vec![name],local:IDENTITY,flags:0x30,children:vec![Some(old)],vertices:vec![],sphere:None,skin:false,shadow:false,team:false,tags:vec![],properties:vec![],controllers:vec![],extras,has_normals:false,world:None,world_sphere:None,mode_c0:Some(0),detached:false});
  self.graph.nodes[old].name="base".into();self.graph.nodes[old].parent=Some(new);self.root=new;self.wrapped=true;Ok(())
 }
 fn update_root(&mut self)->Result<()>{if !self.wrapped{return Err("model root wrapper is not installed".into())}self.update()}
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
 #[test]fn raw_graph_index_never_accepts_unpaired_source_or_aliased_graph(){let index:serde_json::Value=serde_json::from_str(INDEX).unwrap();assert_eq!(index["graphs"].as_object().unwrap().len(),3);for record in index["graphs"].as_object().unwrap().values(){assert_eq!(record["sha256"].as_str().unwrap().len(),64);}}
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
   let native:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-selected-depot-oil.json")).unwrap();
   assert_eq!(native["source_sha256"],input.sha256());
   let native_pc=match pc{Precision::Single24=>"Single",Precision::Double53=>"Double",Precision::Extended64=>"Extended"};
   let expected=&native["rows"].as_array().unwrap().iter().find(|r|r["image"]=="original"&&r["precision"]==native_pc).unwrap()["output"];
   assert_eq!(serde_json::json!({"radial":metadata.radial8,"min_z":metadata.min_z10,"max_z":metadata.max_z0c,"footprint":[metadata.footprint14,metadata.footprint18,metadata.footprint1c],"cuboid":cuboid}),*expected,"actual selected source words must match original numeric children");
   assert!((0..3).all(|i|f32::from_bits(cuboid[i])<f32::from_bits(cuboid[3+i])));
   assert!(payload.selection_cuboid().is_err()); //same source selection child was really unlinked
   payload.prepare_effect(0x871bc0).unwrap();payload.prepare_effect(4).unwrap();payload.prepare_effect(0x871b40).unwrap();
   payload.wrap_root().unwrap();assert!(payload.wrap_root().is_err());payload.update_root().unwrap();
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
