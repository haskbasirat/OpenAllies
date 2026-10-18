use super::*;
use serde_json::{json,Value};
use std::collections::BTreeMap;
struct Children{input:Value,events:Vec<String>,textures:BTreeMap<u32,Option<String>>,effects:BTreeMap<u32,Option<String>>}
impl Children{
    fn event(&mut self,event:String)->Result<()>{if self.input["stop"].as_str()==Some(&event){return Err(format!("error:{event}"))}self.events.push(event);Ok(())}
    fn contains(&self,list:&str,key:&str)->bool{self.input[list].as_array().unwrap().iter().any(|v|v.as_str()==Some(key))}
}
impl Services for Children{
    type Alias=String;type Texture=String;type Effect=String;
    fn environment_43ea50(&mut self)->Result<()>{self.event("environment".into())}
    fn alias_410cb0(&mut self,key:&'static str)->Result<String>{self.event(format!("lookup:{key}"))?;Ok(key.into())}
    fn texture_5a77e0(&mut self,alias:&String,flags:u32)->Result<Option<String>>{assert_eq!(flags,0);self.event(format!("texture:{alias}"))?;Ok((!self.contains("missing",alias)).then(||alias.clone()))}
    fn effect_4bdb70(&mut self,alias:&String)->Result<Option<String>>{self.event(format!("fx:{alias}"))?;Ok((!self.contains("missing",alias)).then(||alias.clone()))}
    fn store_texture(&mut self,slot:TextureSlot,value:Option<String>){self.textures.insert(slot.offset(),value);}
    fn store_effect(&mut self,slot:EffectSlot,value:Option<String>){self.effects.insert(slot.offset(),value);}
    fn effect_present(&self,slot:EffectSlot)->bool{self.effects[&slot.offset()].is_some()}
    fn effect_kind(&self,slot:EffectSlot)->Result<u32>{Ok(if self.contains("wrong_kind",self.effects[&slot.offset()].as_ref().unwrap()){3}else{12})}
    fn warning_404ea0(&mut self,line:u32,text:&'static str)->Result<()>{assert!(matches!(line,0x161|0x166|0x16b));self.event(format!("warning:{text}"))}
}
#[test]
fn original_v14_ordered_assets_stores_nulls_warnings_and_child_failures(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/development10/native-view-assets.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),219);
    for row in rows{
        let mut s=Children{input:row["input"].clone(),events:vec![],textures:TEXTURES.into_iter().map(|v|(v.offset(),Some("old".into()))).collect(),
            effects:EFFECTS.into_iter().chain([EffectSlot::DamageNumbers,EffectSlot::HealNumbers]).map(|v|(v.offset(),Some("old".into()))).collect()};
        let result=initialize(&mut s);let outcome=result.err().unwrap_or_else(||"segment_complete".into());
        let output=json!({"outcome":outcome,"events":s.events,"textures":TEXTURES.map(|v|s.textures[&v.offset()].clone()),
            "fx":EFFECTS.into_iter().chain([EffectSlot::DamageNumbers,EffectSlot::HealNumbers]).map(|v|s.effects[&v.offset()].clone()).collect::<Vec<_>>(),"ad":0});
        assert_eq!(output,row["output"],"{row}");
    }
}
