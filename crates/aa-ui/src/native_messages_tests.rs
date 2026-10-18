use super::*;
use serde_json::{Value,json};
use crate::{fs::DataRoot,tgi::TgiStore,view_variables::SourceOverrides};
use aa_rules::{actors::native_load::{load_files,NativeLoadReport},sources::{Origin,ResolvedFile,sha256},tgi::Limits};
use std::sync::atomic::{AtomicU64,Ordering};
static NEXT:AtomicU64=AtomicU64::new(0);
fn catalog(extra:&str)->ActorCatalog{
    let bytes=format!("[MessageType]\n{{\n string IDS = GENERIC_EVENT\n string filter = UIMessageFilterGeneric\n string message_sound = null\n{extra}\n}}\n").into_bytes();
    load_files(vec![ResolvedFile{logical_path:"messages.tgi".into(),origin:Origin::V14Loose,sha256:sha256(&bytes),bytes}],Limits::default(),&mut NativeLoadReport::default())
}
fn vars()->UVars{
    let path=std::env::temp_dir().join(format!("aa-message-uvars-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::SeqCst)));
    std::fs::create_dir(&path).unwrap();std::fs::write(path.join("UVars.tgi"),"[Vars]\n{\n int MessageLogMaximumLines = 50\n flag UIMessageFilterGeneric = true\n}\n").unwrap();
    let mut store=TgiStore::new(DataRoot::new(&path));let vars=UVars::load_with_view_scope(&mut store,SourceOverrides::ControlledNone);
    std::fs::remove_dir_all(path).unwrap();vars.require_message_source().unwrap();vars
}
fn precision(s:&str)->Precision{match s{"Single"=>Precision::Single24,"Double"=>Precision::Double53,"Extended"=>Precision::Extended64,_=>panic!("unknown PC")}}
fn snapshot(s:&MessageSystem)->Value{json!({"count":s.count_30,"last":s.last,"generation":s.generation,
    "messages":s.messages.iter().map(|m|{let h=m.native_header().unwrap();json!({"id":m.id,"ordinal":h.ordinal(),"time":h.timestamp_bits(),"view":h.view_word14(),"text":h.text_utf16()})}).collect::<Vec<_>>()})}
struct Children<'a>{input:&'a Value,events:Vec<&'static str>}
impl Services for Children<'_>{
    fn milliseconds_59aba0(&mut self)->Result<u32>{self.events.push("timer");Ok(self.input["milliseconds"].as_u64().unwrap()as u32)}
    fn active_view_word_8c(&mut self)->Result<Option<u32>>{self.events.push("view");Ok(self.input["view"].as_u64().map(|v|v as u32))}
    fn message_diagnostic(&mut self,_:&Message)->Result<()>{self.events.push("diagnostic");if self.input["stop"]=="diagnostic"{Err("diagnostic".into())}else{Ok(())}}
    fn application_kind_b396c0(&mut self)->Result<u32>{self.events.push("app_kind");Ok(self.input["app"].as_u64().unwrap()as u32)}
}
fn replay(s:&mut MessageSystem,v:&mut UVars,pc:Precision,input:&Value,expected:&Value){
    v.set_bool("UIMessageFilterGeneric",input["filter"].as_u64().unwrap()!=0);
    v.set_i32("MessageLogMaximumLines",input["maximum"].as_i64().unwrap()as i32);
    let mut children=Children{input,events:vec![]};let text=input["text"].as_str().unwrap().encode_utf16().collect::<Vec<_>>();
    let result=s.send_native_generic(v,&text,&mut children,pc);
    if expected["stop"].is_null(){assert_eq!(result.unwrap(),expected["accepted"].as_bool().unwrap())}else{assert_eq!(result.unwrap_err(),"diagnostic")}
    let calls=expected["events"].as_array().unwrap().iter().filter_map(|s|s.as_str()).filter(|s|["timer","view","diagnostic","app_kind"].contains(s)).collect::<Vec<_>>();
    assert_eq!(children.events,calls);assert_eq!(snapshot(s),expected["system"]);
    assert_eq!(timestamp(input["milliseconds"].as_u64().unwrap()as u32,pc).unwrap()as u64,expected["header"]["time"].as_u64().unwrap());
    assert_eq!(s.native_count().unwrap(),s.count_30,"a caught diagnostic preserves usable exact prefix, not a rebuilt count");
}
#[test]
fn actual_native_message_enum_constructor_timestamp_list_and_failure_prefix_match(){
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-ui-messages.json")).unwrap();
    for row in fixture["enums"].as_array().unwrap(){let entries=row["entries"].as_array().unwrap();assert_eq!(entries.len(),39);assert_eq!(entries[15]["ordinal"],GENERIC_EVENT_ORDINAL);assert_eq!(entries[15]["name"],"GENERIC_EVENT");}
    for row in fixture["predefined_type_rows"].as_array().unwrap(){let definitions=row["definitions"].as_array().unwrap();
        assert_eq!(definitions.len(),39);assert_eq!(definitions[15]["ordinal"],GENERIC_EVENT_ORDINAL);assert_eq!(definitions[15]["ids"],"GENERIC_EVENT");assert_eq!(definitions[15]["vtable"],"0xa0b76c");}
    for row in fixture["source_type_field_rows"].as_array().unwrap(){let output=&row["output"];
        assert_eq!(output["filter"],"UIMessageFilterGeneric");assert_eq!(output["sound_is_null"],true);assert_eq!(output["loaded"],1);
        assert_eq!(output["color"],json!(vec![0x3f800000u32;4]));}
    for row in fixture["constructors"].as_array().unwrap(){assert_eq!(snapshot(&MessageSystem::default()),row["output"]);}
    let source=catalog("");let mut vars=vars();let rows=fixture["rows"].as_array().unwrap();assert_eq!(rows.len(),174);
    for row in rows{
        let pc=precision(row["precision"].as_str().unwrap());let mut system=MessageSystem::default();system.bind_native_generic(&source,Precision::Double53,&vars).unwrap();
        if let Some(sequence)=row["sequence"].as_array(){for case in sequence{replay(&mut system,&mut vars,pc,&case["input"],&case["output"]);}}
        else{replay(&mut system,&mut vars,pc,&row["input"],&row["output"]);}
    }
}
#[test]
fn strict_current_variables_identity_and_public_mutation_are_not_silent_defaults(){
    let source=catalog("");let mut vars=vars();let mut system=MessageSystem::default();system.bind_native_generic(&source,Precision::Double53,&vars).unwrap();
    assert!(MessageSystem::default().clone().bind_native_generic(&source,Precision::Double53,&vars).is_err());
    let input=json!({"milliseconds":1,"view":0,"app":2,"stop":null});let mut children=Children{input:&input,events:vec![]};
    assert!(system.send_native_generic(&vars.clone(),&[65],&mut children,Precision::Single24).is_err());assert!(children.events.is_empty());
    system.last=u32::MAX;system.native.as_mut().unwrap().integrity=integrity(&system);
    assert!(system.send_native_generic(&vars,&[0xd800,0,0xffff],&mut children,Precision::Extended64).unwrap());assert_eq!(system.last,0);
    assert_eq!(system.messages[0].native_header().unwrap().text_utf16(),&[0xd800,0,0xffff]);
    system.messages[0].text.push('!');assert!(system.native_count().is_err());
    let mut fresh=MessageSystem::default();assert!(fresh.bind_native_generic(&catalog(" string filter = other\n"),Precision::Double53,&vars).is_err());
    vars.set_str("UIMessageFilterGeneric","invalid");assert!(vars.message_filter_generic_current().is_err());
    vars.set_str("MessageLogMaximumLines","invalid");assert!(vars.message_maximum_lines_current().is_err());
}
#[test]
fn same_current_source_uvars_overlay_setters_and_defaults_remain_one_owner(){
    let ctor:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-frame-leaves/native-message-uvars.json")).unwrap();
    for row in ctor["rows"].as_array().unwrap(){assert_eq!(row["variables"],json!([
        {"index":67,"kind":3,"flags":0,"current":50},{"index":78,"kind":1,"flags":16,"current":1}]));}
    let registration:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-view-init/native-uvar-registration.json")).unwrap();
    for row in registration["cases"].as_array().unwrap(){let registrations=row["registrations"].as_array().unwrap();
        assert_eq!(registrations[67]["kind"],3);assert_eq!(registrations[67]["flags"],0);
        assert_eq!(registrations[78]["kind"],1);assert_eq!(registrations[78]["flags"],16);}
    let mut vars=vars();assert_eq!(vars.message_maximum_lines_current().unwrap(),50);assert_eq!(vars.message_filter_generic_current().unwrap(),1);
    vars.set_i32("MessageLogMaximumLines",-1);assert_eq!(vars.message_maximum_lines_current().unwrap(),-1);assert_eq!(vars.get_i32("MessageLogMaximumLines"),-1);
    vars.set_bool("UIMessageFilterGeneric",false);assert_eq!(vars.message_filter_generic_current().unwrap(),0);assert!(!vars.get_bool("UIMessageFilterGeneric"));
    let saved=vars.overrides();let mut other=vars.clone();other.apply_overrides(&saved);assert_eq!(other.message_filter_generic_current().unwrap(),0);
    let path=std::env::temp_dir().join(format!("aa-message-overlay-{}-{}",std::process::id(),NEXT.fetch_add(1,Ordering::SeqCst)));
    std::fs::create_dir(&path).unwrap();std::fs::write(path.join("UVars.tgi"),"[Vars]\n{\n int MessageLogMaximumLines = 3\n flag UIMessageFilterGeneric = false\n}\n").unwrap();
    std::fs::write(path.join("UVars-extra.tgi"),"[Vars]\n{\n int MessageLogMaximumLines = -2\n flag UIMessageFilterGeneric = true\n}\n").unwrap();
    let mut store=TgiStore::new(DataRoot::new(&path));let overlay=UVars::load(&mut store);std::fs::remove_dir_all(path).unwrap();
    let order=overlay.message_variable_sources().iter().map(|v|v.path.as_str()).collect::<Vec<_>>();assert_eq!(order.len(),2);
    let expected=if order.last().unwrap().eq_ignore_ascii_case("UVars.tgi"){(3,0)}else{(-2,1)};
    assert_eq!((overlay.message_maximum_lines_current().unwrap(),overlay.message_filter_generic_current().unwrap()),expected);
    let mut unknown_store=TgiStore::new(DataRoot::new("no-such-source"));let unbound=UVars::load_with_view_scope(&mut unknown_store,SourceOverrides::Unbound);
    assert!(unbound.message_filter_generic_current().is_err());
}
#[test]
#[ignore="actual original/v14 source assets only; no original process or renderer"]
fn actual_generic_event_source_and_same_ui_uvars_bind(){
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources=aa_rules::sources::V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),aa_rules::sources::SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let (actors,_)=aa_rules::actors::native_load::load(&mut sources).unwrap();
    let mut store=TgiStore::new(DataRoot::with_overlay(game.join("reconstruction/assets/original/Data"),game.join("reconstruction/native-game/runtime-expanded-v14/Data")));
    let vars=UVars::load(&mut store);assert_eq!(vars.message_variable_sources().len(),3);
    for p in[Precision::Single24,Precision::Double53,Precision::Extended64]{let mut system=MessageSystem::load(&mut store);
        system.bind_native_generic(&actors,p,&vars).unwrap();let (source,definition)=system.native_generic_source().unwrap();assert_eq!(source,actors.sources);assert!(definition.path.eq_ignore_ascii_case("Game/message_types.tgi"));}
}
