use aa_mind::{regiment_update::{self,BaseWorld,World},MindId,Result,Error};
use serde_json::{Value,json};
struct Current<'a>{input:&'a Value,calls:Vec<Value>,flags:u32,changed:bool,head:bool,top:usize,
    freed:Vec<usize>,head_null:bool,tail_null:bool,fail:Option<usize>}
impl Current<'_>{
    fn yes(&self,n:&str)->bool{self.input[n].as_bool().unwrap()}
    fn number(&self,n:&str)->u32{self.input[n].as_u64().unwrap()as u32}
    fn event(&mut self,v:Value)->Result<()>{self.calls.push(v);if self.fail==Some(self.calls.len()-1){Err(Error::World("required Update child".into()))}else{Ok(())}}
}
impl BaseWorld for Current<'_>{
    type Actor=usize;type Kingdom=();
    fn actor(&mut self,_:MindId)->Result<usize>{Ok(usize::from(self.changed))}
    fn actor_has_state10(&mut self,_:usize)->Result<bool>{self.event(json!(["state10"]))?;Ok(self.yes("state10"))}
    fn current_owner108(&mut self,_:usize)->Result<Option<()>>{self.event(json!(["owner108"]))?;Ok(self.yes("owner").then_some(()))}
    fn kingdom_byte239(&mut self,_:())->Result<u8>{Ok(self.number("owner239")as u8)}
    fn attached_to_organization(&mut self,_:usize)->Result<bool>{Ok(self.yes("element")&&self.yes("attached"))}
    fn current_home_4d11a0(&mut self,_:usize)->Result<bool>{Ok(self.yes("home"))}
    fn mind_virtual1a8(&mut self,_:MindId)->Result<()>{self.event(json!(["orphan"]))?;if self.yes("orphan_destroyed"){self.flags|=1;}Ok(())}
    fn timed_list_4d37e0(&mut self,_:usize)->Result<()>{self.event(json!(["timed"]))?;self.flags=self.number("time_flags");Ok(())}
}
impl World for Current<'_>{
    type State=usize;
    fn actor_flags(&mut self,a:usize)->Result<u32>{Ok(if a==1{self.number("think_flags")}else{self.flags})}
    fn member_count_57a730(&mut self,_:usize)->Result<u32>{Ok(self.number("members"))}
    fn refresh_engage(&mut self,_:MindId)->Result<()>{self.event(json!(["engage"]))?;self.top=1;Ok(())}
    fn current_state(&mut self,_:MindId)->Result<usize>{Ok(self.top)}
    fn think(&mut self,state:usize)->Result<()>{self.event(json!(["think",state]))?;
        self.head=self.yes("head_after");if self.yes("think_changes_actor"){self.changed=true;}else{self.flags=self.number("think_flags");}Ok(())}
    fn updatable_head_present(&mut self,_:MindId)->Result<bool>{Ok(self.head)}
    fn schedule(&mut self,_:MindId,delay:u32,payload:u32)->Result<()>{self.event(json!(["schedule",delay,payload]))}
    fn clear_arrivals(&mut self,_:MindId)->Result<()>{self.freed=(0..self.number("arrivals")as usize).collect();self.head_null=true;self.tail_null=true;Ok(())}
}
fn replay(row:&Value,fail:Option<usize>)->(Result<()>,Current<'_>){
    let input=&row["input"];let empty=input["arrivals"].as_u64().unwrap()==0;
    let mut w=Current{input,calls:vec![],flags:input["flags"].as_u64().unwrap()as u32,changed:false,
        head:input["head_before"].as_bool().unwrap(),top:0,freed:vec![],head_null:empty,tail_null:empty,fail};
    let result=regiment_update::update(&mut w,MindId(1));(result,w)
}
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/development10/mind-update/native-regiment-update.json")).unwrap()}
#[test]
fn actual_base_and_regiment_update_both_images_all_precisions(){
    let doc=fixture();assert_eq!(doc["rows"].as_array().unwrap().len(),720);
    for row in doc["rows"].as_array().unwrap(){let(result,w)=replay(row,None);result.unwrap();
        assert_eq!(json!(w.calls),row["calls"],"{row}");assert_eq!(w.flags,row["flags"].as_u64().unwrap()as u32,"{row}");
        assert_eq!(w.changed,row["actor_changed"].as_bool().unwrap(),"{row}");assert_eq!(json!(w.freed),row["freed"],"{row}");
        assert_eq!(w.head_null,row["arrival_head_null"].as_bool().unwrap(),"{row}");assert_eq!(w.tail_null,row["arrival_tail_null"].as_bool().unwrap(),"{row}");}
}
#[test]
fn required_update_children_preserve_the_committed_native_prefix(){
    for row in fixture()["rows"].as_array().unwrap(){let expected=row["calls"].as_array().unwrap();
        for fail in 0..expected.len(){let(result,w)=replay(row,Some(fail));assert!(result.is_err());assert_eq!(w.calls,expected[..=fail]);}}
}
