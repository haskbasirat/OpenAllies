use super::*;
use serde_json::Value;
struct Fixture<'a> {input:&'a Value,category:u32,reads:usize}
impl Fixture<'_> {
    fn bit(&self,k:&str)->bool {self.input[k].as_bool().unwrap()}
    fn word(&self,k:&str)->u32 {self.input[k].as_u64().unwrap() as u32}
}
impl State for Fixture<'_> {
    fn owner_word(&self)->Result<u32> {Ok(self.word("owner"))}
    fn category(&self)->Result<u32> {Ok(self.category)}
    fn class_three(&self)->Result<bool> {
        Ok(classification_three(ActorClassFields {flags8:self.word("flags8"),component90:false,
            components:[false,false,!self.bit("class_three"),false,false,false,false,false,false]}))
    }
    fn flare(&self)->Result<bool> {Ok(self.bit("flare"))}
    fn require_non_marker(&self)->Result<()> {Ok(())}
    fn set_category(&mut self,c:u32)->Result<()> {self.category=c;Ok(())}
    fn ordinary(&mut self,category:u32,owner:u32)->Result<()> {
        let Some(slots)=self.input["slots"].as_array() else {return Err(error("7A0390 ordinary category requires actual79F500 controller/goal ownership"))};
        ordinary_effects(category,owner,slots.iter().filter(|s|s.as_u64()==Some(u64::from(owner))).count(),slots.iter().filter(|s|!s.is_null()).count())
    }
}
impl actor_notify::ActorNotifyWorld for Fixture<'_> {
    fn sai_ready(&mut self)->Result<bool> {Ok(self.bit("ready"))}
    fn actor_c8(&mut self,_:u32)->Result<bool> {self.reads+=1;self.flare()}
    fn actor_category(&mut self,_:u32)->Result<u32> {self.reads+=1;self.category()}
    fn actor_removed(&mut self,_:u32)->Result<bool> {self.reads+=1;Ok(self.word("flags8")&1!=0)}
    fn predicate_79e300(&mut self,_:u32)->Result<bool> {self.reads+=1;self.class_three()}
    fn actor_4ceff0(&mut self,_:u32)->Result<bool> {panic!("out of scoped native path")}
    fn actor_virtual_20(&mut self,_:u32)->Result<bool> {panic!("out of scoped native path")}
    fn predicate_79e080(&mut self,_:u32)->Result<bool> {panic!("out of scoped native path")}
    fn unregister(&mut self,_:u32)->Result<()> {unregister(self)}
    fn register(&mut self,_:u32,c:u32)->Result<()> {register(self,c)}
    fn register_unit(&mut self,_:u32,_:bool)->Result<()> {panic!("out of scoped native path")}
}

#[test]
fn native_complete_non_marker_registration_and_notification_paths() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-sai-world-ready/native-actor-registration.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),162);
    for row in rows {
        let input=&row["input"];
        let mut w=Fixture {input,category:input["previous"].as_u64().unwrap()as u32,reads:0};
        match input["operation"].as_str().unwrap() {
            "register"=>register(&mut w,input["requested"].as_u64().unwrap()as u32).unwrap(),
            "unregister"=>unregister(&mut w).unwrap(),
            "notify"=>actor_notify::notify(&mut w,42).unwrap(),
            _=>unreachable!(),
        }
        assert_eq!(w.category,row["output"]["category"].as_u64().unwrap()as u32,"{input}");
        if !w.bit("ready") {assert_eq!(w.reads,0,"inactive native path must not inspect actor");}
    }
}

#[test]
fn unsupported_ordinary_branch_retains_native_category_write() {
    let input=serde_json::json!({"owner":0,"class_three":false,"flags8":1<<23,"flare":false});
    let mut w=Fixture {input:&input,category:5,reads:0};
    assert!(register(&mut w,1).unwrap_err().to_string().contains("79F500"));
    assert_eq!(w.category,1);
}
#[test]
fn concrete_ordinary_predicates_and_actual_empty_listener_paths_match_native() {
    use aa_navigation::sai_state::{ActorActivity,actor_active_for_sai};
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-sai-world-ready/native-actor-ordinary.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),708);
    for row in rows {
        let c=&row["input"];let bit=|name:&str|c[name].as_bool().unwrap();
        let got=match c["op"].as_str().unwrap() {
            "mind"=>u32::from(c["word"].as_u64().is_some_and(|v|v!=0)),
            "activity"=>u32::from(actor_active_for_sai(ActorActivity {flags8:c["flags"].as_u64().unwrap()as u32,
                contained_object_result:c["parent"].as_bool(),component6c:bit("mover"),componentac:bit("mobile"),componenta0:bit("recruit"),component88:bit("denizen")})),
            "presence"=>u32::from(bit("structure")||bit("element")||c["mind"]=="current"),
            "register"=>{
                let input=serde_json::json!({"owner":c["owner"],"class_three":false,"flags8":1<<23,"flare":false,"slots":c["slots"]});
                let mut world=Fixture {input:&input,category:5,reads:0};register(&mut world,c["category"].as_u64().unwrap()as u32).unwrap();world.category
            },
            _=>unreachable!(),
        };
        assert_eq!(got,row["output"].as_u64().unwrap()as u32,"{c}");
    }
    assert!(ordinary_effects(2,782,1,1).unwrap_err().to_string().contains("7AFE90"));
    assert!(ordinary_effects(1,782,0,1).unwrap_err().to_string().contains("7A83A0"));
    assert!(ordinary_effects(0,782,0,0).unwrap_err().to_string().contains("7A98E0"));
}
