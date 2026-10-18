use aa_orders::{version_ack::{self,DescriptionWorld},Error,Result};
use serde_json::{json,Value};
struct Fields {world:bool,kingdom:bool,name:Vec<u16>,color:Vec<u16>,id:u32,events:Vec<Value>,fail:Option<&'static str>}
impl DescriptionWorld for Fields {
    fn has_current_kingdom(&mut self)->Result<bool>{let has=self.world&&self.kingdom;self.events.push(json!(["kingdom",has]));Ok(has)}
    fn player_name(&mut self)->Result<Vec<u16>>{self.events.push(json!(["name",self.name]));if self.fail==Some("name"){Err(Error::SchemaContext("required actual name"))}else{Ok(self.name.clone())}}
    fn kingdom_color_name(&mut self)->Result<Vec<u16>>{
        self.events.push(json!(["kingdom_again",self.world&&self.kingdom]));
        if self.fail==Some("color"){return Err(Error::SchemaContext("required actual Color name"))}
        self.events.push(json!(["color",self.color]));Ok(self.color.clone())
    }
    fn player_id(&mut self)->Result<u32>{self.events.push(json!(["id",self.id]));Ok(self.id)}
}
fn units(v:&Value)->Vec<u16>{v.as_array().unwrap().iter().map(|n|n.as_u64().unwrap()as u16).collect()}
#[test]
fn complete_native_7515a0_and_real_crt_outputs_match_all_pe_and_precision_rows(){
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-order-version-ack-receipt/native-description.json");
    let fixture:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),360);
    for row in rows {
        let input=&row["input"];let output=&row["output"];
        let mut f=Fields{world:input["world"].as_bool().unwrap(),kingdom:input["kingdom"].as_bool().unwrap(),
            name:units(&input["name"]),color:units(&input["color"]),id:input["id"].as_u64().unwrap()as u32,events:vec![],fail:None};
        assert_eq!(version_ack::describe(&mut f).unwrap(),units(&output["text"]),"{input}");
        assert_eq!(json!(f.events),output["events"],"{input}");
    }
}
#[test]
fn description_errors_stop_before_later_eager_reads_and_color_is_lazy(){
    for has in [false,true] {for fail in ["name","color"] {
        let mut f=Fields{world:has,kingdom:true,name:vec![65],color:vec![66],id:7,events:vec![],fail:Some(fail)};
        let r=version_ack::describe(&mut f);
        assert_eq!(r.is_err(),has||fail=="name");
        let events=if fail=="name" {json!([["kingdom",has],["name",[65]]])}
            else if has {json!([["kingdom",true],["name",[65]],["kingdom_again",true]])}
            else {json!([["kingdom",false],["name",[65]],["id",7]])};
        assert_eq!(json!(f.events),events);
    }}
}
