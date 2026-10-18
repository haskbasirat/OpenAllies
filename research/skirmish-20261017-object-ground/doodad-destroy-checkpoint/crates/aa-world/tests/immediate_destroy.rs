//!The immediate kernel is replayed against native4F5490 rows, independently
//!of request_removal's queue and flag effects. Finalize is an explicit child.
use aa_world::immediate_destroy::{self, World};
use serde_json::Value;
struct Owner {
    flags:u32,
    trace:Vec<&'static str>,
    fail_finalize:bool,
}
impl World for Owner {
    type Object=u32;
    type Error=&'static str;
    fn actor_b4(&mut self,object:u32)->Result<bool,Self::Error> {assert_eq!(object,7);Ok(false)}
    fn resolve_handle(&mut self,_:u32)->Result<Option<u32>,Self::Error> {panic!("false native B4 must not resolve Actor158")}
    fn actor_2c(&mut self,object:u32,reason:u32)->Result<(),Self::Error> {assert_eq!((object,reason),(7,3));self.trace.push("0x943800");Ok(())}
    fn or_object_flags(&mut self,object:u32,bits:u32)->Result<(),Self::Error> {assert_eq!((object,bits),(7,2));self.flags|=bits;Ok(())}
    fn availability_and_classification(&mut self,_:u32)->Result<(),Self::Error> {assert_ne!(self.flags&2,0);self.trace.push("availability");Ok(())}
    fn finalize(&mut self,_:u32)->Result<(),Self::Error> {self.trace.push("0x4ea6f0");if self.fail_finalize {Err("required child")}else {Ok(())}}
}
#[test]
fn actual_object_doodad_dispatch_and_required_child_prefix() {
    let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-object-ground/native-doodad-destroy.json")).unwrap();
    let rows=doc["rows"].as_array().unwrap();assert_eq!(rows.len(),216);
    for row in rows {
        let mut owner=Owner {flags:row["input"]["flags"].as_u64().unwrap()as u32,trace:vec![],fail_finalize:!row["output"]["error"].is_null()};
        let result=immediate_destroy::run(&mut owner,7);
        assert_eq!(result.is_err(),owner.fail_finalize);
        assert_eq!(owner.flags,row["output"]["flags"].as_u64().unwrap()as u32);
        let expected=row["output"]["trace"].as_array().unwrap()[..3].iter().map(|v|v.as_str().unwrap()).collect::<Vec<_>>();
        assert_eq!(owner.trace,expected);
    }
}
