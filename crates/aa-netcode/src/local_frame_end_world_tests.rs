use super::*;
use serde_json::{json,Value};
struct Replay{input:Value,events:Vec<Value>,getters:Vec<Value>,notified:u8,phase:&'static str,online_reads:usize}
impl Replay{
    fn cut(&self,name:&str)->Result<()>{if self.input["failure"]==name{Err(missing(name))}else{Ok(())}}
}
impl all_tasks_complete::OppositionFields for Replay{
    type Kingdom=usize;type Team=u32;
    fn count(&mut self)->usize{self.input["players"].as_array().unwrap().len()}
    fn ai(&mut self,i:usize)->Result<bool>{Ok(self.input["players"][i]["ai"].as_bool().unwrap())}
    fn kingdom(&mut self,i:usize)->Result<Option<usize>>{
        self.getters.push(json!({"phase":self.phase,"player":i}));
        Ok(self.input["players"][i]["kingdom"].as_u64().map(|v|v as usize))
    }
    fn result(&mut self,i:usize)->Result<u32>{Ok(self.input["kingdoms"][i]["result"].as_u64().unwrap() as u32)}
    fn team(&mut self,i:usize)->Result<Option<u32>>{let v=self.input["kingdoms"][i]["team"].as_u64().unwrap() as u32;Ok((v!=0).then_some(v))}
}
impl Fields for Replay{
    type Player=usize;
    fn enabled(&mut self)->Result<bool>{self.events.push(json!("read16d"));Ok(self.input["enabled"].as_u64().unwrap()!=0)}
    fn opposition(&mut self)->Result<bool>{self.events.push(json!("opposition"));self.cut("opposition")?;self.phase="opposition";all_tasks_complete::opposition(self)}
    fn notified(&mut self)->Result<bool>{self.events.push(json!("read16e"));Ok(self.notified!=0)}
    fn store_notified(&mut self)->Result<()>{self.events.push(json!("store16e"));self.notified=1;Ok(())}
    fn world48(&mut self)->Result<u32>{self.cut("world")?;self.events.push(json!("world48"));Ok(self.input["world48"].as_u64().unwrap() as u32)}
    fn first_active(&mut self)->Result<Option<usize>>{self.events.push(json!("first_active"));self.cut("first_active")?;self.phase="first_active";all_tasks_complete::first_active_index(self)}
    fn online(&mut self)->Result<bool>{self.online_reads+=1;Ok(self.input[if self.online_reads==1{"online"}else{"online_after"}].as_bool().unwrap())}
    fn localized(&mut self,key:&str)->Result<Vec<u16>>{self.events.push(json!({"lookup":key}));self.cut("lookup")?;Ok("No human opponents remain.".encode_utf16().collect())}
    fn format_message(&mut self,text:Vec<u16>)->Result<Vec<u16>>{self.events.push(json!({"format":String::from_utf16(&text).unwrap()}));self.cut("format")?;Ok(text)}
    fn show(&mut self,text:&[u16])->Result<()>{self.events.push(json!({"ui":15,"text":String::from_utf16(text).unwrap()}));self.cut("ui")}
    fn gamespy(&mut self,p:usize)->Result<()>{self.events.push(json!({"gamespy":format!("kingdom{p}")}));self.cut("gamespy")}
}
#[test]
fn native_positive_end_order_flags_getters_and_callback_failures(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-positive-end-condition.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(),618);
    for row in fixture["rows"].as_array().unwrap(){
        let i=&row["input"];let mut f=Replay{input:i.clone(),events:vec![],getters:vec![],notified:i["notified"].as_u64().unwrap() as u8,phase:"",online_reads:0};
        let result=process(&mut f);let out=&row["output"];
        assert_eq!(result.is_err(),!out["stop"].is_null(),"{row}");
        assert_eq!(json!(f.events),out["events"],"{row}");assert_eq!(json!(f.getters),out["getters"],"{row}");
        assert_eq!(json!(f.notified),out["notified"],"{row}");
    }
}
#[test]
fn required_ui_does_not_claim_message_or_gamespy_success(){
    let mut ui=RequiredMessages;
    assert!(ui.show_message(&[]).unwrap_err().to_string().contains("actual40BC50"));
    assert!(ui.gamespy_remaining(&[]).unwrap_err().to_string().contains("actual793B40"));
}
