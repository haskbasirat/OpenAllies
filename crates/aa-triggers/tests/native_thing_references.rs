use aa_triggers::{decode::*,manager::{Manager,Result},thing_references::{self,Thing}};
use serde_json::{Value,json};
fn span()->Span{Span{offset:0,length:0}}
fn element(value:&Value)->Element{
    Element{source:span(),name:"fixture".into(),opcode:0,word_10:0,
        parameters:value.as_array().unwrap().iter().map(|p|{
            let argument=match p["argument"].as_u64().unwrap(){5=>"ma_actor",11=>"ma_camera",10=>"ma_area",6=>"ma_kingdom",4=>"ma_object",_=>unreachable!()};
            let item=p["item"].as_u64().unwrap()as u32;
            Parameter{source:span(),argument:argument.into(),item:"fixture".into(),item_index:item,
                storage:if item==15||item==45{6}else{1},flag:0,
                value:if item==15||item==45{DecodedValue::Text(p["text"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u16).collect())}
                    else{DecodedValue::Word(p["word"].as_u64().unwrap()as u32)}}
        }).collect()}
}
// serde_json::Value and the native decoded Value deliberately stay distinct.
use aa_triggers::decode::Value as DecodedValue;
fn manager(input:&Value)->Manager{
    let triggers=input["triggers"].as_array().unwrap().iter().enumerate().map(|(i,t)|Trigger{
        source:span(),id:i as u32,mode:0,enabled:0,repeat:0,activation_bits:0,name:vec![],description:vec![],word_2c:0,
        events:t[0].as_array().unwrap().iter().map(element).collect(),
        conditions:t[1].as_array().unwrap().iter().map(element).collect(),
        actions:t[2].as_array().unwrap().iter().map(element).collect(),
    }).collect();
    Manager::from_saved(SavedManager{next_id:1,triggers,variables:vec![],strings:vec![]})
}
struct Current<'a>{input:&'a Value,index:usize,calls:Vec<Value>,fail:bool}
impl Thing for Current<'_>{
    fn current_gtid(&mut self)->Result<u32>{Ok(self.input["actors"][self.index]["gtid"].as_u64().unwrap()as u32)}
    fn name(&mut self)->Result<Vec<u16>>{
        self.calls.push(json!(["name",self.index]));
        if self.fail{return Err(aa_triggers::manager::Error::Callback("required name".into()));}
        Ok(self.input["actors"][self.index]["name"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u16).collect())
    }
}
#[test]
fn actual_manager_trigger_element_chain_both_images_all_precisions(){
    let doc:Value=serde_json::from_str(include_str!("../../../research/development10/doodad-clear/native-clear.json")).unwrap();
    assert_eq!(doc["rows"].as_array().unwrap().len(),306);
    let mut named=0;let mut protected=0;
    for row in doc["rows"].as_array().unwrap(){let input=&row["input"];let manager=manager(input);
        for (index,expected) in row["references"].as_array().unwrap().iter().enumerate(){
            let mut thing=Current{input,index,calls:vec![],fail:false};
            let found=thing_references::first_referencing(&manager,&mut thing).unwrap();
            assert_eq!(json!(found),expected["ordinal"],"{input}, thing{index}");
            assert_eq!(json!(thing.calls),expected["calls"],"{input}, thing{index}");
            if found.is_some(){protected+=1;}
            if !thing.calls.is_empty(){named+=1;let mut current=Current{input,index,calls:vec![],fail:true};
                assert!(thing_references::first_referencing(&manager,&mut current).is_err());assert_eq!(current.calls.len(),1);}
        }
    }
    assert!(protected>0&&named>0);
}
