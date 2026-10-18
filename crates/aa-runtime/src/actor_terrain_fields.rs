//! One native4D4130 transition over borrowed held or stored Actor fields.
use aa_actors::actor_creation::{Error,Result};
pub(crate) fn gates(flags:u32,low:u32,high:u32)->bool{
    flags&(1<<29)==0&&aa_actors::overlap_list::eligible(low,high)
        &&low&(1<<24)==0&&flags&(1<<12)==0
}
pub(crate) trait Fields{
    type Property;
    fn flags_state(&self)->Result<(u32,u32,u32)>;
    fn current(&self)->Result<u32>;
    fn select(&mut self)->Result<u32>;
    fn store(&mut self,token:u32)->Result<()>;
    fn organization(&self)->Result<bool>;
    fn refresh_organization(&mut self)->Result<()>;
    fn properties(&self)->Result<bool>;
    fn property_list(&self,token:u32)->Result<Vec<Self::Property>>;
    fn remove(&mut self,property:&Self::Property)->Result<()>;
    fn applies(&mut self,property:&Self::Property)->Result<bool>;
    fn add(&mut self,property:&Self::Property,kind:u32)->Result<()>;
    fn null_diagnostic(&mut self)->Result<()>;
}
pub(crate) fn transition(fields:&mut impl Fields)->Result<()>{
    let(flags,low,high)=fields.flags_state()?;
    if !gates(flags,low,high){return Ok(())}
    let old=fields.current()?;
    let next=fields.select()?;
    fields.store(next)?;
    if next==old{return Ok(())}
    if fields.organization()?{fields.refresh_organization()?;}
    if !fields.properties()?{return Ok(())}
    if old!=0{
        for property in fields.property_list(old)?{fields.remove(&property)?;}
    }
    let next=fields.current()?;
    if next==0{
        fields.null_diagnostic()?;
        return Err(Error::NullDereference("4D438D TerrainData+B0"))
    }
    for property in fields.property_list(next)?{
        if fields.properties()?&&fields.applies(&property)?{fields.add(&property,5)?;}
    }
    Ok(())
}
#[cfg(test)]
mod tests{
    use super::*;
    use serde_json::Value;
    struct Probe<'a>{input:&'a Value,current:u32,stop:Option<&'static str>}
    impl Probe<'_>{fn required<T>(&mut self,s:&'static str)->Result<T>{self.stop=Some(s);Err(Error::Unsupported(s))}}
    fn token(v:&Value)->u32{v.as_u64().map(|v|v as u32+1).unwrap_or(0)}
    impl Fields for Probe<'_>{
        type Property=u32;
        fn flags_state(&self)->Result<(u32,u32,u32)>{Ok((self.input["flags"].as_u64().unwrap()as u32,self.input["state_low"].as_u64().unwrap()as u32,self.input["state_high"].as_u64().unwrap()as u32))}
        fn current(&self)->Result<u32>{Ok(self.current)}
        fn select(&mut self)->Result<u32>{Ok(if self.input["element"]=="active_parent"{token(&self.input["parent_terrain"])}else{self.input["tile"].as_u64().unwrap()as u32+1})}
        fn store(&mut self,t:u32)->Result<()>{self.current=t;Ok(())}
        fn organization(&self)->Result<bool>{Ok(self.input["organization"].as_bool().unwrap())}
        fn refresh_organization(&mut self)->Result<()>{self.required("required_Organization579EB0")}
        fn properties(&self)->Result<bool>{Ok(self.input["weapon"].as_bool().unwrap())}
        fn property_list(&self,t:u32)->Result<Vec<u32>>{Ok(self.input["properties"][(t-1)as usize].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect())}
        fn remove(&mut self,_:&u32)->Result<()>{self.required("required_Weapon581810")}
        fn applies(&mut self,p:&u32)->Result<bool>{if *p==0{Ok(false)}else{self.required("required_Property5035E0")}}
        fn add(&mut self,_:&u32,_:u32)->Result<()>{panic!("fixture stops before positive Property5035E0")}
        fn null_diagnostic(&mut self)->Result<()>{self.required("required_null_terrain_diagnostic")}
    }
    #[test]
    fn shared_terrain_transition_matches186_native_outcomes_and_committed_f8(){
        let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261016-actor-terrain-refresh/native-actor-terrain-refresh.json")).unwrap();
        let rows=fixture["cases"].as_array().unwrap();assert_eq!(rows.len(),186);
        for row in rows{
            let mut fields=Probe{input:&row["input"],current:token(&row["input"]["old"]),stop:None};
            let result=transition(&mut fields);
            let outcome=if result.is_ok(){"returned"}else{fields.stop.expect("only declared fixture stops")};
            assert_eq!(outcome,row["output"]["outcome"],"{row}");
            assert_eq!(fields.current,token(&row["output"]["current_terrain"]),"{row}");
        }
    }
}
