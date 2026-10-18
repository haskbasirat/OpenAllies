use super::*;
use serde_json::{Value,json};
#[derive(Default)]
struct Log {events:Vec<Value>,fail:Option<usize>}
impl Log {
 fn add(&mut self,kind:&str,label:&str,value:Value)->Result<()> {
  if self.fail==Some(self.events.len()) {return Err("explicit sink stop".into())}
  self.events.push(json!({"kind":kind,"label":label,"value":value}));Ok(())
 }
}
impl Sink for Log {
 fn marker(&mut self,l:&str)->Result<()>{self.add("marker",l,json!(0))}
 fn int(&mut self,l:&str,v:u32)->Result<()>{self.add("int",l,json!(v))}
 fn float(&mut self,l:&str,v:u32)->Result<()>{self.add("float",l,json!(v))}
 fn boolean(&mut self,l:&str,v:bool)->Result<()>{self.add("bool",l,json!(v))}
 fn text(&mut self,l:&str,v:&[u16])->Result<()>{self.add("text",l,json!(v))}
}
struct App;
fn map_text_chunk(text:&str)->aa_mapworld::Chunk {
 let bytes=text.as_bytes().to_vec();let identity=aa_mapworld::SourceIdentity::of(&bytes);
 aa_mapworld::Chunk{map_source:identity.clone(),payload_source:identity,encoding:aa_mapworld::archive::Encoding::Raw,
 directory:aa_mapworld::ChunkDirectory{name:"MAPSTRINGS".into(),directory_ordinal:0,packed_offset:0,packed_bytes:bytes.len()as u64,source_timestamp:0,decoded_bytes:bytes.len()as u64},bytes}
}
#[test]
fn map_strings_replacement_preserves_global_identity_and_retains_removal_on_read_failure() {
 use crate::localization::Localization;use aa_rules::{sources::{ResolvedFile,Origin,sha256},tgi::Limits};
 let bytes=b"[Text language=Default]\n{\n ids=global\n name=\"global\"\n global_key=\"Kept\"\n}\n".to_vec();
 //A global filename spelling cannot confer membership in map_strings.
 let mut text=Localization::from_files([ResolvedFile{logical_path:"MAPSTRINGS/not-a-map-origin.tgi".into(),origin:Origin::V14Loose,sha256:sha256(&bytes),bytes}],Limits::default()).unwrap();
 let units=|s:&str|s.encode_utf16().collect::<Vec<_>>();let global=text.reference_existing(&units("global_key")).unwrap().unwrap();
 text.replace_map_strings(||Ok(map_text_chunk("[Text language=Default]\n{\nids=map_strings\n name=\"old\"\nmap_key=\"Old\"\n}\n")),Limits::default()).unwrap();
 let old=text.reference_existing(&units("map_key")).unwrap().unwrap();text.validate_reference(&global).unwrap();
 text.replace_map_strings(||Ok(map_text_chunk("[Text language=Default]\n{\nids=map_strings\n name=\"new\"\nmap_key=\"New\"\nnew_key=\"Added\"\n}\n")),Limits::default()).unwrap();
 assert_eq!(text.lookup(&units("map_key")).unwrap().unwrap(),units("New"));assert_eq!(text.lookup(&units("global_key")).unwrap().unwrap(),units("Kept"));
 text.validate_reference(&global).unwrap();assert!(text.validate_reference(&old).is_err());
 assert!(text.replace_map_strings(||Err("explicit native source-read cutoff after478D20".into()),Limits::default()).is_err());
 text.validate_reference(&global).unwrap();assert!(text.lookup(&units("map_key")).unwrap().is_none());assert!(text.lookup(&units("new_key")).unwrap().is_none());
}
impl AppRead for App {
 fn word(&self,o:u32)->Result<u32>{match o {0=>Ok(7),0x10=>Ok(3),0x18=>Ok(11),0x1c=>Ok(4),0x24=>Ok(2),_=>Err(format!("unexpected App word{o:X}"))}}
 fn byte(&self,o:u32)->Result<bool>{match o {4|0x14|0x15|0x16|0x20=>Ok(o==0x15||o==4),_=>Err(format!("unexpected App byte{o:X}"))}}
 fn failure_text_8(&self)->Result<&[u16]>{Ok(&[65,0,66,0xd800])}
}
#[test]
fn app_dump_and_error_prefix_match_actual_784560_order() {
 let fixture:Value=serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/../../research/skirmish-20261017-launch-settings/native-app-dump.json")).unwrap()).unwrap();
 for row in fixture["rows"].as_array().unwrap() {
  let mut log=Log{fail:row["stop"].as_u64().map(|v|v as usize),..Log::default()};
  let result=app(&App,&mut log);assert_eq!(result.is_err(),row["output"]["stopped"].as_bool().unwrap());
  assert_eq!(json!(log.events),row["output"]["events"]);
 }
}
#[test]
fn sync_sink_borrows_same_owner_and_evaluates_gate_only_at_reached_loggers() {
 use aa_orders::net::sync::{Synchronizer,step};
 let mut actual=Synchronizer::new(64);let mut calls=0;
 let mut gate=||{calls+=1;Ok(true)};
 {let mut binding=SyncBinding::Direct(crate::live_clocked::SyncLog{owner:&mut actual,gate:&mut gate});let mut sink=SyncSink{binding:&mut binding};sink.int("disabled",4).unwrap();}
 assert_eq!(calls,0);actual.reset(0,-1);actual.enabled=true;
 let mut gate=||{calls+=1;Ok(true)};
 {let mut binding=SyncBinding::Direct(crate::live_clocked::SyncLog{owner:&mut actual,gate:&mut gate});let mut sink=SyncSink{binding:&mut binding};sink.text("stored-length",&[65,0,66]).unwrap();sink.boolean("normalized",true).unwrap();}
 assert_eq!(calls,2);assert_eq!(actual.hash,step(step(step(step(-1,65),0),66),1));
 actual.suspend=1;let mut gate=||Err("disabled gate must not run".into());
 {let mut binding=SyncBinding::Direct(crate::live_clocked::SyncLog{owner:&mut actual,gate:&mut gate});SyncSink{binding:&mut binding}.marker("suspended").unwrap();}
}
struct Launch<'a>{header:&'a game_settings_value::Owner,load:&'a Creator,others:&'a Others}
impl LaunchRead for Launch<'_> {
 fn session_state_0(&self)->Result<u32>{Ok(2)}fn map_type_4(&self)->Result<u32>{Ok(3)}
 fn filename_8(&self)->Result<&[u16]>{Ok(&[77,0,65,80])}fn launch_type_0c(&self)->Result<u32>{Ok(5)}
 fn header_14(&self)->Result<&game_settings_value::Owner>{Ok(self.header)}fn load_1c(&self)->Result<&Creator>{Ok(self.load)}
 fn others(&self)->Result<&Others>{Ok(self.others)}fn app_2c(&self)->Result<&dyn AppRead>{Ok(&App)}
}
fn raw_rules(c:&Catalog<'_>,v:&CreatorRules)->Value {
 json!({"kind":v.mode(),"condition":definition_index(c,"GameType",v.game_type()).unwrap(),"changes":0,
 "rules":v.records().map(|r|json!({"rule":definition_index(c,"WorldRule",&r.ids().encode_utf16().collect::<Vec<_>>()).unwrap(),"enabled":r.locked()!=0,"value":r.value()})).collect::<Vec<_>>()})
}
fn raw_parameters(c:&Catalog<'_>,p:&WorldParameters,l:&creator_labels::State,closed:&[u8])->Value {
 json!({"names":p.teams.iter().enumerate().map(|(i,t)|json!({"name":t.id,"pair":{"first":t.display_key,"second":l.team_pair(p,i).unwrap().text()}})).collect::<Vec<_>>(),
 "kingdoms":p.kingdoms.iter().enumerate().map(|(i,k)|json!({"name":k.id,"pair":{"first":k.display_key,"second":l.kingdom_pair(p,i).unwrap().text()},"flag_10":k.byte_10!=0,"text_14":k.string_14,
 "nation":if k.nation.is_empty(){None}else{Some(definition_index(c,"Nation",&k.nation).unwrap())},"faction":if k.faction.is_empty(){None}else{Some(definition_index(c,"Faction",&k.faction).unwrap())},
 "color":if k.color.is_empty(){None}else{Some(definition_index(c,"KingdomColor",&k.color).unwrap())},"ai_handicap":if k.handicap.is_empty(){None}else{Some(definition_index(c,"Handicap",&k.handicap).unwrap())},
 "flag_28":k.byte_28!=0,"flag_29":k.byte_29!=0,"text_2c":k.string_2c,"flag_30":closed[i]!=0,"flag_31":k.byte_31!=0,"flag_32":k.byte_32!=0,
 "words":[],"pairs":k.localized_entries.iter().enumerate().map(|(j,key)|json!({"first":key,"second":l.objective_pair(p,i,j).unwrap().text()})).collect::<Vec<_>>(),
 "start_properties":k.definitions.iter().map(|ids|if ids.is_empty(){None}else{Some(definition_index(c,"Property",ids).unwrap())}).collect::<Vec<_>>(),"text_58":k.string_58,
 "additional_actors":[],"bonus":k.resources.iter().map(|r|r.1).collect::<Vec<_>>()})).collect::<Vec<_>>()})
}
fn raw_creator(c:&Catalog<'_>,v:&Creator)->Value {
 json!({"conditions":raw_rules(c,v.rules()),"header":raw_parameters(c,v.parameters(),v.labels(),v.closed30()),"size_x":v.extents_bits()[0],"size_y":v.extents_bits()[1],
 "def_44":v.diagnostic_reference_index(c,"Biome").unwrap(),"def_48":if v.base_payload().weather.is_empty(){None}else{Some(v.diagnostic_reference_index(c,"Weather").unwrap())},"field_4c":v.rng_seed()})
}
#[test]
#[ignore="requires actual AA_GAME_ROOT source, text, maps and sealed startup source receipt"]
fn actual_three_map_launch_dump_replays_native_and_retains_nonnull_siblings() {
 use aa_rules::{sources::{SourceLimits,V14Sources},actors::ActorCatalog,tgi::Limits};
 let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();let settings=crate::settings::Settings::load_v14(&mut files,Limits::default()).unwrap();let text=crate::localization::Localization::load_v14(&mut files,Limits::default()).unwrap();
 let mut captured=vec![];
 for name in ["Campaign/tutorial02","DeveloperMaps/(3v3)Firefight","(2v2)Berlin"] {
  let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(root.join(format!("reconstruction/assets/original/Data/Maps/{name}.RMP"))).unwrap()).unwrap();
  let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings.clone());
  catalog.definition_text=Some(text.clone());catalog.replace_current_text(text.clone());
  let receipt=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();crate::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();crate::combat_source_completion::compute(&catalog).unwrap();
  let variables=crate::creator_variables::State::from_fresh_sources(&catalog,&mut files,crate::load_view_variables::NativeOverrides::ControlledNone).unwrap();
  let (others,mut load)=Others::construct_with_load(&catalog,&variables).unwrap();
  assert_eq!([others.blank().creator().kind(),load.kind(),others.random().creator().kind(),others.preset().creator().kind(),others.metagame().creator().kind()],[crate::resource_creator::Kind::Blank,crate::resource_creator::Kind::Load,crate::resource_creator::Kind::Random,crate::resource_creator::Kind::Preset,crate::resource_creator::Kind::Metagame]);
  assert!(others.blank().doodify()&&others.metagame().doodify());assert_eq!(others.metagame().words_54_58(),[0;2]);assert!(others.random().config_is_null()&&others.preset().source_is_null());
  assert!(others.random().terrain_weights().is_empty()&&others.random().object_counts().is_empty());assert_eq!(others.random().num_heroes(),0);
  let streams=crate::map_resource_streams::Owner::open(map);
  let resource=crate::resource_creator::SourceResource::load(&catalog,&streams,&format!("Maps/{name}.RMP").encode_utf16().collect::<Vec<_>>()).unwrap();
  let bank=catalog.definition_bank.borrow();let header=game_settings_value::Owner::copy_source_header_5a46d0(&catalog,&bank,&resource).unwrap();load.assign_current_game_settings(&catalog,&bank,&header).unwrap();load.assign_source_resource(&catalog,&bank,std::sync::Arc::clone(&resource)).unwrap();
  let v=Launch{header:&header,load:&load,others:&others};let mut log=Log::default();dump(&catalog,&v,&mut log,"not-a-marker").unwrap();
  assert_eq!(log.events[0]["label"],"_settings.m_session_state");assert_eq!(log.events.iter().filter(|e|e["label"]=="(KKC_WorldCreator &)_wc").count(),5);
  for stop in [0,1,3,10,log.events.len()/2,log.events.len()-1] {let mut failed=Log{fail:Some(stop),..Log::default()};assert!(dump(&catalog,&v,&mut failed,"not-a-marker").is_err());assert_eq!(failed.events,log.events[..stop]);}
  let mut game=json!({"conditions":raw_rules(&catalog,header.rules()),"header":raw_parameters(&catalog,header.parameters(),header.labels(),header.closed30()),"version":header.words_0c_10()[0],"field_10":header.words_0c_10()[1],"time_low":header.words_18_1c()[0],"time_high":header.words_18_1c()[1],"pair_20":{"first":header.name_pair().unwrap().key(),"second":header.name_pair().unwrap().text()},"pair_28":{"first":header.description_pair().unwrap().key(),"second":header.description_pair().unwrap().text()},"field_30":header.display_extents_bits().unwrap()[0],"field_34":header.display_extents_bits().unwrap()[1],"text_38":header.string_38(),"text_3c":header.string_3c(),"field_40":header.word_40().unwrap(),"entries":[]});
  game["entries"]=json!(header.entries().iter().map(|r|json!({"text_0":r.string_18,"flag_4":r.flag!=0,"flag_5":r.flag_v5.unwrap()!=0,"text_8":r.string_20})).collect::<Vec<_>>());
  let mut blank=raw_creator(&catalog,others.blank().creator());blank["extra"]=json!(true);let mut selected=raw_creator(&catalog,&load);selected["source"]=json!(resource.ids());let mut random=raw_creator(&catalog,others.random().creator());random["source"]=Value::Null;random["rmc"]=json!({"total":0,"weights":[],"counts":[],"heroes":0});
  let native=json!({"game_type":2,"map_type":3,"file":[77,0,65,80],"launch_type":5,"game":game,"map_default":blank,"map_scenario":selected,"map_random":random,"map_24":raw_creator(&catalog,others.preset().creator()),"map_metagame":raw_creator(&catalog,others.metagame().creator()),"app":{"kind":7,"flag_04":true,"text":[65,0,66,55296],"field_0c":0,"field_10":3,"flag_14":false,"flag_15":true,"flag_16":false,"field_18":11,"field_1c":4,"flag_20":false,"field_24":2}});
  //Property uses its separate native registry producer. These actual maps and
  //fresh siblings never dereference it in this traversal; do not invent one.
  for p in [header.parameters(),load.parameters(),others.blank().creator().parameters(),others.random().creator().parameters(),others.preset().creator().parameters(),others.metagame().creator().parameters()] {assert!(p.kingdoms.iter().all(|k|k.definitions.is_empty()&&k.properties.is_empty()));}
  let registry= [ ("MAP_DEF_44","Biome"),("MAP_DEF_48","Weather"),("CONDITIONS","GameType"),("WORLD_RULES","WorldRule"),("NATIONS","Nation"),("FACTIONS","Faction"),("COLORS","KingdomColor"),("AI_HANDICAPS","Handicap")].into_iter().map(|(key,manager)|{let r=actors.native_data_registry(manager).unwrap_or_else(||panic!("actual diagnostic registry {manager} absent"));(key.to_owned(),json!({"count":r.entries.len(),"null":[],"kinds":if manager=="WorldRule"{header.rules().records().map(|r|r.kind()).collect::<Vec<_>>()}else{vec![]}}))}).collect::<serde_json::Map<_,_>>();
  captured.push(json!({"map":name,"native":native,"registries":registry,"unread_registries":["Property","Thing"],"events":log.events}));
 }
 if let Some(path)=std::env::var_os("AA_LAUNCH_DUMP_CAPTURE") {std::fs::write(path,serde_json::to_vec_pretty(&json!(captured)).unwrap()).unwrap();return}
 let fixture:Value=serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"),"/../../research/skirmish-20261017-launch-settings/native-actual-launch-dump.json")).unwrap()).unwrap();
 for capture in &captured {let rows=fixture["rows"].as_array().unwrap().iter().filter(|r|r["map"]==capture["map"]).collect::<Vec<_>>();assert_eq!(rows.len(),3);for row in rows {assert_eq!(capture["events"],row["output"]["events"]);}}
}
