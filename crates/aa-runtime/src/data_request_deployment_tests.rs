//!Replays research/skirmish-20261018-actor-deployment-534 (ORIGINAL4624B0
//!slice,464BD0 MobileDeployment block,475230/461840/4605B0 and recursive
//!45DD80 under Unicorn) through the Rust source writer, the aa-rules deferred
//!pass and this45DD80 dispatcher. Synthetic and shipped rows alike.
use super::*;
use crate::data_pre_request::actor_request::deployment::{self,Block,Source};
use aa_numeric::Precision;
use aa_rules::native_mobile_deployment::{Definition,NativeMobileDeploymentData};
use serde_json::{Value,json};
use std::collections::BTreeMap;

struct Recorded<'a>{c:&'a Value,reads:Vec<Value>}
impl Source for Recorded<'_>{
    fn string(&mut self,key:&'static str)->Result<String>{self.reads.push(json!(["5B54D0",key]));Ok(self.c[key].as_str().unwrap().into())}
    fn fixed(&mut self,key:&'static str)->Result<u32>{self.reads.push(json!(["5B5130",key]));Ok(self.c[key].as_u64().unwrap() as u32)}
    fn point(&mut self,key:&'static str)->Result<[u32;2]>{
        self.reads.push(json!(["5B4D80",key]));let v=&self.c[key];Ok([v[0].as_u64().unwrap() as u32,v[1].as_u64().unwrap() as u32])
    }
}
struct World{masks:BTreeMap<String,u32>,links:BTreeMap<String,Option<String>>,counters:BTreeMap<String,u32>,trace:Vec<Value>}
struct At<'w>{w:&'w mut World,id:String}
impl At<'_>{fn fx(&mut self,call:String,d:u32,m:u32)->Result<()>{self.w.trace.push(json!({"actor":self.id,"call":call,"args":[d,m]}));Ok(())}}
fn empty<T>()->Result<T>{Err("oracle child list is empty".into())}
impl ObjectWorld for At<'_>{
    type Data=String;type Aspect=();type Model=();
    fn base_request(&mut self,d:u32,_:u32)->Result<()>{let c=self.w.counters.entry(self.id.clone()).or_insert(0);*c=c.wrapping_add(d);Ok(())}
    fn portrait(&mut self)->Result<Option<String>>{Ok(None)}
    fn request_data(&mut self,data:String,d:u32,m:u32)->Result<()>{actor(&mut At{w:&mut *self.w,id:data},d,m)}
    fn aspect_count(&mut self)->Result<isize>{Ok(0)}
    fn aspect(&mut self,_:usize)->Result<()>{empty()}
    fn request_aspect(&mut self,_:(),_:u32,_:u32)->Result<()>{empty()}
    fn flags_1c(&mut self)->Result<u32>{Ok(0)}
    fn unique_fc(&mut self)->Result<bool>{Ok(false)}
    fn current_aspect_model(&mut self,_:usize)->Result<()>{empty()}
    fn or_model_flags(&mut self,_:(),_:u32)->Result<()>{empty()}
    fn request_texture(&mut self,_:u32,_:u32,_:u32)->Result<()>{Ok(())}
    fn request_banner(&mut self,_:u32,_:u32)->Result<()>{Ok(())}
    fn request_view_fx(&mut self,i:usize,d:u32,m:u32)->Result<()>{self.fx(format!("viewfx{i}"),d,m)}
}
impl ActorWorld for At<'_>{
    type AnimationSet=();type Animation=();type Ability=();
    fn mask_174(&mut self)->Result<u32>{Ok(self.w.masks[&self.id])}
    fn mark_all_aspect_models(&mut self,_:u32)->Result<()>{Ok(())}
    fn animation_set_count(&mut self)->Result<isize>{Ok(0)}
    fn animation_set(&mut self,_:usize)->Result<Option<()>>{empty()}
    fn animation_count(&mut self,_:&())->Result<isize>{empty()}
    fn animation(&mut self,_:&(),_:usize)->Result<()>{empty()}
    fn request_animation(&mut self,_:(),_:u32)->Result<()>{empty()}
    fn request_actor_fx(&mut self,o:u32,d:u32,m:u32)->Result<()>{self.fx(format!("actorfx{o}"),d,m)}
    fn ability_count(&mut self,_:bool)->Result<isize>{Ok(0)}
    fn ability(&mut self,_:bool,_:usize)->Result<()>{empty()}
    fn request_ability(&mut self,_:(),_:u32,_:u32)->Result<()>{empty()}
    fn extra_ability_2a4(&mut self)->Result<Option<()>>{Ok(None)}
    fn deployment_data_534(&mut self)->Result<Option<String>>{Ok(self.w.links[&self.id].clone())}
}
fn precision(v:&Value)->Precision{match v.as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,p=>panic!("{p}")}}
fn words(b:&Block)->Value{
    json!({"538":b.packed_position_offset_bits[0],"53c":b.packed_position_offset_bits[1],"540":b.packed_orientation_bits[0],
        "544":b.packed_orientation_bits[1],"54c":b.pack_bits[0],"550":b.pack_bits[1],"554":b.pack_bits[2]})
}

