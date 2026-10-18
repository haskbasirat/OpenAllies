use super::*;
use serde_json::{Value,json};
#[test]
fn render_capacity_keeps_constructor_slots_and_overwrites_only_reached_element() {
    let mut slots=vec![SLOT_CONSTRUCTOR];let mut count=0;
    push_5a6c70(&mut slots,&mut count,[17;9]).unwrap();
    assert_eq!(count,1);assert_eq!(slots,vec![[17;9]]);
    push_5a6c70(&mut slots,&mut count,[23;9]).unwrap();
    assert_eq!(count,2);assert_eq!(slots.len(),4);
    assert_eq!(slots,vec![[17;9],[23;9],SLOT_CONSTRUCTOR,SLOT_CONSTRUCTOR]);
}

struct NativeInput<'a>{input:&'a Value,flags:[u32;2],events:Vec<Value>}
impl EntryWorld for NativeInput<'_>{
    type Texture=usize;
    fn flags_08(&mut self,t:&usize)->Result<u32>{Ok(self.flags[*t])}
    fn requests_04(&mut self,t:&usize)->Result<u32>{Ok(self.input["requests"][*t].as_i64().unwrap() as u32)}
    fn store_flags_08(&mut self,t:&usize,v:u32)->Result<()>{self.flags[*t]=v;Ok(())}
    fn warning_404ed0(&mut self,_:&usize,reading:bool)->Result<()>{
        self.events.push(json!(["warning",if reading{0x107}else{0x13f}]));
        if self.input["warning_error"]==true{Err("warning".into())}else{Ok(())}
    }
    fn lock_857e10(&mut self,t:&usize)->Result<()>{self.events.push(json!(["lock"]));self.flags[*t]^=self.input["lock_toggle"].as_u64().unwrap()as u32;Ok(())}
    fn loaded_reading_child(&mut self,_:&usize,enabled:bool)->Result<()>{self.events.push(json!(["loaded",enabled]));Err("loaded".into())}
    fn unlock_857de0(&mut self,_:&usize)->Result<()>{self.events.push(json!(["unlock"]));Ok(())}
}
impl World for NativeInput<'_>{
    fn last_location_writable(&mut self,t:&usize)->Result<bool>{
        if self.input["locations"][*t]==0{return Ok(false)}
        self.events.push(json!(["writable",*t]));Ok(self.input["writable"]==1)
    }
    fn writable_child_40(&mut self,t:&usize)->Result<Option<usize>>{Ok(if *t==0&&self.input["child"]==true{Some(1)}else{None})}
}
#[test]
fn original_v14_texture_settings_order_flags_capacity_and_error_prefix_all_pc(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-fxc-texture-binding/native-texture-settings.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(),570);
    for row in fixture["rows"].as_array().unwrap(){
        let c=&row["input"];let expected=&row["output"];
        let mut world=NativeInput{input:c,flags:[c["flags"][0].as_u64().unwrap()as u32,c["flags"][1].as_u64().unwrap()as u32],events:vec![]};
        let capacity=c["capacity"].as_u64().unwrap()as usize;
        let mut slots=(0..capacity).map(|i|if c["poison_slots"]==true{std::array::from_fn(|j|0x11000000+i as u32*0x100+j as u32)}else{SLOT_CONSTRUCTOR}).collect::<Vec<_>>();
        let mut count=c["count"].as_u64().unwrap()as usize;
        let format:[u32;9]=[1,0,255,65280,16711680,4278190080,32,0x20820881,0xc4100];
        assert_eq!(json!(format),expected["format"]);
        if c["push"]==true{
            let old_capacity=slots.len();
            push_5a6c70(&mut slots,&mut count,format).unwrap();
            if slots.len()!=old_capacity{world.events.push(json!(["alloc",slots.len()*36]));if old_capacity!=0{world.events.push(json!(["free"]));}}
        }
        let result=match c["operation"].as_str().unwrap(){
            "radar"=>set_reading_5a9330(&mut world,0,1).and_then(|()|set_rendering_5a6340(&mut world,0,0)),
            "reading"=>set_reading_5a9330(&mut world,0,c["raw"].as_u64().unwrap()as u32),
            "rendering"=>set_rendering_5a6340(&mut world,0,c["raw"].as_u64().unwrap()as u32),
            _=>unreachable!(),
        };
        assert_eq!(json!(result.err()),expected["stopped"],"{row}");
        assert_eq!(json!(world.flags),expected["flags"],"{row}");
        assert_eq!(json!(count),expected["count"],"{row}");
        assert_eq!(json!(slots.len()),expected["capacity"],"{row}");
        assert_eq!(json!(slots),expected["slots"],"{row}");
        assert_eq!(json!(world.events),expected["events"],"{row}");
    }
}
