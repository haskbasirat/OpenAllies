use super::*;
struct Source {events:Vec<Slot>,present:bool,stop:Option<Slot>}
impl Services for Source {
    fn map_present(&mut self)->Result<bool>{Ok(self.present)}
    fn current_input_pixel(&mut self)->Result<[i32;2]>{panic!("actual event must supply its point")}
    fn project_43c4b0(&mut self,point:[i32;2],slot:Slot,normal:&mut[u32;3])->Result<[u32;3]>{
        assert_eq!(point,[-4,31]);self.events.push(slot);*normal=[10,20,30];
        if self.stop==Some(slot){Err("actual terrain intersection child".into())}else{Ok([1,2,3])}
    }
}
#[test]fn later_intersection_error_keeps_prior_point_and_current_normal_prefix(){
    let mut state=State::from_vworld_constructor();let event=Event{pixel:[-4,31],flags:1,code:0x100};
    let mut source=Source{events:vec![],present:true,stop:Some(Slot::Down)};
    assert!(state.update_43c550(Some(&event),&mut source).is_err());
    assert_eq!(source.events,vec![Slot::Current,Slot::Down]);
    assert_eq!(state.position(Slot::Current),[1,2,3]);assert_eq!(state.normal(Slot::Down),[10,20,30]);
    assert_eq!(state.position(Slot::Down),[0;3]);
    source.present=false;source.events.clear();state.update_43c550(Some(&event),&mut source).unwrap();assert!(source.events.is_empty());
}
fn pc(v:&serde_json::Value)->Precision{match v.as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()}}
fn words(v:&serde_json::Value)->Vec<u32>{v.as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect()}
#[test]fn native_6fe590_ray_and_43c550_pointer_prefixes(){
    let fixture:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-ui-world-pointer/native-ui-pointer.json")).unwrap();
    struct Camera(Vec<u32>);impl CameraWords for Camera{fn word(&self,offset:u32)->Result<u32>{Ok(self.0[((offset-0x114)/4)as usize])}}
    for row in fixture["rays"].as_array().unwrap(){let c=&row["input"];assert_eq!(ray_6fe590(&Camera(words(&c["camera"])),words(&c["xy"]).try_into().unwrap(),pc(&row["precision"])).unwrap().to_vec(),words(&row["native"]),"{row}")}
    for row in fixture["projections"].as_array().unwrap(){
        struct CameraWithEye<'a>(&'a serde_json::Value);impl CameraWords for CameraWithEye<'_>{fn word(&self,offset:u32)->Result<u32>{let(a,i)=if offset<0x114{("eye",((offset-0x90)/4)as usize)}else{("camera",((offset-0x114)/4)as usize)};Ok(self.0[a][i].as_u64().unwrap()as u32)}}
        let c=&row["input"];let point=c["pixel"].as_array().unwrap().iter().map(|v|v.as_i64().unwrap()as i32).collect::<Vec<_>>().try_into().unwrap();let width=c["width"].as_i64().unwrap()as i32;let height=c["height"].as_i64().unwrap()as i32;
        let normalized=normalized_pixel(point,width,height,pc(&row["precision"])).unwrap();assert_eq!(normalized.to_vec(),words(&row["native"]["trace"][0]["normalized"]));
        let mut normal=[0xa5a5a5a5;3];let value=project_43c4b0(&CameraWithEye(c),point,width,height,&mut normal,pc(&row["precision"]),|eye,ray,normal|{
            assert_eq!(eye.to_vec(),words(&row["native"]["trace"][1]["eye"]));assert_eq!(ray.to_vec(),words(&row["native"]["trace"][1]["ray"]));*normal=[44,55,66];Ok([11,22,33])}).unwrap();
        assert_eq!(value.to_vec(),words(&row["native"]["point"]));assert_eq!(normal.to_vec(),words(&row["native"]["normal"]));
    }
    for row in fixture["pointers"].as_array().unwrap(){
        struct Call<'a>{input:&'a serde_json::Value,trace:Vec<serde_json::Value>}
        impl Services for Call<'_>{
            fn map_present(&mut self)->Result<bool>{Ok(self.input["map"].as_bool().unwrap())}
            fn current_input_pixel(&mut self)->Result<[i32;2]>{Ok(self.input["fallback"].as_array().unwrap().iter().map(|v|v.as_i64().unwrap()as i32).collect::<Vec<_>>().try_into().unwrap())}
            fn project_43c4b0(&mut self,point:[i32;2],slot:Slot,normal:&mut[u32;3])->Result<[u32;3]>{
                let i=State::index(slot)as u32;self.trace.push(serde_json::json!({"slot":i,"point":point}));*normal=[100+i,200+i,300+i];
                if self.input.get("stop").and_then(|v|v.as_u64())==Some(u64::from(i)){Err("required project".into())}else{Ok([400+i,500+i,600+i])}
            }
        }
        let c=&row["input"];let initial=c["initial"].as_u64().unwrap()as u32;let mut s=State{normals:[[initial;3];3],positions:[[initial;3];3]};
        let event=Event{pixel:c["pixel"].as_array().unwrap().iter().map(|v|v.as_i64().unwrap()as i16).collect::<Vec<_>>().try_into().unwrap(),flags:c["flags"].as_u64().unwrap()as u32,code:c["code"].as_u64().unwrap()as u16};
        let mut world=Call{input:c,trace:vec![]};let result=s.update_43c550(c["event"].as_bool().unwrap().then_some(&event),&mut world);
        let projected=(0..3).flat_map(|i|s.normals[i].into_iter().chain(s.positions[i])).collect::<Vec<_>>();
        assert_eq!(serde_json::json!({"trace":world.trace,"words":projected,"required":result.is_err()}),row["native"],"{row}");
    }
}
