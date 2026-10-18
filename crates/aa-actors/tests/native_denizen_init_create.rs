use aa_actors::{actor_creation::{Error,Result},component_virtuals::{self,DenizenCreateWorld,LeafParams}};
use aa_numeric::Binary;
use serde_json::{Value,json};
use std::cell::RefCell;

struct World<'a>{input:&'a Value,events:RefCell<Vec<Value>>,health:Vec<u32>,time:u32}
impl World<'_>{
    fn event(&self,value:Value)->Result<()>{
        let mut events=self.events.borrow_mut();events.push(value);
        if self.input["fail"].as_u64()==Some(events.len()as u64){return Err(Error::World(format!("required-{}",events.last().unwrap()[0].as_str().unwrap())))}
        Ok(())
    }
}
impl DenizenCreateWorld for World<'_>{
    type Actor=();
    fn denizen_construction(&self,_:())->Result<bool>{let value=self.input["state"].as_u64().unwrap()as u32&0x200000!=0;self.event(json!(["construction",value]))?;Ok(value)}
    fn denizen_count(&self,_:())->Result<usize>{let n=self.health.len();self.event(json!(["count",n]))?;Ok(n)}
    fn denizen_health(&mut self,_:(),index:usize)->Result<Binary>{let bits=self.input["health"][index].as_u64().unwrap()as u32;self.event(json!(["health",index,bits]))?;Ok(Binary::from_f32_bits(bits)?)}
    fn denizen_store_health(&mut self,_:(),index:usize,bits:u32)->Result<()>{self.events.borrow_mut().push(json!(["store-health",index,bits]));self.health[index]=bits;Ok(())}
    fn denizen_guard_range(&self,_:())->Result<u32>{let bits=self.input["guard"].as_u64().unwrap()as u32;self.event(json!(["guard",bits]))?;Ok(bits)}
    fn denizen_inventory_head_present(&self,_:())->Result<bool>{let value=self.input["head"].as_bool().unwrap();self.event(json!(["head",value]))?;Ok(value)}
    fn denizen_schedule(&mut self,_:())->Result<()>{self.event(json!(["schedule"]))}
    fn denizen_delay(&self,_:())->Result<u32>{let bits=self.input["delay"].as_u64().unwrap()as u32;self.event(json!(["delay",bits]))?;Ok(bits)}
    fn denizen_store_time(&mut self,_:(),bits:u32)->Result<()>{self.events.borrow_mut().push(json!(["store-time",bits]));self.time=bits;Ok(())}
}
#[test]
fn native_denizen_create_health_queue_guards_and_committed_error_prefixes(){
    let doc:Value=serde_json::from_str(include_str!("../../../research/development10/denizen-init-create/native-denizen-init-create.json")).unwrap();
    let mut rows=0;
    for row in doc["rows"].as_array().unwrap(){
        let input=&row["input"];let mut world=World{input,events:RefCell::new(vec![]),health:input["old_health"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect(),time:input["old_time"].as_u64().unwrap()as u32};
        let params=LeafParams{word_34:0,mode:input["mode"].as_u64().unwrap()as u32,word_40:0,word_48:0};
        let params=(!input["null_params"].as_bool().unwrap()).then_some(&params);
        let result=component_virtuals::init_create_denizen((),params,&mut world);
        let output=&row["output"];
        if output["outcome"]=="ok"{assert_eq!(result.unwrap(),true)}else{assert!(result.unwrap_err().to_string().ends_with(output["outcome"].as_str().unwrap()),"row{rows}")}
        assert_eq!(json!(*world.events.borrow()),output["events"],"ordered services row{rows}");
        assert_eq!(json!(world.health),output["health"],"committed slot stores row{rows}");
        assert_eq!(json!(world.time),output["time"],"final store row{rows}");rows+=1;
    }
    assert_eq!(rows,195);
}
