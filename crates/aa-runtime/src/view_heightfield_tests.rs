use super::*;
use serde_json::{json,Value};
fn pc(v:&Value)->Precision{match v.as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()}}
fn word(v:&Value)->u32{v.as_u64().unwrap().try_into().unwrap()}
fn fixture()->Value{serde_json::from_str(include_str!("../../../research/development10/native-view-heightfield.json")).unwrap()}
struct Input{value:Value,events:Vec<[u32;2]>}
impl Source for Input{
    fn quad_size(&mut self)->Result<i32>{Ok(self.value["quad"].as_i64().unwrap()as i32)}
    fn base_dimensions(&self)->[i32;2]{std::array::from_fn(|i|self.value["base"][i].as_i64().unwrap()as i32)}
    fn base_cell_size(&self)->u32{word(&self.value["cell"])}
    fn base_limits(&self)->[u32;2]{std::array::from_fn(|i|word(&self.value["limits"][i]))}
    fn sample(&mut self,xy:[u32;2],flags:u32,_:Precision)->Result<Binary>{
        assert_eq!(flags,0x122);let i=self.events.len();
        if self.value["stop"].as_u64()==Some(i as u64){return Err("sample_error".into())}
        self.events.push(xy);b(((i as i32%7-3)as f32*0.125).to_bits())
    }
}
fn snapshot(f:&Fields)->Value{json!({"step":f.step,"inverse":f.inverse,"columns":f.columns,"rows":f.rows,"limits":f.limits,"heights":f.heights})}
#[test]
fn native_visual_constructor_and_source_sampling_preserve_partial_owner(){
    let f=fixture();let constructors=f["constructors"].as_array().unwrap();assert_eq!(constructors.len(),6);
    for row in constructors{
        let state=Fields::constructed();let got=snapshot(&state);let expected=&row["fields"];
        for key in["step","inverse","limits","heights"]{assert_eq!(got[key],expected[key]);}
        assert_eq!(state.columns,None);assert_eq!(state.rows,None);
        assert_eq!(state.auxiliary_count,word(&row["aux_count"])as usize);
        assert_eq!(state.auxiliary.len(),word(&row["aux_capacity"])as usize);
        for (actual,expected)in state.auxiliary.iter().zip(row["aux"].as_array().unwrap()){
            assert_eq!(actual[0],Some(word(&expected[0])));assert_eq!(actual[1],Some(word(&expected[1])));
            assert_eq!(actual[2],None);assert_eq!(actual[3],None);
        }
    }
    let rows=f["initialize"].as_array().unwrap();assert_eq!(rows.len(),162);
    for row in rows{
        let mut fields=Fields::constructed();let mut input=Input{value:row["input"].clone(),events:vec![]};
        let outcome=initialize(&mut fields,&mut input,pc(&row["precision"])).err().unwrap_or_else(||"complete".into());
        assert_eq!(json!({"fields":snapshot(&fields),"events":input.events,"outcome":outcome}),row["output"],"{row}");
    }
}
#[test]
fn native_visual_bilinear_is_distinct_and_matches_exact_extended_return(){
    let f=fixture();let rows=f["queries"].as_array().unwrap();assert_eq!(rows.len(),180);
    for row in rows{
        let i=&row["input"];let mut fields=Fields::constructed();fields.step=word(&i["step"]);fields.inverse=word(&i["inverse"]);
        fields.columns=Some(i["columns"].as_i64().unwrap()as i32);fields.rows=Some(i["rows"].as_i64().unwrap()as i32);
        fields.heights=i["heights"].as_array().unwrap().iter().map(word).collect();
        let result=sample(&fields,std::array::from_fn(|j|word(&i["xy"][j])),pc(&row["precision"])).unwrap();
        assert_eq!(result.to_x87_extended_bits().unwrap().to_string(),row["output"]["extended"].as_str().unwrap(),"{row}");
        assert_eq!(n(result.store_f32()).unwrap(),word(&row["output"]["height"]));
    }
}
///research/phase8-vworld-update rects_4318e0: original 431E10 appends, then
///4318E0/431390 with the CRT floor/ceil (SSE2 and x87 paths) and 96CC4C, over
///a recorded 4F0E50 ground seam.
#[test]
fn native_rect_append_and_resample_4318e0(){
    struct Lab{heights:Vec<u32>,samples:Vec<[u32;2]>}
    impl Ground for Lab{
        fn sample(&mut self,xy:[u32;2],flags:u32,_:Precision)->Result<Binary>{
            assert_eq!(flags,0x122);let i=self.samples.len();self.samples.push(xy);
            b(self.heights[i%self.heights.len()])
        }
    }
    let fixture=crate::vworld_update::tests_fixture::load();
    let rows=fixture["rects_4318e0"].as_array().unwrap();assert_eq!(rows.len(),270);
    for row in rows{
        let i=&row["input"];let o=&row["output"];
        let mut fields=Fields::constructed();
        let columns=i["columns"].as_i64().unwrap() as i32;let rows=i["rows"].as_i64().unwrap() as i32;
        fields.step=word(&i["step"]);fields.inverse=word(&i["inverse"]);
        fields.columns=Some(columns);fields.rows=Some(rows);
        fields.heights=(0..columns*rows).map(|k|0x7f7f0000+k as u32).collect();
        fields.limits=[word(&i["limits"][0]),word(&i["limits"][1])];
        for r in i["append"].as_array().unwrap(){
            append_rect_431e10(&mut fields,[word(&r[0]),word(&r[1])],word(&r[2]),word(&r[3])).unwrap();
        }
        let appended:Vec<Value>=fields.auxiliary[..fields.auxiliary_count].iter().map(|r|json!(r.map(|v|v.unwrap()))).collect();
        assert_eq!(json!(appended),o["appended"],"{row}");
        let mut lab=Lab{heights:i["height"].as_array().unwrap().iter().map(word).collect(),samples:vec![]};
        let resampled=drain_rects_4318e0(&mut fields,&mut lab,pc(&i["precision"])).unwrap();
        assert_eq!(resampled,i["append"].as_array().unwrap().len());
        assert_eq!(json!(lab.samples),o["samples"],"{row}");
        assert_eq!(json!(fields.heights),o["heights"],"{row}");
        assert_eq!(json!(fields.limits),o["limits"],"{row}");
        assert_eq!(fields.auxiliary_count as u64,o["count"].as_u64().unwrap());
    }
}
