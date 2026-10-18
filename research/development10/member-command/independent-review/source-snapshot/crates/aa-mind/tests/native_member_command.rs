use aa_mind::{cai_member_command::{self,World},cai_thinks::stack_action::ActionCommand,MindId,Result,Error};
use aa_numeric::Precision;
use serde_json::{Value,json};
struct Current<'a>{input:&'a Value,calls:Vec<Value>,tile:usize,fail:Option<usize>}
impl Current<'_>{
    fn pair(&self,name:&str)->[u32;2]{std::array::from_fn(|i|self.input[name][i].as_u64().unwrap()as u32)}
    fn event(&mut self,e:Value)->Result<()>{self.calls.push(e);if self.fail==Some(self.calls.len()-1){Err(Error::World("required live child".into()))}else{Ok(())}}
    fn yes(&self,name:&str)->bool{self.input[name].as_bool().unwrap()}
    fn command(&mut self,a:ActionCommand)->Result<()>{self.event(json!(["process",a.ordinal,a.flags,a.words18_28]))}
}
impl World for Current<'_>{
    type Organization=();type Tile=usize;type Zone=();
    fn organization(&mut self,_:MindId)->Result<()>{Ok(())}
    fn leader_position(&mut self,_:())->Result<[u32;2]>{Ok(self.pair("leader"))}
    fn relocate_current_organization(&mut self,_:MindId,p:[u32;2])->Result<()>{self.event(json!(["relocate",p]))}
    fn formation_origin(&mut self,_:())->Result<[u32;2]>{Ok(self.pair("origin"))}
    fn member_slot(&mut self,_:u32)->Result<i32>{Ok(self.input["slot"].as_i64().unwrap()as i32)}
    fn current_slot_offset(&mut self,_:MindId,s:i32)->Result<[u32;2]>{assert_eq!(s,self.input["slot"].as_i64().unwrap()as i32);Ok(self.pair("offset"))}
    fn tile(&mut self,p:[u32;2])->Result<Option<usize>>{self.event(json!(["tile",p]))?;let i=self.tile;self.tile+=1;
        Ok(self.yes(if i==0{"destination_tile"}else{"member_tile"}).then_some(i))}
    fn current_zone4(&mut self,_:MindId)->Result<Option<()>>{Ok(self.yes("zone").then_some(()))}
    fn contains_zone(&mut self,t:usize,_:())->Result<bool>{self.event(json!(["contains",t]))?;Ok(self.yes(if t==0{"destination_inside"}else{"member_inside"}))}
    fn member_position(&mut self,_:u32)->Result<[u32;2]>{Ok(self.pair("member_position"))}
    fn member_active_4cede0(&mut self,_:u32)->Result<bool>{self.event(json!(["active"]))?;Ok(self.yes("active"))}
    fn member_target_position(&mut self,_:u32)->Result<[u32;2]>{self.event(json!(["target"]))?;Ok(self.pair("target"))}
    fn current_sleep_point(&mut self,_:MindId)->Result<[u32;2]>{Ok(self.pair("sleep"))}
    fn issue_stop(&mut self,_:u32)->Result<()>{self.command(ActionCommand::new(6,1))}
    fn issue_move(&mut self,_:u32,p:[u32;2],sleep:[u32;2])->Result<()>{self.command(ActionCommand::move_to(1,p,0,sleep))}
}
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/development10/member-command/native-member-command.json")).unwrap()}
fn precision(row:&Value)->Precision{match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>unreachable!()}}
#[test]
fn actual_formation_move_stop_and_distance_both_images_all_precisions(){
    let doc=fixture();assert_eq!(doc["rows"].as_array().unwrap().len(),576);
    for row in doc["rows"].as_array().unwrap(){let mut w=Current{input:&row["input"],calls:vec![],tile:0,fail:None};
        cai_member_command::run(&mut w,MindId(1),7,row["input"]["flag"].as_u64().unwrap()as u8,precision(row)).unwrap();
        assert_eq!(json!(w.calls),row["calls"],"{row}");
        assert_eq!(cai_member_command::close(w.pair("member_position"),w.pair("target"),precision(row)).unwrap(),row["close"].as_bool().unwrap(),"{row}");}
}
#[test]
fn required_formation_children_keep_each_native_prefix(){
    for row in fixture()["rows"].as_array().unwrap(){let expected=row["calls"].as_array().unwrap();
        for fail in 0..expected.len(){let mut w=Current{input:&row["input"],calls:vec![],tile:0,fail:Some(fail)};
            assert!(cai_member_command::run(&mut w,MindId(1),7,row["input"]["flag"].as_u64().unwrap()as u8,precision(row)).is_err());
            assert_eq!(w.calls,expected[..=fail]);}}
}

#[test]
fn distance_comparison_keeps_x87_result_without_binary32_spill(){
    let doc:Value=serde_json::from_str(include_str!("../../../research/development10/member-command/independent-review/distance-no-fst-edges.json")).unwrap();
    let rows=doc["rows"].as_array().unwrap();assert_eq!(rows.len(),24);
    let mut discriminating=0;
    for row in rows {
        let pair=|name:&str|std::array::from_fn(|i|row[name][i].as_u64().unwrap()as u32);
        let actual=cai_member_command::close(pair("position"),pair("target"),precision(row)).unwrap();
        assert_eq!(actual,row["close"].as_bool().unwrap(),"{row}");
        if actual&&!row["spilled_close"].as_bool().unwrap(){
            assert_eq!(row["binary32_spilled_squared"].as_u64().unwrap(),0x3e80_0000);
            discriminating+=1;
        }
    }
    assert_eq!(discriminating,8);
}
