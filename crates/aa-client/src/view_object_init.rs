//!Actual457A40 ordering over typed retained source/model/view owners.
//!The source factory, radius-grid child and common Init remain reached
//!operations; this kernel neither publishes Thing+C nor grants VWorld ready.
use anyhow::Result;
use aa_numeric::{Binary,Precision};
pub(super) trait Services{
    type Aspect;
    type Model;
    fn variations(&mut self)->Result<bool>;
    fn thing_present(&mut self)->Result<bool>;
    fn thing_variant(&mut self)->Result<u32>;
    fn actor_present(&mut self)->Result<bool>;
    fn under_construction(&mut self)->Result<bool>;
    fn construction_count(&mut self)->Result<u32>;
    fn first_construction_aspect(&mut self)->Result<u32>;
    fn stance_present(&mut self)->Result<bool>;
    fn stance_aspect(&mut self)->Result<u32>;
    fn default_aspect(&mut self)->Result<u32>;
    fn random_aspect(&mut self)->Result<u32>;
    fn index(&self)->Result<u32>;
    fn store_index(&mut self,index:u32)->Result<()>;
    fn aspect(&mut self)->Result<Self::Aspect>;
    fn model_for_aspect(&mut self,aspect:&Self::Aspect)->Result<Self::Model>;
    fn store_model(&mut self,model:Self::Model)->Result<()>;
    fn model_flags(&mut self)->Result<u32>;
    fn model_radius(&mut self)->Result<u32>;
    fn aspect_radius(&mut self,aspect:&Self::Aspect)->Result<u32>;
    fn thing_scale(&mut self)->Result<u32>;
    fn set_radius(&mut self,radius:u32)->Result<()>;
    fn common_init(&mut self)->Result<bool>;
}
pub(super) fn initialize(s:&mut impl Services,pc:Precision)->Result<bool>{
    if s.variations()?{
        if s.thing_present()?{
            let index=s.thing_variant()?;s.store_index(index)?;
        }else if s.index()?==u32::MAX{
            let index=s.random_aspect()?;s.store_index(index)?;
        }
    }else{
        let actor=s.actor_present()?;
        if actor&&s.under_construction()?&&s.construction_count()?!=0{
            let index=s.first_construction_aspect()?;s.store_index(index)?;
        }
        if s.index()?==u32::MAX{
            let index=if actor&&s.stance_present()?{s.stance_aspect()?}else{s.default_aspect()?};
            s.store_index(index)?;
        }
    }
    let aspect=s.aspect()?;
    let model=s.model_for_aspect(&aspect)?;
    s.store_model(model)?;
    let mut radius=if s.model_flags()?&4==0{
        //The native virtual104 getter is intentionally invoked AGAIN.
        let aspect=s.aspect()?;s.aspect_radius(&aspect)?
    }else{s.model_radius()?};
    if s.thing_present()?{
        radius=Binary::from_f32_bits(s.thing_scale()?)?
            .mul(&Binary::from_f32_bits(radius)?,pc)?.store_f32()?;
    }
    s.set_radius(radius)?;
    s.common_init()
}

#[cfg(test)]mod tests{
    use super::*;
    struct Replay<'a>{c:&'a serde_json::Value,index:u32,model:bool,radius:u32,events:Vec<serde_json::Value>}
    impl Replay<'_>{
        fn word(&self,k:&str)->u32{self.c[k].as_u64().unwrap()as u32}
        fn actor(&self)->bool{matches!(self.c["thing"].as_str(),Some("Actor"|"Kingdom"))}
    }
    impl Services for Replay<'_>{
        type Aspect=u32;type Model=();
        fn variations(&mut self)->Result<bool>{Ok(self.word("variations")!=0)}
        fn thing_present(&mut self)->Result<bool>{Ok(self.c["thing"]!="None")}
        fn thing_variant(&mut self)->Result<u32>{Ok(self.word("variant"))}
        fn actor_present(&mut self)->Result<bool>{Ok(self.actor())}
        fn under_construction(&mut self)->Result<bool>{Ok(self.word("state")&(1<<21)!=0)}
        fn construction_count(&mut self)->Result<u32>{Ok(self.word("construction_count"))}
        fn first_construction_aspect(&mut self)->Result<u32>{Ok(2)}
        fn stance_present(&mut self)->Result<bool>{Ok(self.word("stance")!=u32::MAX)}
        fn stance_aspect(&mut self)->Result<u32>{assert_eq!(self.word("stance"),1);Ok(3)}
        fn default_aspect(&mut self)->Result<u32>{Ok(0)}
        fn random_aspect(&mut self)->Result<u32>{self.events.push(serde_json::json!(["random"]));anyhow::bail!("49B900")}
        fn index(&self)->Result<u32>{Ok(self.index)}
        fn store_index(&mut self,index:u32)->Result<()>{self.index=index;Ok(())}
        fn aspect(&mut self)->Result<u32>{assert!(self.index<4);Ok(self.index)}
        fn model_for_aspect(&mut self,aspect:&u32)->Result<()>{self.events.push(serde_json::json!(["model",aspect]));anyhow::ensure!(self.c["cutoff"]!="model","454F70");Ok(())}
        fn store_model(&mut self,():())->Result<()>{self.model=true;Ok(())}
        fn model_flags(&mut self)->Result<u32>{assert!(self.model);Ok(self.word("model_flags"))}
        fn model_radius(&mut self)->Result<u32>{Ok(self.word("model_radius"))}
        fn aspect_radius(&mut self,aspect:&u32)->Result<u32>{Ok([0x3fa00000,0x40200000,0xc0500000,0x3dcccccd][*aspect as usize])}
        fn thing_scale(&mut self)->Result<u32>{Ok(self.word("scale"))}
        fn set_radius(&mut self,radius:u32)->Result<()>{
            if f32::from_bits(self.radius)!=f32::from_bits(radius){
                if self.c["occupied"].as_bool().unwrap(){self.events.push(serde_json::json!(["grid",0x41280000u32,0xc0780000u32,self.radius,radius]));anyhow::bail!("4306C0")}
                self.radius=radius;
            }Ok(())
        }
        fn common_init(&mut self)->Result<bool>{self.events.push(serde_json::json!(["common"]));anyhow::ensure!(self.c["cutoff"]!="common","45B530");Ok(self.word("result")!=0)}
    }
    #[test]fn native_object_selection_model_radius_and_common_prefix_all_pc(){
        let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/development10/view-object-init/native-object-init.json")).unwrap();
        let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),648);
        for row in rows{
            let c=&row["input"];let mut s=Replay{c,index:c["initial_index"].as_u64().unwrap()as u32,model:false,radius:0,events:vec![]};
            let pc=match row["precision"].as_str().unwrap(){"Single"=>Precision::Single24,"Double"=>Precision::Double53,"Extended"=>Precision::Extended64,_=>unreachable!()};
            let result=initialize(&mut s,pc);let (value,error)=match result{Ok(v)=>(Some(u8::from(v)),None),Err(e)=>(None,Some(e.to_string()))};
            let actual=serde_json::json!({"index":s.index,"model":s.model,"radius":s.radius,
                "flags":if c["occupied"].as_bool().unwrap(){0x1000020u32}else{0x1000000u32},"events":s.events,"error":error,"result":value});
            assert_eq!(actual,row["output"],"{} PC{} {c}",row["image"],row["precision"]);
        }
    }
}
