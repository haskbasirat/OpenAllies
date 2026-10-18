use aa_actors::organization_state::{Arg, Cursor, LoadError};
use aa_runtime::saved_misc::{NameListCounter, Pair, Reference, State, World};
use serde_json::{Value,json};

fn unhex(s:&str)->Vec<u8>{(0..s.len()).step_by(2).map(|i|u8::from_str_radix(&s[i..i+2],16).unwrap()).collect()}
fn text(s:&str)->Vec<u16>{s.encode_utf16().collect()}
fn snapshot(s:&State)->Value {
    json!({"names":s.name_list_counters.iter().map(|v|[v.definition,v.value]).collect::<Vec<_>>(),
      "speed":s.pre_cutscene_speed,"weather":match s.weather {Reference::Uninitialized=>json!("Uninitialized"),Reference::Null=>json!(0),Reference::Definition(v)=>json!(v)},
      "texts":s.used_strings,"pair":s.pair_27c.as_ref().map(|p|[p.word_00,p.word_04])})
}
fn outcome(e:Option<LoadError>)->Value {
    match e {
        None=>Value::Null,
        Some(LoadError::Truncated{at,wanted})=>json!({"kind":"Truncated","at":at,"wanted":wanted}),
        Some(LoadError::Exception(m))=>json!({"kind":"ThrowException","format":m.format,"args":m.args.into_iter().map(|v|match v{Arg::Int(x)=>json!(x),Arg::Text(x)=>json!(x)}).collect::<Vec<_>>()}),
        Some(e)=>panic!("unexpected error {e:?}"),
    }
}
struct Host {events:Vec<Value>}
impl World for Host {
    fn name_list(&mut self,ids:&[u16])->Result<Option<u32>,LoadError>{let s=String::from_utf16(ids).unwrap();self.events.push(json!(["name_list",s]));Ok(match s.to_lowercase().as_str(){"alpha"|"alias"=>Some(101),_=>None})}
    fn warn_unknown_name_list(&mut self,ids:&[u16])->Result<(),LoadError>{self.events.push(json!(["warn",String::from_utf16(ids).unwrap()]));Ok(())}
    fn weather(&mut self,ids:&[u16])->Result<Option<u32>,LoadError>{let s=String::from_utf16(ids).unwrap();self.events.push(json!(["weather",s]));Ok((s.to_lowercase()=="rain").then_some(202))}
}

#[test]
fn original_and_v14_misc_body_all_pcs_and_every_truncated_prefix() {
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/development10/saved-misc/native-saved-misc.json");
    let fixture:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),774);
    for row in rows {
        let c=&row["input"];let n=&row["native"];
        let label=format!("{} {} {}",row["build"],row["precision"],c["name"]);
        let mut state=State::new(c["speed"].as_u64().unwrap() as u32);
        assert_eq!(snapshot(&state),n["ctor"],"ctor {label}");
        if c["seed"].as_bool().unwrap() {
            state.name_list_counters.push(NameListCounter{definition:101,value:99});
            state.weather=Reference::Definition(202);state.used_strings.push_back(text("existing"));
            state.pair_27c=Some(Pair{word_00:Some(7),word_04:Some(8)});
        }
        let mut world=Host{events:vec![]};let bytes=unhex(c["payload"].as_str().unwrap());
        let mut cursor=Cursor::new(&bytes);
        let result=state.load_cursor(&mut cursor,c["version"].as_u64().unwrap() as u32,&mut world);
        assert_eq!(outcome(result.err()),n["outcome"],"outcome {label}");
        assert_eq!(json!(cursor.position()),n["consumed"],"consumed {label}");
        assert_eq!(snapshot(&state),n["state"],"state {label}");
        assert_eq!(json!(world.events),n["events"],"events {label}");
    }
}

#[test]
fn original_three_map_misc_payloads_match_both_native_builds_all_pcs() {
    let path=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/development10/saved-misc/native-actual-misc.json");
    let fixture:Value=serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),18);
    for row in rows {
        let c=&row["input"];let n=&row["native"];
        let mut state=State::new(c["speed"].as_u64().unwrap() as u32);
        let mut world=Host{events:vec![]};let bytes=unhex(c["payload"].as_str().unwrap());
        let consumed=state.load(&bytes,c["version"].as_u64().unwrap() as u32,&mut world).unwrap();
        assert_eq!(consumed,bytes.len());
        assert_eq!(json!(consumed),n["consumed"]);
        assert_eq!(snapshot(&state),n["state"]);
        assert!(world.events.is_empty());
    }
}

#[test]
#[ignore="requires AA_GAME_ROOT actual source bytes"]
fn actual_fresh_source_constructor_binds_speed_and_refuses_changed_settings_or_rules() {
    use aa_rules::{actors::ActorCatalog,sources::{SourceLimits,V14Sources}};
    use aa_runtime::{catalog::Catalog,combat_source_completion,combat_source_receipt::VerifiedStartupReceipt,settings::Settings};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();
    let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(Settings::load_v14(&mut sources,aa_rules::tgi::Limits::default()).unwrap());
    let reference=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut sources).unwrap();
    combat_source_completion::install(&catalog,&mut sources,reference.context(),reference.csv(),reference.globals()).unwrap();
    combat_source_completion::compute(&catalog).unwrap();
    let mut reports=vec![];
    for name in ["Campaign/tutorial02","Campaign/allies01_el_alamein","(2v2)Berlin"] {
        let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join(format!("reconstruction/assets/original/Data/Maps/{name}.RMP"))).unwrap()).unwrap();
        let inventory=aa_mapworld::WorldInventory::decode(&map,u64::MAX).unwrap();
        let mut state=aa_runtime::saved_misc::from_source_constructor(&catalog,&inventory).unwrap();
        assert_eq!(state.pre_cutscene_speed,0);
        assert_eq!(state.weather,Reference::Uninitialized);
        let before=snapshot(&state);
        let chunk=map.decode_chunk("MISC_DATA",u64::MAX).unwrap();
        let mut host=Host{events:vec![]};
        let consumed=state.load(&chunk.bytes,inventory.world.version,&mut host).unwrap();
        assert_eq!(consumed,chunk.bytes.len());assert!(host.events.is_empty());
        reports.push(json!({"map":name,"map_source":map.source(),"chunk_source":chunk.payload_source,"version":inventory.world.version,
          "source_pc":format!("{:?}",reference.context().precision()),"constructor":before,"loaded":snapshot(&state),
          "scope":"controlled fresh source settings; no intervening SVar mutation; isolated codec, no world-ready certificate"}));
        // A public Settings row cannot forge its claimed original source span.
        let old=catalog.settings.as_ref().unwrap().declarations["gamespeed"][0].clone();
        catalog.settings.as_mut().unwrap().declarations.get_mut("gamespeed").unwrap()[0].raw="1".into();
        assert!(aa_runtime::saved_misc::from_source_constructor(&catalog,&inventory).unwrap_err().contains("value/span"));
        catalog.settings.as_mut().unwrap().declarations.get_mut("gamespeed").unwrap()[0]=old;
        let mut altered=inventory.clone();
        altered.header.as_mut().unwrap().rules.rules.push(aa_mapworld::header::Rule{key:text("GameSpeed"),enabled:0,kind:2,value:0x3f800000});
        assert!(aa_runtime::saved_misc::from_source_constructor(&catalog,&altered).unwrap_err().contains("saved WorldRule"));
    }
    if let Some(path)=std::env::var_os("AA_MISC_REPORT") {
        let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        serde_json::to_writer_pretty(file,&reports).unwrap();
    }
    println!("{}",serde_json::to_string_pretty(&reports).unwrap());
}
