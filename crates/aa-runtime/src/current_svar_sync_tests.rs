use super::*;
use serde_json::{Value,json};
use std::{cell::{Cell,RefCell},rc::Rc};
struct Explicit<'a> {
    input:&'a [Value],
    text:Vec<Vec<u16>>,
    mutation:Rc<Cell<bool>>,
    reads:RefCell<Vec<(usize,usize)>>,
    fail_value:Option<usize>,
}
impl<'a> Explicit<'a> {
    fn new(input:&'a [Value],mutation:Rc<Cell<bool>>)->Self {
        Self{input,text:input.iter().map(|v|if v["kind"]==4{v["value"].as_array().unwrap().iter().map(|x|x.as_u64().unwrap()as u16).collect()}else{Vec::new()}).collect(),mutation,reads:RefCell::new(Vec::new()),fail_value:None}
    }
}
impl Read for Explicit<'_> {
    fn count(&self)->usize{self.input.len()}
    fn name(&self,i:usize)->Result<(&str,u32)>{Ok((self.input[i]["name"].as_str().unwrap(),self.input[i]["word20"].as_u64().unwrap()as u32))}
    fn kind(&self,i:usize)->Result<u32>{Ok(self.input[i]["kind"].as_u64().unwrap()as u32)}
    fn word(&self,i:usize,c:usize)->Result<u32>{
        self.reads.borrow_mut().push((i,c));
        if self.fail_value==Some(i){return Err("explicit required current value".into())}
        if self.input[i]["kind"]==5 {
            let first=self.input.iter().position(|v|v["kind"]==5).unwrap();
            if i==first&&c==1&&self.mutation.get(){return Ok(0x7fc01234)}
            Ok(self.input[i]["value"][c].as_u64().unwrap()as u32)
        }else{Ok(self.input[i]["value"].as_u64().unwrap()as u32)}
    }
    fn text(&self,i:usize)->Result<&[u16]>{Ok(&self.text[i])}
}
struct Log {
    events:Vec<Value>,
    cutoff:Option<usize>,
    intervene:bool,
    mutation:Rc<Cell<bool>>,
}
impl Log {
    fn add(&mut self,kind:&str,label:&str,value:Value)->Result<()> {
        if self.cutoff==Some(self.events.len()){return Err("explicit native logger cutoff".into())}
        self.events.push(json!({"kind":kind,"label":label,"value":value}));
        if self.intervene&&label=="var->GetValuePoint3().x"{self.mutation.set(true)}
        Ok(())
    }
}
impl Sink for Log {
    fn marker(&mut self,l:&str)->Result<()>{self.add("marker",l,json!(0))}
    fn int(&mut self,l:&str,v:u32)->Result<()>{self.add("int",l,json!(v))}
    fn float(&mut self,l:&str,v:u32)->Result<()>{self.add("float",l,json!(v))}
    fn boolean(&mut self,l:&str,v:bool)->Result<()>{self.add("bool",l,json!(v))}
    fn text(&mut self,l:&str,v:&[u16])->Result<()>{self.add("text",l,json!(v))}
}
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/development10/current-svar-sync/native-current-svar.json")).unwrap()}
#[test]
fn original_v14_full_svar_traversal_raw_values_and_native_error_prefixes() {
    let f=fixture();assert_eq!(f["native_runs"],84);
    assert_eq!(f["rows"].as_array().unwrap().len(),42);
    for (n,row)in f["rows"].as_array().unwrap().iter().enumerate() {
        let mutation=Rc::new(Cell::new(false));
        let values=Explicit::new(row["input"].as_array().unwrap(),mutation.clone());
        let mut log=Log{events:Vec::new(),cutoff:row["stop"].as_u64().map(|v|v as usize),intervene:row["variant"]==1,mutation};
        let result=traverse(&values,&mut log);
        assert_eq!(result.is_err(),row["output"]["stopped"].as_bool().unwrap(),"row{n}");
        assert_eq!(json!(log.events),row["output"]["events"],"row{n}");
        if row["stop"]==0||row["stop"]==1 {assert!(values.reads.borrow().is_empty());}
    }
}
#[test]
fn actual_boolean_logger_prefix_normalizes_the_raw_native_byte() {
    let f=fixture();assert_eq!(f["normalization_runs"],24);
    for row in f["normalization"].as_array().unwrap(){
        assert_eq!(row["kind"],1);
        let raw=row["value"].as_u64().unwrap()as u8;
        assert_eq!(row["normalized"],u32::from(raw!=0));
    }
}
#[test]
fn missing_current_value_stops_after_the_actual_name_and_ordinal_prefix() {
    let f=fixture();let input=f["rows"][0]["input"].as_array().unwrap();
    let mutation=Rc::new(Cell::new(false));let mut values=Explicit::new(input,mutation.clone());
    values.fail_value=Some(0);
    let mut log=Log{events:Vec::new(),cutoff:None,intervene:false,mutation};
    assert!(traverse(&values,&mut log).unwrap_err().contains("required current value"));
    assert_eq!(log.events.len(),2);
    assert_eq!(log.events[0]["label"],"SyncSettings");
    assert_eq!(log.events[1]["label"],input[0]["name"]);
    assert_eq!(log.events[1]["value"],input[0]["word20"]);
    assert_eq!(&*values.reads.borrow(),&[(0,0)]);
}
