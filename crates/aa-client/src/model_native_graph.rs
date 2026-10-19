//!The selected Model's source graph, built at load time from the native
//!object graph of the same bytes (the aa-formats port of NiStream::Load,
//!with NiNode::LinkObject 8678D0 child re-parenting).
//!
//!The root is what the model loader 5A7120 takes: the stream's first
//!top-level object (5A7175, [NiStream+0x148][0]), cast to NiAVObject
//!(711540 with B43E10); a NULL or non-NiAVObject first object takes its
//!diagnostic path. Nodes are numbered in preorder over the linked child
//!slots, the same ordinals the paired material audit's mesh rows use.
//!
//!Each field is the native word the loader stored, except those the source
//!projection keeps as the file word for the native checks of `prepare`
//!(NiAVObject and NiAlphaProperty flags, controller flags). Properties are
//!listed in link order (`prepare` compares them with the native head-first
//!list). `tags` is the conservative inventory of named and CRLF-line
//!material/process tags of the extra data, used only to refuse unported
//!processing; the shadow/team context is computed by the native metadata
//!processing itself.
use super::{Descriptor,Document,Node,Property,Texture};
use aa_formats::nif::{is_kind_of,NifFile,Value};
use std::collections::BTreeMap;
type Result<T>=std::result::Result<T,String>;
const TAGS:[&str;10]=["Mod","Eq","Ambient_RepColor","EmRC","RepColor","FastPath","tred","Sg","Sg2","mkf"];
fn text(bytes:Option<&[u8]>)->Result<String>{
 let bytes=bytes.unwrap_or(&[]);let bytes=bytes.split(|&b|b==0).next().unwrap_or(&[]);
 if !bytes.is_ascii(){return Err("actual native non-ASCII name conversion is required".into())}
 Ok(String::from_utf8(bytes.to_vec()).expect("ASCII"))
}
fn lossy(bytes:Option<&[u8]>)->String{String::from_utf8_lossy(bytes.unwrap_or(&[])).into_owned()}
fn field_text(nif:&NifFile,i:usize,name:&str)->Option<Vec<u8>>{nif.objects[i].fields.string(name).flatten().map(<[u8]>::to_vec)}
///Extra data rows of one NiObjectNET: the 5.0.0.11+ list, then the older
///single head with its `next` chain; each object once, in file order.
fn extras(nif:&NifFile,object:usize)->Result<Vec<serde_json::Value>>{
 let f=&nif.objects[object].fields;let mut items:Vec<usize>=f.links("extra_data_list").unwrap_or(&[]).iter().filter_map(|&l|nif.target(l)).collect();
 let mut link=f.link("extra_data");let mut chain=std::collections::BTreeSet::new();
 while let Some(i)=link.and_then(|l|nif.target(l)){if !chain.insert(i){break}items.push(i);link=nif.objects[i].fields.link("next");}
 let mut seen=std::collections::BTreeSet::new();let mut rows=Vec::new();
 for i in items{if !seen.insert(i){continue}let o=&nif.objects[i];if !is_kind_of(o.class,"NiExtraData"){return Err(format!("extra data link to {}",o.class))}let f=&o.fields;
  let mut row=serde_json::Map::new();
  row.insert("extra_name".into(),lossy(f.string("extra_name").flatten()).into());
  row.insert("value".into(),lossy(f.string("string").flatten()).into());
  row.insert("is_string".into(),(o.class=="NiStringExtraData").into());
  row.insert("kind".into(),o.class.into());
  match o.class{
   "NiIntegerExtraData"=>{row.insert("integer".into(),f.u32("integer").ok_or("87B110 integer absent")?.into());}
   "NiBooleanExtraData"=>{row.insert("boolean".into(),(f.u8("boolean").ok_or("90F980 boolean absent")?!=0).into());}
   "NiBinaryExtraData"=>{let b=match f.get("binary"){Some(Value::Bytes(b))=>b.clone(),_=>vec![]};row.insert("binary".into(),b.iter().map(|x|format!("{x:02x}")).collect::<String>().into());}
   "NiStringsExtraData"=>{let list=f.list("strings").unwrap_or(&[]);let mut out=Vec::new();for s in list{let Value::Str(s)=s else{return Err("887710 string absent".into())};out.push(serde_json::Value::from(text(s.as_deref())?));}row.insert("strings".into(),out.into());}
   _=>{}
  }
  rows.push(serde_json::Value::Object(row));
 }
 Ok(rows)
}
fn tags(extras:&[serde_json::Value])->Vec<String>{
 let mut out=Vec::new();
 for e in extras{
  let name=e["extra_name"].as_str().unwrap_or("");if TAGS.contains(&name){out.push(name.to_string())}
  if e["is_string"]==true{for line in e["value"].as_str().unwrap_or("").split("\r\n"){let candidate=line.rsplit_once('=').map_or(line,|(k,_)|k);
   for known in TAGS{if candidate.is_ascii()&&candidate.eq_ignore_ascii_case(known){out.push(known.to_string())}}}}
 }
 out
}
fn words3(words:&[u32])->Result<Vec<[u32;3]>>{if words.len()%3!=0{return Err("NIF point array length".into())}Ok(words.chunks_exact(3).map(|c|[c[0],c[1],c[2]]).collect())}
fn flags_read(nif:&NifFile,i:usize)->Result<u32>{
 let field=nif.objects[i].fields.field("flags").ok_or("native flags absent")?;
 match(&field.read,&field.value){(Some(Value::U16(v)),_)|(None,Value::U16(v))=>Ok(*v as u32),_=>Err("native flags type".into())}
}
fn property(nif:&NifFile,block:usize)->Result<Property>{
 let o=&nif.objects[block];if !is_kind_of(o.class,"NiProperty"){return Err(format!("NiAVObject property link to {}",o.class))}let f=&o.fields;
 let mut slots=vec![];let mut shader_maps=vec![];let mut shader_textures=0;
 if o.class=="NiTexturingProperty"{
  //The native map array (every slot index 86CF20 stores, absent maps NULL)
  //and the shader map array (+0x44).
  let descriptor=|v:&Value|->Result<Option<Descriptor>>{match v{Value::Record(m)=>{
   let link=m.iter().find(|x|x.name=="source").and_then(|x|match x.value{Value::Link(l)=>nif.target(l),_=>None});
   let filter=m.iter().find(|x|x.name=="filter").and_then(|x|match x.value{Value::U32(v)=>Some(v),_=>None}).ok_or("86D720 filter absent")?;
   let texture=link.and_then(|t|field_text(nif,t,"file_name")).map(|b|text(Some(&b))).transpose()?;
   Ok(Some(Descriptor{texture,source:link.map(|t|t as u32),filter}))}
   _=>Ok(None)}};
  for m in f.list("maps").ok_or("86CF20 maps absent")?{slots.push(descriptor(m)?);}
  for m in f.list("shader_maps").unwrap_or(&[]){shader_maps.push(descriptor(m)?);}
  shader_textures=f.u32("shader_map_count").unwrap_or(0);
 }
 Ok(Property{kind:o.class.into(),block:block as u32,flags:if o.class=="NiAlphaProperty"{flags_read(nif,block)?}else{0},slots,shader_textures,shader_maps,extras:extras(nif,block)?,
  generated:BTreeMap::new(),material:None,alpha_threshold:None,vertex_color:None,controllers:None})
}
pub(super) fn document(nif:&NifFile,source:&str,source_sha256:&str,converted_sha256:String,material_audit_sha256:String,mesh_mapping:serde_json::Value)->Result<Document>{
 //5A716B: no top-level object; 711540: the first one is not an NiAVObject.
 let root=nif.top_level.first().ok_or("5A71A7 selected NIF has no top-level object (diagnostic path)")?.ok_or("5A71A7 selected NIF first top-level object is NULL (diagnostic path)")?;
 if !is_kind_of(nif.objects[root].class,"NiAVObject"){return Err(format!("5A71A7 selected NIF first top-level object is {} (diagnostic path)",nif.objects[root].class))}
 let mut nodes:Vec<Node>=Vec::new();let mut seen=BTreeMap::new();let mut nulls:Vec<Vec<bool>>=Vec::new();
 //(object, parent ordinal, hierarchy); children are numbered as they are reached.
 let mut work=vec![(root,None::<usize>,Vec::<String>::new())];
 while let Some((i,parent,mut hierarchy))=work.pop(){
  if seen.insert(i,nodes.len()).is_some(){return Err("NiAVObject reached twice: the linked native graph is cyclic".into())}
  let o=&nif.objects[i];let f=&o.fields;let name=text(f.string("name").ok_or("86B090 name absent")?)?;hierarchy.push(name.clone());
  let mut local=Vec::with_capacity(13);local.extend_from_slice(f.f32s("rotation").ok_or("870C10 rotation absent")?);local.extend_from_slice(f.f32s("translation").ok_or("869420 translation absent")?);local.push(f.f32("scale").ok_or("86F206 scale absent")?);
  let local:[u32;13]=local.try_into().map_err(|_|"86F150 transform word census")?;
  let mut properties=Vec::new();
  for &link in f.links("properties").ok_or("86F3A0 property links absent")?{if let Some(p)=nif.target(link){properties.push(property(nif,p)?)}}
  let controllers=nif.controllers(i).map_err(|e|e.to_string())?.into_iter().map(|c|->Result<super::Controller>{Ok(super::Controller{kind:nif.objects[c].class.into(),flags:flags_read(nif,c)?,stop:nif.objects[c].fields.f32("stop").ok_or("89F070 stop absent")?})}).collect::<Result<Vec<_>>>()?;
  let extras=extras(nif,i)?;
  let(mut vertices,mut normals,mut sphere,mut has_normals,mut data_block,mut skin)=(vec![],vec![],None,false,None,false);
  if is_kind_of(o.class,"NiGeometry"){
   skin=nif.linked(i,"skin_instance").is_some();
   if let Some(d)=nif.linked(i,"data"){let g=&nif.objects[d].fields;data_block=Some(d as u32);
    if g.u32("has_vertices").unwrap_or(0)!=0{vertices=words3(g.f32s("vertices").unwrap_or(&[]))?;}
    has_normals=g.u32("has_normals").unwrap_or(0)!=0;if has_normals{normals=words3(g.f32s("normals").unwrap_or(&[]))?;normals.truncate(g.u16("vertex_count").unwrap_or(0) as usize);}
    let c=g.f32s("center").ok_or("8C5470 bound centre absent")?;sphere=Some([c[0],c[1],c[2],g.f32("radius").ok_or("8C548C bound radius absent")?]);}
  }
  let index=nodes.len();if seen[&i]!=index{return Err("graph ordinal".into())}
  nodes.push(Node{kind:o.class.into(),name,parent,hierarchy:hierarchy.clone(),local,flags:flags_read(nif,i)?,children:vec![],vertices,normals,sphere,skin,
   tags:tags(&extras),properties,controllers,extras,has_normals,data_block,property_blocks:vec![],generated:BTreeMap::new(),world:None,world_sphere:None,mode_c0:None,detached:false,source_block:None,effect_blocks:None,property_state:None,effects_updated:false});
  //Linked child slots (NULL slots kept); preorder in slot order.
  let slots=nif.node_children(i).map_err(|e|e.to_string())?;
  nulls.push(slots.iter().map(Option::is_none).collect::<Vec<bool>>());
  debug_assert_eq!(nulls.len(),index+1);
  for c in slots.into_iter().flatten().rev(){work.push((c,Some(index),hierarchy.clone()));}
 }
 //A parent's children are reached in slot order, so their ordinals ascend
 //in slot order; NULL slots stay in place.
 let mut reached:BTreeMap<usize,Vec<usize>>=BTreeMap::new();for(k,n)in nodes.iter().enumerate(){if let Some(p)=n.parent{reached.entry(p).or_default().push(k);}}
 for(k,n)in nodes.iter_mut().enumerate(){let mut it=reached.remove(&k).unwrap_or_default().into_iter();n.children=nulls[k].iter().map(|&null|if null{None}else{it.next()}).collect();}
 let textures=nif.objects.iter().enumerate().filter(|(_,o)|o.class=="NiSourceTexture").map(|(i,o)|->Result<Texture>{Ok(Texture{block:i as u32,
  filename:if o.fields.u8("use_external").unwrap_or(0)!=0{Some(text(o.fields.string("file_name").flatten())?)}else{None},extras:extras(nif,i)?,generated:BTreeMap::new()})}).collect::<Result<Vec<_>>>()?;
 Ok(Document{schema:2,source:source.into(),source_sha256:source_sha256.into(),converted_sha256,material_audit_sha256,version:nif.version,roots:vec![Some(0)],nodes,textures,mesh_mapping})
}
