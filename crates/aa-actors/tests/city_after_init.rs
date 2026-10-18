use aa_actors::{actor_creation::{Error,Result},component_virtuals::{CityAfterWorld,city_after_init}};
use serde_json::{Value,json};
struct Probe {input:Value,events:Vec<Value>,radius:u32,xy:[u32;2],allocated:bool}
impl Probe {
    fn event(&mut self,label:&str,value:Value)->Result<()> {
        self.events.push(json!([label,value]));
        if self.input["stop"]==label {return Err(Error::Unsupported("native probe cutoff"))}
        Ok(())
    }
}
impl CityAfterWorld for Probe {
    fn create_zone8(&mut self)->Result<bool> {
        self.event("create",json!(8))?;
        let result=self.input["create"].as_u64().unwrap() as u8!=0;
        self.allocated=result;
        if let Some(change)=self.input.get("mutate") {
            self.radius=change["radius"].as_u64().unwrap() as u32;
            self.xy=change["xy"].as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u32).collect::<Vec<_>>().try_into().unwrap();
        }
        Ok(result)
    }
    fn radius_5b4(&mut self)->Result<u32> {self.event("radius",json!(self.radius))?;Ok(self.radius)}
    fn position_xy(&mut self)->Result<[u32;2]> {self.event("position",json!(self.xy))?;Ok(self.xy)}
    fn move_zone8(&mut self,xy:[u32;2],radius:u32)->Result<()> {self.event("move",json!([xy[0],xy[1],radius]))}
}
#[test]
fn whole_native_city_after_order_raw_bits_mutations_and_error_prefixes() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-city-after-init/native-city-after.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),78);
    for row in rows {
        let input=&row["input"];
        let mut p=Probe {input:input.clone(),events:vec![],radius:input["radius"].as_u64().unwrap()as u32,xy:input["xy"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect::<Vec<_>>().try_into().unwrap(),allocated:false};
        let result=city_after_init(&mut p);
        assert_eq!(result.is_ok(),row["native"]["terminal"]=="complete","{row}");
        assert_eq!(json!(p.events),row["native"]["events"],"{row}");
        assert_eq!(p.allocated,row["native"]["zone_allocated"].as_bool().unwrap(),"{row}");
    }
}
