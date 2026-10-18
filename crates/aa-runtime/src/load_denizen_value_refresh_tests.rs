use super::*;
use serde_json::{Value,json};

struct Fixture {slots:Vec<Value>,current:u32,maximum:u32}
fn word(v:&Value,name:&str)->u32 {v[name].as_u64().unwrap() as u32}
impl World for Fixture {
    fn count(&self)->usize {self.slots.len()}
    fn occupant(&self,i:usize)->Result<Occupant> {
        let o=&self.slots[i]["occupant"];
        if o.is_null()||o["stale"].as_bool().unwrap(){return Ok(Occupant::Absent)}
        Ok(Occupant::Present{value:o["value"].as_u64().map(|v|v as u32),health:o["health"].as_array().map(|a|[a[0].as_u64().unwrap()as u32,a[1].as_u64().unwrap()as u32])})
    }
    fn target_flags(&self,i:usize)->Result<Option<u32>> {Ok(self.slots[i]["target"].as_object().map(|_|word(&self.slots[i]["target"],"flags")))}
    fn target_health(&self,i:usize)->Result<u32>{Ok(word(&self.slots[i]["target"],"health"))}
    fn slot_value(&self,i:usize)->Result<u32>{Ok(word(&self.slots[i],"slot_value"))}
    fn target_value(&self,i:usize)->Result<u32>{Ok(word(&self.slots[i]["target"],"value"))}
    fn current(&self)->Result<u32>{Ok(self.current)}fn maximum(&self)->Result<u32>{Ok(self.maximum)}
    fn set_current(&mut self,v:u32){self.current=v}fn set_maximum(&mut self,v:u32){self.maximum=v}
}
#[test]
fn all_native_denizen_value_rows_match_with_real_store_rounding() {
    let p=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-actor-value-refresh/native-denizen-value.json");
    let fixture:Value=serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();
    let rows=fixture["cases"].as_array().unwrap();assert_eq!(rows.len(),141);
    for row in rows {
        let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>unreachable!()};
        let input=&row["input"];
        let mut world=Fixture{slots:input["slots"].as_array().unwrap().clone(),current:word(input,"old_current"),maximum:word(input,"old_maximum")};
        recompute(&mut world,pc).unwrap();assert_eq!(json!({"current":world.current,"maximum":world.maximum}),row["output"],"{row}");
    }
}