#[test]
fn native_actor_deployment_534_fixture_replay(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261018-actor-deployment-534/native-actor-deployment-534.json")).unwrap();
    assert_eq!(fixture["passed"],true);
    let (mut shipped,mut requests)=(0,0);
    for case in fixture["cases"].as_array().unwrap(){
        let (input,output,label)=(&case["input"],&case["output"],case["input"]["label"].as_str().unwrap());
        let pc=precision(&input["precision"]);
        let aliases=input.get("aliases").and_then(Value::as_object).cloned().unwrap_or_default();
        let actors=input["actors"].as_array().unwrap();
        let (mut blocks,mut masks,mut created)=(BTreeMap::new(),BTreeMap::new(),Vec::new());
        let mut error:Option<(String,String)>=None;
        //464BD0 block for every actor in load order; a410C40 aborts the load.
        for a in actors{
            let id=a["id"].as_str().unwrap().to_string();
            let mut mask=a["mask_before"].as_u64().unwrap() as u32;
            let mut reads=vec![json!(["5B4410",deployment::COMPONENT,0])];
            let mut block=Block::constructed();
            if !a["component"].is_null(){
                mask|=deployment::MOBILE_DEPLOYMENT_BIT;
                let mut source=Recorded{c:&a["component"],reads:vec![]};
                block=deployment::read_block(&mut source).unwrap();reads.extend(source.reads);
                if let Some(r)=&block.reference{created.push(json!([id,r]));}
                if let Err(e)=deployment::validate_block(&block,&id){error=Some((id.clone(),e));}
            }
            assert_eq!(Value::Array(reads),output["reads"][&id],"{label} {id} reads");
            blocks.insert(id.clone(),(!a["component"].is_null()).then_some(block));masks.insert(id.clone(),mask);
            if error.is_some(){break}
        }
        assert_eq!(Value::Array(created.clone()),output["created"],"{label} deferred creation");
        for a in actors{
            let id=a["id"].as_str().unwrap();let word=&output["actors"][id];
            let b=blocks.get(id).cloned().flatten().unwrap_or_else(Block::constructed);
            for(k,v)in words(&b).as_object().unwrap(){assert_eq!(&word[k],v,"{label} {id} +{k}");}
            let mask=masks.get(id).copied().unwrap_or(a["mask_before"].as_u64().unwrap() as u32);
            assert_eq!(word["174"].as_u64().unwrap() as u32,mask,"{label} {id} +174");
        }
        if let Some((id,e))=error{
            assert_eq!(output["error"]["phase"],"load","{label}");assert_eq!(output["error"]["actor"],id.as_str());
            assert!(e.contains(output["error"]["format"].as_str().unwrap()),"{label}: {e}");
            for a in actors{assert!(output["actors"][a["id"].as_str().unwrap()]["534"].is_null())}
            continue;
        }
        //475230 walks the473AD0 list head first: reverse creation order.
        let mut data=NativeMobileDeploymentData::default();
        for a in actors{
            let id=a["id"].as_str().unwrap().to_string();let mask=masks[&id];let b=blocks[&id].clone().unwrap_or_else(Block::constructed);
            data.definitions.insert(id.clone(),Definition{id:id.clone(),body:mask&1!=0,mobile:mask&deployment::MOBILE_DEPLOYMENT_BIT!=0,
                element:mask&0x100!=0,packed_position_offset_bits:b.packed_position_offset_bits,pack_health_min_bits:b.pack_bits[0],
                unpack_to:None,pack_to:None,soft_radius_bits:Some(a["soft_radius"].as_u64().unwrap() as u32),sources:vec![]});
        }
        let mut deferred=None;
        for pair in created.iter().rev(){
            let (source,name)=(pair[0].as_str().unwrap(),pair[1].as_str().unwrap());
            let found=aliases.get(name).and_then(Value::as_str).unwrap_or(name);
            let target=data.definitions.contains_key(&found.to_ascii_lowercase()).then_some(found);
            if let Err(e)=data.resolve_reference(source,target,pc){deferred=Some(e.to_string());break}
        }
        let lookups:Vec<Value>=output["lookups"].as_array().unwrap().iter().map(|l|l[0].clone()).collect();
        assert_eq!(lookups,data.resolution_order.iter().map(|(s,_)|json!(s)).collect::<Vec<_>>(),"{label} 4BDBA0 order");
        assert!(output["lookups"].as_array().unwrap().iter().all(|l|l[2]==0x2b));
        for a in actors{
            let id=a["id"].as_str().unwrap();let d=&data.definitions[id];
            assert_eq!(output["actors"][id]["534"].as_str(),d.unpack_to.as_deref(),"{label} {id} +534");
            assert_eq!(output["actors"][id]["548"].as_str(),d.pack_to.as_deref(),"{label} {id} +548");
        }
        match(&deferred,output["error"]["format"].as_str()){
            (Some(e),Some(f))=>{assert_eq!(output["error"]["phase"],"deferred");assert!(e.contains(f),"{label}: {e} / {f}");continue}
            (None,None)=>{}
            other=>panic!("{label}: {other:?}"),
        }
        //Production+534 accessor, then the45DD80 dispatcher with the target's
        //own Data request re-entered at mode0.
        let mut world=World{masks:masks.clone(),links:BTreeMap::new(),counters:BTreeMap::new(),trace:vec![]};
        for a in actors{
            let id=a["id"].as_str().unwrap();
            let link=deployment::forward_534(masks[id],id,blocks[id].as_ref(),(masks[id]&deployment::MOBILE_DEPLOYMENT_BIT!=0).then_some(&data)).unwrap();
            assert_eq!(link.as_deref(),output["actors"][id]["534"].as_str(),"{label} {id}");
            world.links.insert(id.to_string(),link);
        }
        let mut expected=output["requests"].as_array().unwrap().iter();
        for args in input["requests"].as_array().unwrap(){
            let (d,m)=(args[0].as_u64().unwrap() as u32,args[1].as_u64().unwrap() as u32);
            for a in actors{
                let id=a["id"].as_str().unwrap();world.trace.clear();
                actor(&mut At{w:&mut world,id:id.into()},d,m).unwrap();
                let row=expected.next().unwrap();assert_eq!(row["actor"],id);
                assert_eq!(Value::Array(world.trace.clone()),row["trace"],"{label} {id} request {d}/{m}");
                requests+=1;
            }
        }
        assert!(expected.next().is_none());
        for a in actors{let id=a["id"].as_str().unwrap();assert_eq!(output["actors"][id]["20"].as_u64().unwrap() as u32,world.counters[id],"{label} {id} +20");}
        if input["shipped"]==true{shipped+=actors.len();}
    }
    assert!(requests>=200,"{requests}");
    println!("replayed shipped actors {shipped}, requests {requests}");
}
