//!451F90 suffix after the real457A40 result; Organization44F320 forwards to
//!this same algorithm. This grants no publication or renderer completion.
use anyhow::Result;
use aa_numeric::Precision;

pub(super) trait Services{
    type TrackArray;
    fn base_init(&mut self)->Result<bool>;
    fn actor_present(&mut self)->Result<bool>;
    fn set_current_owner_color(&mut self)->Result<()>;
    fn position(&self)->Result<[u32;3]>;
    fn facing(&self)->Result<[u32;3]>;
    fn store_tail(&mut self,offset:usize,word:u32)->Result<()>;
    fn source_actor_38(&mut self)->Result<bool>;
    fn track_count_260(&mut self)->Result<i32>;
    ///Constructors run before the returned allocation is stored at144. A
    ///failed child must retain its pending allocation in the concrete owner.
    fn construct_track_array(&mut self,count:i32)->Result<Self::TrackArray>;
    fn store_track_array(&mut self,array:Self::TrackArray)->Result<()>;
    fn current_track_count_148(&self)->Result<i32>;
    fn initialize_track(&mut self,index:usize)->Result<()>;
    fn ui_random_word_d4(&mut self)->Result<u32>;
}
pub(super) fn initialize(s:&mut impl Services,pc:Precision)->Result<bool>{
    if !s.base_init()?{return Ok(false)}
    if s.actor_present()?{s.set_current_owner_color()?;}
    //Keep each actual copy/store before the next reached child. These are
    //the current common fields, not the original placement or a new pose.
    let p=s.position()?;
    for base in [0xf0,0xd4]{for(i,word)in p.into_iter().enumerate(){s.store_tail(base+4*i,word)?;}}
    for(i,word)in s.facing()?.into_iter().enumerate(){s.store_tail(0xe0+4*i,word)?;}
    for offset in [0xfc,0xec,0x118,0x11c,0x120,0x124]{s.store_tail(offset,0)?;}
    if s.source_actor_38()?&&s.track_count_260()?>0{
        let count=s.track_count_260()?;s.store_tail(0x148,count as u32)?;
        let array=s.construct_track_array(count)?;s.store_track_array(array)?;
        let mut i=0usize;
        while (i as i64)<i64::from(s.current_track_count_148()?){s.initialize_track(i)?;i+=1;}
    }
    let random=s.ui_random_word_d4()?;
    let fraction=aa_simulation::native_unit_float(random,pc)?.0;
    s.store_tail(0x14c,fraction)?;Ok(true)
}

#[cfg(test)]mod tests{
    use super::*;
    use serde_json::{Value,json};
    struct Replay<'a>{c:&'a Value,words:std::collections::BTreeMap<usize,u32>,rng:aa_simulation::NativeRng,events:Vec<&'static str>}
    impl Services for Replay<'_>{
        type TrackArray=();
        fn base_init(&mut self)->Result<bool>{self.events.push("base457A40");Ok(self.c["base"]==1)}
        fn actor_present(&mut self)->Result<bool>{Ok(matches!(self.c["thing"].as_str(),Some("Actor"|"Kingdom")))}
        fn set_current_owner_color(&mut self)->Result<()>{
            let rgb=if self.c["owner"]==true{[0x3dcccccd,0x3f000000,0x3f800000]}else{[0x3e800000,0x3f400000,0x3f000001]};
            for(i,v)in rgb.into_iter().enumerate(){self.words.insert(0xc8+i*4,v);}Ok(())
        }
        fn position(&self)->Result<[u32;3]>{Ok([0x3f800001,0x80000000,0x47ffffff])}
        fn facing(&self)->Result<[u32;3]>{Ok([0xbf19999a,0x3f4ccccd,0])}
        fn store_tail(&mut self,o:usize,v:u32)->Result<()>{self.words.insert(o,v);Ok(())}
        fn source_actor_38(&mut self)->Result<bool>{Ok(self.c["source_actor"]==true)}
        fn track_count_260(&mut self)->Result<i32>{Ok(self.c["tracks"].as_i64().unwrap()as i32)}
        fn construct_track_array(&mut self,_:i32)->Result<()>{self.events.push("track42D7B0");anyhow::bail!("42D7B0")}
        fn store_track_array(&mut self,_:())->Result<()>{unreachable!()}
        fn current_track_count_148(&self)->Result<i32>{unreachable!()}
        fn initialize_track(&mut self,_:usize)->Result<()>{unreachable!()}
        fn ui_random_word_d4(&mut self)->Result<u32>{Ok(self.rng.next_u32())}
    }
    #[test]fn actual_actor_and_organization_suffix_prefixes_all_pc(){
        let fixture:Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-actor-init.json")).unwrap();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),456);
        for row in rows{
            let c=&row["input"];let offsets=[0xc8,0xcc,0xd0,0xd4,0xd8,0xdc,0xe0,0xe4,0xe8,0xec,0xf0,0xf4,0xf8,0xfc,0x118,0x11c,0x120,0x124,0x144,0x148,0x14c];
            let mut s=Replay{c,words:offsets.into_iter().map(|v|(v,0xa5a5a5a5)).collect(),rng:aa_simulation::NativeRng::new(c["seed"].as_u64().unwrap()as u32),events:vec![]};
            let pc=match row["precision"].as_str().unwrap(){"Single"=>Precision::Single24,"Double"=>Precision::Double53,"Extended"=>Precision::Extended64,_=>unreachable!()};
            let(value,error)=match initialize(&mut s,pc){Ok(v)=>(Some(u8::from(v)),None),Err(e)=>(None,Some(e.to_string()))};
            let fields=s.words.into_iter().map(|(k,v)|(format!("0x{k:x}"),json!(v))).collect::<serde_json::Map<String,Value>>();
            assert_eq!(json!({"result":value,"error":error,"fields":fields,"seed":s.rng.state,"events":s.events}),row["output"],"{c} {pc:?}");
        }
    }
}
