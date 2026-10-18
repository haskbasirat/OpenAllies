use super::*;
use serde_json::Value;
struct Replay<'a>{input:&'a Value,events:Vec<&'static str>,comparison:Option<ValueComparison>,list:aa_actors::city_candidates::State,allocated:usize}
fn u(v:&Value)->u32{v.as_u64().unwrap() as u32}
impl World for Replay<'_>{
    type Error=String;type Definition=();
    fn city_owner(&mut self)->Result<u32,String>{self.events.push("city_owner");Ok(1)}
    fn actor_flags(&mut self,_:u32)->Result<u32,String>{self.events.push("flags");Ok(u(&self.input["flags"]))}
    fn actor_owner_108(&mut self,_:u32)->Result<u32,String>{self.events.push("owner");Ok(u32::from(self.input["owner"].as_bool().unwrap()))}
    fn kingdom_nonmajor_190(&mut self,_:u32)->Result<u32,String>{self.events.push("nonmajor");Ok(u(&self.input["nonmajor"]))}
    fn element_organization(&mut self,_:u32)->Result<Option<u32>,String>{self.events.push("element");Ok((self.input["element"]=="parent").then_some(4))}
    fn actor_value_4d01e0(&mut self,_:u32,pc:Precision)->Result<ValueComparison,String>{
        self.events.push("value");
        let values=self.input["values"].as_array().unwrap();
        let field=|index:usize|(!values[index].is_null()).then(||aa_body::F32(u(&values[index])));
        let result=crate::load_city_candidate::compare_fields(&aa_body::experience::ValueFields{regiment:field(0),wall:field(1),combat:field(2),component88:field(3)},pc)?;
        self.comparison=Some(result);Ok(result)
    }
    fn body_present_60(&mut self,_:u32)->Result<bool,String>{self.events.push("body");Ok(self.input["body"].as_bool().unwrap())}
    fn definition(&mut self,_:u32)->Result<(),String>{self.events.push("definition");Ok(())}
    fn source_byte(&mut self,_:(),offset:u32)->Result<u8,String>{
        let(key,event)=match offset{0x290=>("captureable","byte290"),0x291=>("destructible","byte291"),_=>unreachable!()};
        self.events.push(event);Ok(u(&self.input[key])as u8)
    }
    fn actor_reference_14(&mut self,_:u32)->Result<u32,String>{self.events.push("reference");Ok(0x12345678)}
    fn accepted_contains_10(&mut self,r:u32)->Result<bool,String>{self.events.push("accepted");Ok(self.input["accepted"].as_array().unwrap().iter().any(|v|u(v)==r))}
    fn candidates_contains_18(&mut self,r:u32)->Result<bool,String>{self.events.push("candidates");self.list.contains(r)}
    fn candidates_prepend_18(&mut self,r:u32)->Result<(),String>{
        self.events.push("candidates");self.events.push("allocate");
        if self.input["stop_allocation"].as_bool().unwrap(){return Err("allocation_required".into())}
        let tail=self.list.tail();self.list.prepend(r)?;self.allocated+=1;
        self.events.push("store_head");if tail.is_none(){self.events.push("store_tail")};Ok(())
    }
    fn pending_c(&mut self)->Result<bool,String>{self.events.push("pending");Ok(self.input["pending"].as_bool().unwrap())}
    fn schedule_50ac60(&mut self)->Result<(),String>{self.events.push("schedule");Err("schedule_required".into())}
}

#[test]
fn whole_native_city_admission_and_same_list_prefixes_both_images_all_pc(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-city-admission/native-city-admission.json")).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),132);
    for row in rows{
        let input=&row["input"];let mut list=aa_actors::city_candidates::State::constructed();
        for reference in input["candidates"].as_array().unwrap().iter().rev(){list.prepend(u(reference)).unwrap();}
        let mut world=Replay{input,events:vec![],comparison:None,list,allocated:0};
        let target=match input["target"].as_str().unwrap(){"null"=>0,"self"=>1,"other"=>2,_=>unreachable!()};
        let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>unreachable!()};
        let result=admit(&mut world,target,pc);
        let mut references=vec![];let mut current=world.list.head();let mut last=None;
        while let Some(id)=current{let node=world.list.node(id).unwrap();references.push(node.reference);last=Some(id);current=node.next;}
        assert_eq!(world.list.tail(),last,"{row}");
        let terminal=result.err().unwrap_or_else(||"returned".into());
        assert_eq!(terminal,row["native"]["terminal"].as_str().unwrap(),"{row}");
        assert_eq!(serde_json::json!(world.events),row["native"]["events"],"{row}");
        assert_eq!(serde_json::json!(references),row["native"]["candidates"],"{row}");
        assert_eq!(world.allocated,row["native"]["allocated"].as_u64().unwrap() as usize,"{row}");
        assert_eq!(world.comparison.map(|v|format!("{v:?}")),row["native"]["value_comparison"].as_str().map(str::to_owned),"{row}");
    }
}

#[test]
fn candidate_node_identity_unlinks_on_same_owner_and_keeps_loaded_list_separate(){
    let mut list=aa_actors::city_candidates::State::constructed();
    let first=list.prepend(7).unwrap();let second=list.prepend(7).unwrap();
    assert_ne!(first,second);assert_eq!(list.tail(),Some(first));assert_eq!(list.head(),Some(second));
    assert_eq!(list.pop_front().unwrap(),Some(7));assert!(list.node(second).is_err());
    assert_eq!(list.head(),Some(first));assert_eq!(list.tail(),Some(first));
    assert_eq!(list.pop_front().unwrap(),Some(7));assert!(list.head().is_none());assert!(list.tail().is_none());
    assert_eq!(list.allocated_nodes(),0);assert_eq!(list.pop_front().unwrap(),None);
}
