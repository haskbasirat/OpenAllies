use super::*;
use serde_json::{Value,json};
use std::collections::BTreeMap;
struct Lab<'a>{c:&'a Value,pc:aa_numeric::Precision,words:BTreeMap<u32,u32>,events:Vec<Value>,pairs:Vec<(Vec<u16>,Vec<u16>)>,capacity:usize,payload:bool,stopped:Option<String>}
impl Lab<'_>{
 fn event(&mut self,name:&str,args:&[Value])->Result<()>{let mut event=vec![json!(name)];event.extend_from_slice(args);self.events.push(json!(event));let fail=self.c["fail"].as_str();
  if fail==Some(name)||(fail==Some("texture1")&&name=="texture"&&args[0]==1){let reason=fail.unwrap().to_owned();self.stopped=Some(reason.clone());return Err(reason)}Ok(())}
 fn word_in(&self,n:&str)->u32{self.c[n].as_u64().unwrap()as u32}fn bool_in(&self,n:&str)->bool{self.c[n].as_bool().unwrap()}
}
impl Fields for Lab<'_>{
 fn no_ni_objects(&mut self)->Result<u8>{self.event("no_ni",&[])?;Ok(self.word_in("no_ni")as u8)}
 fn word(&mut self,o:u32)->Result<u32>{self.words.get(&o).copied().ok_or_else(||format!("missing test field{o:x}"))}
 fn store(&mut self,o:u32,v:u32)->Result<()>{self.events.push(json!(["write",o,v]));self.words.insert(o,v);Ok(())}
 fn filename_length(&mut self)->Result<u32>{self.event("filename",&[])?;Ok(if self.bool_in("filename"){self.c["model_name"].as_str().unwrap().encode_utf16().count()as u32}else{0})}
 fn begin(&mut self)->Result<()>{self.event("begin",&[])}fn open_input(&mut self)->Result<()>{self.event("open",&[])}
 fn input_size(&mut self)->Result<u32>{self.event("size",&[])?;crate::ni_cache_math::weighted_size_constant(self.word_in("bytes"),0x3fa8f5c3,self.pc)}
 fn decode_input(&mut self)->Result<()>{self.event("decode",&[])?;self.payload=self.bool_in("payload");Ok(())}
 fn close_input(&mut self)->Result<()>{self.event("close",&[])}fn default_payload(&mut self)->Result<()>{self.event("default",&[])?;self.payload=self.bool_in("payload");Ok(())}
 fn payload_present(&mut self)->Result<bool>{self.event("present",&[])?;Ok(self.payload)}
 fn unable_to_load(&mut self)->Result<()>{self.event("unable",&[])?;self.stopped=Some("unable".into());Err("Unable to load AVObject".into())}
 fn loading_context(&mut self)->Result<()>{self.event("context",&[])}
 fn hardware_model(&mut self)->Result<bool>{self.event("hardware",&[])?;Ok(self.bool_in("hardware"))}
 fn hardware_count(&mut self)->Result<u32>{self.event("hardware_count",&[])?;Ok(self.word_in("hardware_count"))}
 fn hardware_transform(&mut self,c:u32)->Result<()>{self.event("hardware_transform",&[json!(c)])}
 fn process_metadata(&mut self,skin:bool)->Result<()>{self.event("metadata",&[json!(u32::from(skin))])}
 fn metadata_word(&mut self,o:u32)->Result<u32>{Ok(match o{4=>self.word_in("value48"),8=>self.word_in("radius"),12=>self.word_in("max_z"),16=>self.word_in("min_z"),20=>self.word_in("footprint"),24=>self.c["center"][0].as_u64().unwrap()as u32,28=>self.c["center"][1].as_u64().unwrap()as u32,32=>self.c["textures"].as_array().unwrap().len()as u32,_=>return Err("bad test metadata word".into())})}
 fn metadata_byte(&mut self,o:u32)->Result<u8>{Ok(self.word_in(if o==1{"byte1"}else{"byte24"})as u8)}
 fn texture_capacity(&mut self,count:u32)->Result<()>{self.capacity=crate::model_texture_path::capacity(self.capacity,count as usize)?;Ok(())}
 fn collect_texture_names(&mut self)->Result<()>{self.event("names",&[])}
 fn append_texture(&mut self,index:u32)->Result<()>{self.event("texture",&[json!(index)])?;let model=self.c["model_name"].as_str().unwrap().encode_utf16().collect::<Vec<_>>();let name=self.c["textures"][index as usize].as_str().unwrap().encode_utf16().collect::<Vec<_>>();self.pairs.push(crate::model_texture_path::pair(&model,&name)?);self.store(0x84,self.pairs.len()as u32)}
 fn profile_enabled(&mut self)->Result<bool>{Ok(false)}fn profile(&mut self,label:&'static str)->Result<()>{self.event("profile",&[json!(label)])}
 fn prepare_effect(&mut self,code:u32)->Result<()>{self.event("effect",&[json!(code)])}
 fn skip_loaded_bounds(&mut self)->Result<u8>{self.event("skip",&[])?;Ok(self.word_in("skip")as u8)}
 fn controllers(&mut self)->Result<bool>{self.event("controllers",&[])?;Ok(self.bool_in("controllers"))}
 fn selection_cuboid(&mut self)->Result<()>{self.event("selection",&[])?;for(i,v)in self.c["selection"].as_array().unwrap().iter().enumerate(){self.words.insert(0x4c+4*i as u32,v.as_u64().unwrap()as u32);}Ok(())}
 fn wrap_root(&mut self)->Result<()>{self.event("wrap",&[])}fn update_root(&mut self)->Result<()>{self.event("update",&[])}fn property_name_length(&mut self)->Result<u32>{Ok(0)}fn apply_named_property(&mut self)->Result<()>{Err("unexpected test property".into())}
 fn publish_payload(&mut self)->Result<()>{self.store(0x40,1)}fn display_profile(&mut self)->Result<()>{self.event("display",&[])}
}
fn fixture(name:&str)->Value{let p=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-model-resource-load").join(name);serde_json::from_str(&std::fs::read_to_string(p).unwrap()).unwrap()}
#[test]fn original_v14_model_stores_branches_and_failure_prefixes(){
 let data=fixture("native-model-load.json");for row in data["cases"].as_array().unwrap(){let c=&row["input"];let pc=match row["pc"].as_str().unwrap(){"Single24"=>aa_numeric::Precision::Single24,"Double53"=>aa_numeric::Precision::Double53,_=>aa_numeric::Precision::Extended64};
  let mut words=BTreeMap::new();for o in [8,12,16,64,72,76,80,84,88,92,96,100,104,108,112,116,120,124,132]{words.insert(o,0);}
  for(o,v)in[(8,c["flags"].as_u64().unwrap()as u32),(12,17),(16,19),(72,0x41200000),(100,0x3f800000),(104,0x3f800000),(108,0x3f800000),(112,0x3f800000),(116,0x3f800000)]{words.insert(o,v);}
  let mut lab=Lab{c,pc,words,events:vec![],pairs:vec![],capacity:1,payload:false,stopped:None};let mut state=State::new_call();let result=state.load(&mut lab).ok().map(u32::from);
  let pairs=lab.pairs.iter().map(|(a,b)|[String::from_utf16(a).unwrap(),String::from_utf16(b).unwrap()]).collect::<Vec<_>>();
  let actual=json!({"events":lab.events,"words":lab.words.iter().map(|(k,v)|(k.to_string(),*v)).collect::<BTreeMap<_,_>>(),"pairs":pairs,"capacity":lab.capacity,"stopped":lab.stopped,"result":result});assert_eq!(actual,row["output"],"{row}");
 }
}
#[test]fn original_v14_ni_cache_size_and_filename_hash(){let data=fixture("native-ni-cache.json");for row in data["rows"].as_array().unwrap(){if !row["entry"].as_bool().unwrap(){continue}let pc=match row["pc"].as_str().unwrap(){"Single24"=>aa_numeric::Precision::Single24,"Double53"=>aa_numeric::Precision::Double53,_=>aa_numeric::Precision::Extended64};assert_eq!(crate::ni_cache_math::weighted_size(row["bytes"].as_u64().unwrap()as u32,pc).unwrap(),row["cache_size"].as_u64().unwrap()as u32);assert_eq!(crate::ni_cache_math::filename_hash(&row["name"].as_str().unwrap().encode_utf16().collect::<Vec<_>>()),row["hash"].as_u64().unwrap()as u32);}}
#[test]fn original_v14_model_class_weights_and_cached_lookup(){let data=fixture("native-model-class.json");for row in data["rows"].as_array().unwrap(){let current=row["ram_current"].as_i64().map(|v|v as i32);let(ram,vram)=crate::ni_cache_math::class_weights(current).unwrap();assert_eq!(ram as i64,row["ram_weight"].as_i64().unwrap());assert_eq!(vram as i64,row["vram_weight"].as_i64().unwrap());assert_eq!(row["class_count"],1);assert_eq!(row["cached_lookup_count"],0);assert_eq!(row["cached_same_allocation"],true);assert_eq!(row["class_name"],"KKC_NiAVObjResource");}assert!(crate::ni_cache_math::class_weights(Some(0)).is_err());assert!(crate::ni_cache_math::class_weights(Some(-1)).is_err());}
