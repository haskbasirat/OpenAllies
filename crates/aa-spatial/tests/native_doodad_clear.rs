use aa_spatial::{doodad_clear::{self,World},Result,Error,geometry::Placement,collision::Shape};
use serde_json::{Value,json};
struct Current<'a>{row:&'a Value,flags:Vec<u32>,calls:Vec<Value>,fail_at:Option<usize>}
impl Current<'_>{
    fn event(&mut self,e:Value)->Result<()>{self.calls.push(e);if self.fail_at==Some(self.calls.len()-1){Err(Error::World("required current child".into()))}else{Ok(())}}
    fn actor(&self,i:usize)->&Value{&self.row["input"]["actors"][i]}
}
impl World for Current<'_>{
    type Object=usize;
    fn collect_overlaps(&mut self,_:&Placement)->Result<Vec<u32>>{self.event(json!(["collect"]))?;
        Ok(self.row["input"]["order"].as_array().unwrap().iter().map(|j|self.actor(j.as_u64().unwrap()as usize)["gtid"].as_u64().unwrap()as u32).collect())}
    fn resolve(&mut self,reference:u32)->Result<usize>{Ok((reference&0xffff)as usize-1)}
    fn runtime_class(&mut self,i:usize)->Result<u32>{Ok(self.actor(i)["category"].as_u64().unwrap()as u32)}
    fn flags(&mut self,i:usize)->Result<u32>{Ok(self.flags[i])}
    fn referenced_by_trigger(&mut self,i:usize)->Result<bool>{
        for call in self.row["references"][i]["calls"].as_array().unwrap(){self.event(call.clone())?;}
        Ok(!self.row["references"][i]["ordinal"].is_null())
    }
    fn definition_ids(&mut self,i:usize)->Result<String>{Ok(self.actor(i)["ids"].as_u64().unwrap().to_string())}
    fn current_gtid(&mut self,i:usize)->Result<u32>{Ok(self.actor(i)["gtid"].as_u64().unwrap()as u32)}
    fn log_purge(&mut self,reference:u32,ids:String)->Result<()>{self.event(json!(["purge",reference,ids.parse::<u32>().unwrap()]))}
    fn destroy(&mut self,i:usize)->Result<()>{self.event(json!(["destroy",i]))?;
        if self.row["input"]["mutate_next"].as_bool().unwrap(){self.flags[(i+1)%4]=1<<10;}Ok(())}
}
fn run(row:&Value,fail_at:Option<usize>)->(Result<()>,Vec<Value>,Vec<u32>){
    let mut world=Current{row,flags:row["input"]["actors"].as_array().unwrap().iter().map(|a|a["flags"].as_u64().unwrap()as u32).collect(),calls:vec![],fail_at};
    let place=Placement{shape:Shape{kind:0,radius_bits:0,bounds:[0;4]},facing:[0,0xbf800000],position:[0,0]};
    let result=doodad_clear::clear(&mut world,&place);(result,world.calls,world.flags)
}
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/development10/doodad-clear/native-clear.json")).unwrap()}
#[test]
fn actual_purge_coordinator_both_images_all_precisions(){
    for row in fixture()["rows"].as_array().unwrap(){let(result,calls,flags)=run(row,None);result.unwrap();
        assert_eq!(json!(calls),row["calls"],"{row}");assert_eq!(json!(flags),row["final_flags"],"{row}");}
}
#[test]
fn required_children_keep_the_exact_committed_prefix(){
    for row in fixture()["rows"].as_array().unwrap(){let calls=row["calls"].as_array().unwrap();
        for fail_at in 0..calls.len(){let(result,got,_)=run(row,Some(fail_at));assert!(result.is_err());assert_eq!(got,calls[..=fail_at]);}}
}
