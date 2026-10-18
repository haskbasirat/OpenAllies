use aa_actors::organization_state::{Arg, Cursor, LoadError};
use aa_runtime::sun_state::{State, World};
use serde_json::{Value, json};

fn words(s: &State) -> Vec<u32> {
    let mut out = s.direction.to_vec();
    out.extend(s.target_colors.iter().flatten());
    out.extend(s.current_colors.iter().flatten());
    out.extend([s.color_duration, s.current_dimmer, s.target_dimmer, s.dimmer_duration, s.start_time]);
    out.extend([s.byte_70 as u32, s.word_74, s.word_78, s.byte_7c as u32, s.word_80, s.word_84]);
    out
}
fn seed(s: &mut State) {
    let data: Vec<_> = (8..0x70).step_by(4).map(|i|0x5a000000+i).collect();
    s.direction.copy_from_slice(&data[..3]);
    for (i,v) in s.target_colors.iter_mut().flatten().enumerate() {*v=data[3+i];}
    for (i,v) in s.current_colors.iter_mut().flatten().enumerate() {*v=data[12+i];}
    s.color_duration=data[21];s.current_dimmer=data[22];s.target_dimmer=data[23];
    s.dimmer_duration=data[24];s.start_time=data[25];
    s.byte_70=0x55;s.byte_7c=0x66;s.word_74=0x5a000074;s.word_78=0x5a000078;
    s.word_80=0x5a000080;s.word_84=0x5a000084;
}
fn unhex(s:&str)->Vec<u8> {(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
fn outcome(e:Option<LoadError>)->Value {
    match e {
        None=>Value::Null,
        Some(LoadError::Truncated{at,wanted})=>json!({"kind":"Truncated","at":at,"wanted":wanted}),
        Some(LoadError::Exception(m))|Some(LoadError::Fatal(m))=>{
            let kind=if m.format.starts_with("Bad data version") {"ThrowException"} else {"Fatal"};
            json!({"kind":kind,"format":m.format,"args":m.args.into_iter().map(|v|match v{Arg::Int(x)=>json!(x),Arg::Text(x)=>json!(x)}).collect::<Vec<_>>()})
        }
        Some(e)=>panic!("unexpected {e:?}"),
    }
}
struct Host {bound:i32,occupied:Vec<i32>,events:Vec<Value>,registered:Vec<i32>,cleared:bool}
impl World for Host {
    fn cancel_scheduled(&mut self,head:&mut Option<u64>)->Result<(),LoadError>{self.events.push(json!(["cancel"]));self.cleared=true;*head=None;Ok(())}
    fn table_word0(&mut self)->Result<i32,LoadError>{self.events.push(json!(["bound",self.bound]));Ok(self.bound)}
    fn slot_occupied(&mut self,id:i32)->Result<bool,LoadError>{self.events.push(json!(["occupied",id]));Ok(self.occupied.contains(&id))}
    fn register_updatable(&mut self,id:i32)->Result<(),LoadError>{self.events.push(json!(["register",id]));self.registered.push(id);Ok(())}
}
#[test]
fn original_and_v14_sun_ctor_load_all_pcs_and_every_truncated_prefix() {
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/development10/saved-sun/native-saved-sun.json");
    let fixture:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows=fixture["rows"].as_array().unwrap();
    assert_eq!(rows.len(),2376);
    for row in rows {
        let c=&row["input"];let n=&row["native"];
        let label=format!("{} {} {}",row["build"],row["precision"],c["name"]);
        let colors=std::array::from_fn(|i|c["colors"][i].as_u64().unwrap() as u32);
        let mut s=State::new(colors);
        assert_eq!(s.scheduled_head,None,"ctor base+4 {label}");
        assert_eq!(json!(words(&s)),n["ctor"],"ctor {label}");
        if c["seed"].as_bool().unwrap(){seed(&mut s);s.scheduled_head=Some(0xdeadbeef);}
        let mut world=Host{bound:c["table_word0"].as_i64().unwrap() as i32,
          occupied:c["occupied"].as_array().unwrap().iter().map(|v|v.as_i64().unwrap() as i32).collect(),
          events:vec![],registered:vec![],cleared:false};
        let bytes=unhex(c["payload"].as_str().unwrap());
        let mut cursor=Cursor::new(&bytes);
        let result=s.load_cursor(&mut cursor,&mut world);
        assert_eq!(outcome(result.err()),n["outcome"],"outcome {label}");
        assert_eq!(json!(cursor.position()),n["consumed"],"consumed {label}");
        assert_eq!(json!(words(&s)),n["state"],"state {label}");
        assert_eq!(json!(world.events),n["events"],"events {label}");
        assert_eq!(json!(world.registered),n["registered"],"table {label}");
        assert_eq!(json!(world.cleared),n["scheduled_cleared"],"cancel {label}");
        assert_eq!(json!(s.scheduled_head.is_none()),n["scheduled_cleared"],"base+4 {label}");
    }
}

#[test]
#[ignore="requires AA_GAME_ROOT original maps"]
fn actual_original_map_sun_chunks_have_complete_native_loads() {
    let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut reports=vec![];
    for name in ["Campaign/tutorial02","Campaign/allies01_el_alamein","(2v2)Berlin"] {
        let bytes=std::fs::read(root.join(format!("reconstruction/assets/original/Data/Maps/{name}.RMP"))).unwrap();
        let map=aa_mapworld::MapArchive::from_bytes(bytes).unwrap();
        let chunk=map.decode_chunk("SUN",u64::MAX).unwrap();
        let table=map.decode_chunk("UPDATABLES",u64::MAX).unwrap();
        let bound=i32::from_le_bytes(table.bytes[..4].try_into().unwrap());
        // Isolated codec diagnostic. This test does not claim live scheduling.
        let mut world=Host{bound,occupied:vec![],events:vec![],registered:vec![],cleared:false};
        let mut s=State::new([0;3]);
        let consumed=s.load(&chunk.bytes,&mut world).unwrap();
        assert_eq!(consumed,chunk.bytes.len());
        reports.push(json!({"map":name,"sun_payload":chunk.bytes.iter().map(|v|format!("{v:02x}")).collect::<String>(),"state":words(&s),"bound":bound,"events":world.events,"registered":world.registered,"scope":"isolated codec, explicit image-zero constructor inputs; no live-world scheduling certificate"}));
    }
    if let Some(path)=std::env::var_os("AA_SUN_REPORT") {
        let out=std::fs::OpenOptions::new().create_new(true).write(true).open(path).unwrap();
        serde_json::to_writer_pretty(out,&reports).unwrap();
    }
    println!("{}",serde_json::to_string_pretty(&reports).unwrap());
}
