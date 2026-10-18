use super::*;
struct Input<'a>{c:&'a serde_json::Value,events:Vec<&'static str>}
impl Input<'_>{
    fn read(&mut self,name:&'static str)->Result<()>{self.events.push(name);if self.c["fail"]==name{Err(name.into())}else{Ok(())}}
}
impl Sources for Input<'_>{
    fn session_state_e4(&mut self)->Result<Option<u32>>{self.read("session")?;Ok(self.c["state"].as_u64().map(|v|v as u32))}
    fn multiplayer_f4(&mut self)->Result<bool>{self.read("multiplayer")?;Ok(self.c["multiplayer"].as_u64().unwrap()!=0)}
    fn local_player_first_name_unit(&mut self)->Result<Option<u16>>{self.read("local")?;Ok(self.c["name"].as_array().map(|s|s.first().map(|v|v.as_u64().unwrap()as u16).unwrap_or(0)))}
    fn dev_mode_62(&mut self)->Result<u8>{self.read("dev")?;Ok(self.c["dev"].as_u64().unwrap()as u8)}
}
#[test]
fn real_40eaa0_order_and_lazy_error_prefix_match(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-application-gate.json")).unwrap();
    for row in fixture["cases"].as_array().unwrap(){
        let mut input=Input{c:&row["input"],events:vec![]};let result=can_process_40eaa0(&mut input);
        assert_eq!(serde_json::json!(input.events),row["output"]["events"],"{row}");
        match row["output"]["stop"].as_str(){Some(stop)=>assert_eq!(result.unwrap_err(),stop),None=>assert_eq!(result.unwrap(),row["output"]["value"].as_bool().unwrap(),"{row}")}
    }
    assert_eq!(fixture["cases"].as_array().unwrap().len(),228);
}
