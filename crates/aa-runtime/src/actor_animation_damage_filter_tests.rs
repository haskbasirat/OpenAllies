//!Replays research/development10/actor-animation-damage-filter (actual
//!461F70 on DamageType vectors built by actual 481410/481570/4C0A80).
use super::*;
use super::super::core::{FxList,Set,Source,registered_kind};
use aa_rules::{actors::native_load::{NativeLoadReport,load_files},sources::{Origin,ResolvedFile,sha256},tgi::{Limits,Span}};
use serde_json::{Value,json};
const INVALID:&str="Animation '%s' for actor '%s' specifies an invalid damage filter '%d'";
fn fixture()->Value{
    let path=concat!(env!("CARGO_MANIFEST_DIR"),"/../../research/development10/actor-animation-damage-filter/native-damage-filter.json");
    serde_json::from_str(&std::fs::read_to_string(path).unwrap()).unwrap()
}
fn pc(name:&str)->Precision{match name{"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!("{name}")}}
///Native vector +24 order with the RAW +2C words (including a never-stored
///word, which the native compares like any other).
fn native_vector(registry:&Value)->Option<Vec<i32>>{
    Some(registry["entries"].as_array().unwrap().iter().map(|e|e["raw_2c"].as_i64().unwrap()as i32).collect())
}
///Typed node getter / Sequence transports identical to the producer's.
struct Input<'a>{input:&'a Value,trace:Vec<String>,vector:Option<Vec<i32>>}
impl Input<'_>{fn read(&mut self,key:&str){self.trace.push(format!("read:{key}"))}}
impl Source<usize,Vec<()>> for Input<'_>{
    fn string(&mut self,key:&str)->Result<String>{self.read(key);Ok(self.input[key].as_str().unwrap().into())}
    fn resource(&mut self,path:&str)->Result<Option<usize>>{
        let path=format!("{}/{path}",self.input["folder"].as_str().unwrap());
        self.trace.push(format!("relative:{path}"));self.trace.push(format!("sequence:{path}"));Ok(Some(7))
    }
    fn fixed(&mut self,key:&str)->Result<u32>{self.read(key);Ok(self.input[key].as_u64().unwrap()as u32)}
    fn boolean(&mut self,key:&str)->Result<u8>{self.read(key);Ok(self.input[key].as_u64().unwrap()as u8)}
    fn integer(&mut self,key:&str)->Result<i32>{self.read(key);Ok(self.input[key].as_i64().unwrap()as i32)}
    fn resolve_kind(&mut self,text:&str)->Result<i32>{Ok(registered_kind(text))}
    fn validate_damage_filter(&mut self,filter:i32)->Result<()>{
        //[B36530]+3A4 == 0: the native faults at4620BA; Rust reports it unbound.
        let vector=self.vector.as_ref().ok_or("461F70 DamageType DataManager [B36530]+3A4 is unbound")?;
        if lookup(vector.iter().copied(),filter){Ok(())}else{Err(invalid_filter(self.input["ids"].as_str().unwrap(),"actor_ids",filter))}
    }
    fn begin_fx(&mut self)->Result<Vec<()>>{Ok(vec![])}
    fn append_fx(&mut self,context:&mut Vec<()>,list:FxList)->Result<usize>{
        let FxList::Standard(index)=list else{panic!("no timed cue in this fixture")};
        self.trace.push(format!("children:{}",["WhenStart","WhenDo","WhenEnd"][index]));context.push(());Ok(context.len()-1)
    }
    fn permanent(&mut self,_:&Vec<()>,_:usize)->Result<bool>{Ok(false)}
    fn timed_count(&mut self)->Result<usize>{self.trace.push("children:WhenDoAtTime".into());Ok(0)}
    fn timed_time(&mut self,_:usize)->Result<u32>{unreachable!()}
    fn finish_fx(&mut self,_:&mut Vec<()>)->Result<()>{Ok(())}
}
fn replay(case:&Value,vector:Option<Vec<i32>>,precision:Precision,label:&str){
    let input=&case["input"];
    let mut set=Set::constructed();let mut source=Input{input,trace:vec!["children:Animation".into()],vector};
    let result=set.load(input["prefix"].as_str().unwrap(),false,&mut source,precision);
    let native=case["status"].as_str().unwrap();
    match &result{
        Ok(())=>assert_eq!(native,"ok","{label} {case}"),
        Err(e)if e.starts_with("410C40")=>{
            source.trace.push("diagnostic".into());assert_eq!(native,"required:diagnostic","{label} {case}");
            let d=&case["diagnostic"];
            if d["format"]==INVALID{
                let args=d["args"].as_array().unwrap();
                assert_eq!(*e,invalid_filter(args[0].as_str().unwrap(),args[1].as_str().unwrap(),args[2].as_i64().unwrap()as i32),"{label}");
            }
        }
        Err(e)=>assert!(native.starts_with("fault:0x4620ba")&&e.contains("3A4 is unbound"),"{label} {e} {case}"),
    }
    assert_eq!(json!(source.trace),case["trace"],"{label} {case}");
    assert_eq!(set.rows.len()as u64,case["count"].as_u64().unwrap(),"{label}");
    let last=set.rows.last().unwrap();
    assert_eq!(last.kind as u64,case["kind"].as_u64().unwrap(),"{label}");
    assert_eq!(last.damage_filter as i64,case["stored_filter"].as_i64().unwrap(),"{label}");
    assert_eq!(set.weights[3]as u64,case["complete_weight"].as_u64().unwrap(),"{label}");
}
#[test]
fn actual_461f70_damage_filter_hits_misses_and_skips_replay_native_all_pc(){
    let doc=fixture();let mut cases=0;
    for row in doc["rows"].as_array().unwrap(){
        let precision=pc(row["precision"].as_str().unwrap());let out=&row["output"];
        for case in out["lookups"].as_array().unwrap(){
            let name=case["registry"].as_str().unwrap();
            let vector=if name=="absent"{None}else{native_vector(&out["registries"][name])};
            replay(case,vector,precision,&format!("{} {name}",row["precision"]));cases+=1;
        }
        let real=native_vector(&out["registries"]["real"]);
        for definition in out["real_definitions"].as_array().unwrap(){
            for case in definition["animations"].as_array().unwrap(){
                replay(case,real.clone(),precision,definition["source"].as_str().unwrap());cases+=1;
                assert_eq!(case["status"],"ok");
            }
        }
    }
    assert_eq!(cases,3*(95+66));
}
fn file(path:&str,bytes:Vec<u8>)->ResolvedFile{ResolvedFile{logical_path:path.into(),origin:Origin::V14Loose,sha256:sha256(&bytes),bytes}}
fn dup(f:&ResolvedFile)->ResolvedFile{file(&f.logical_path,f.bytes.clone())}
///Fixture text is latin-1 decoded: every char is one original byte.
fn bytes(source:&Value)->Vec<u8>{
    let raw:Vec<u8>=source["text"].as_str().unwrap().chars().map(|c|u8::try_from(u32::from(c)).unwrap()).collect();
    assert_eq!(sha256(&raw),source["sha256"].as_str().unwrap());raw
}
fn glyph_file(doc:&Value)->ResolvedFile{
    let text:String=doc["glyph_seam"].as_array().unwrap().iter().map(|g|format!("[Glyph]\r\n{{\r\n\tIDS = {}\r\n}}\r\n",g.as_str().unwrap())).collect();
    file("Fonts/fixture_glyphs.tgi",text.into_bytes())
}
fn actor_file()->(ResolvedFile,Provenance){
    let f=file("Units/fixture_actor.tgi",b"[Template FixtureOnly]\r\n{\r\n}\r\n".to_vec());
    let p=Provenance{path:f.logical_path.clone(),sha256:f.sha256.clone(),span:Span{start:0,end:0}};(f,p)
}
fn catalog(files:Vec<ResolvedFile>)->ActorCatalog{let mut report=NativeLoadReport::default();let c=load_files(files,Limits::default(),&mut report);assert_eq!(report.exceptions().count(),0,"{report:?}");c}
#[test]
fn actual_damage_type_vector_projection_matches_native_4c0a80_registration(){
    let doc=fixture();let template=file(doc["template"]["path"].as_str().unwrap(),bytes(&doc["template"]));
    for row in doc["rows"].as_array().unwrap(){
        let precision=pc(row["precision"].as_str().unwrap());let out=&row["output"];
        for (name,source) in doc["registry_sources"].as_object().unwrap(){
            if name=="absent"{continue}
            let native=&out["registries"][name.as_str()];
            let (actor,actor_source)=actor_file();
            let mut files=vec![dup(&template),glyph_file(&doc)];
            if !source.is_null(){files.push(file(source["path"].as_str().unwrap(),bytes(source)));}
            files.push(actor);
            let actors=catalog(files);
            let result=Registry::at_actor_load(&actors,&actor_source,precision);
            let roots=native["roots"].as_array().unwrap();
            match roots.iter().position(|r|r["status"]=="thrown"){
                None=>{
                    let registry=result.unwrap_or_else(|e|panic!("{name}: {e}"));
                    let expected:Vec<Entry>=native["entries"].as_array().unwrap().iter().map(|e|{
                        assert_eq!(e["stored"],true);assert_eq!(e["vtable"],0xa228d0);
                        Entry{ids:e["ids"].as_str().unwrap().into(),index:e["index"].as_u64().unwrap()as u32,filter:e["raw_2c"].as_i64().unwrap()as i32}
                    }).collect();
                    assert_eq!(registry.entries,expected,"{name}");
                    for case in out["lookups"].as_array().unwrap().iter().filter(|c|c["registry"]==name.as_str()){
                        let filter=case["input"]["damage_filter"].as_i64().unwrap()as i32;
                        if case["kind"]==3&&filter!= -1&&filter!=666{
                            assert_eq!(registry.contains(filter),case["status"]=="ok","{name} {filter}");
                        }
                    }
                }
                Some(thrown)=>{
                    //Native registered the throwing entry, then 48D660 stopped
                    //the file; the canonical registry has every root, so Rust
                    //must refuse instead of projecting a different vector.
                    assert!(native["entries"].as_array().unwrap().len()==thrown+1);
                    let error=result.unwrap_err();
                    let format=roots[thrown]["message"].as_str().unwrap();
                    let ids=native["entries"][thrown]["ids"].as_str().unwrap();
                    assert!(error.starts_with("4C0A80")&&error.contains(&format!("'{ids}'")),"{name}: {error}");
                    assert!(format=="Unknown special glyph '%s' loading damage type '%s'"&&error.contains("Unknown special glyph")
                        ||format=="Damage type '%s' specifies an invalid damage_filter '%d'"&&error.contains("invalid damage_filter"),"{name}: {error}");
                    assert_eq!(native["entries"][thrown]["stored"],format.contains("invalid damage_filter"));
                }
            }
        }
    }
}
///Load-order rule (464BD0 runs inside its own file's 48D660 dispatch). This
///is the catalog ordering contract, not an additional oracle claim.
#[test]
fn damage_type_vector_is_taken_at_the_actor_file_time(){
    let doc=fixture();let template=file(doc["template"]["path"].as_str().unwrap(),bytes(&doc["template"]));
    let real=file("Game/damage_types.tgi",bytes(&doc["registry_sources"]["real"]));
    let (actor,actor_source)=actor_file();
    let later=catalog(vec![dup(&template),glyph_file(&doc),dup(&actor),dup(&real)]);
    assert_eq!(Registry::at_actor_load(&later,&actor_source,Precision::Double53).unwrap().entries,vec![]);
    let glyph_late=catalog(vec![dup(&template),dup(&real),glyph_file(&doc),dup(&actor)]);
    assert!(Registry::at_actor_load(&glyph_late,&actor_source,Precision::Double53).unwrap_err().contains("Unknown special glyph 'ch_gun'"));
    let same=file("Units/fixture_actor.tgi",bytes(&doc["registry_sources"]["real"]));
    let p=Provenance{path:same.logical_path.clone(),sha256:same.sha256.clone(),span:Span{start:0,end:0}};
    assert!(Registry::at_actor_load(&catalog(vec![template,glyph_file(&doc),same]),&p,Precision::Double53).unwrap_err().contains("Actor's own file"));
    assert!(Registry::at_actor_load(&ActorCatalog::from_files(vec![],Limits::default()),&actor_source,Precision::Double53).unwrap_err().contains("unbound"));
}
///Actual v14 sources: the infantry/special-ops definitions the pre-World
///census stopped at, through the production463B50/463440/461F70 core.
struct Real<'a,'n>{node:&'a aa_rules::actors::ResolvedNode<'n>,pc:Precision,registry:&'a Registry,actor:&'a str,cues:usize}
impl Source<(),()> for Real<'_,'_>{
    fn string(&mut self,key:&str)->Result<String>{super::super::text(self.node,key,self.pc)}
    fn resource(&mut self,_:&str)->Result<Option<()>>{Ok(None)}
    fn fixed(&mut self,key:&str)->Result<u32>{super::super::number(self.node,key,self.pc,aa_rules::native::ScalarType::Fixed)}
    fn boolean(&mut self,key:&str)->Result<u8>{Ok(self.node.field(key,self.pc).map_err(|e|e.to_string())?.ok_or("Bool absent")?.boolean().map_err(|e|e.to_string())?as u8)}
    fn integer(&mut self,key:&str)->Result<i32>{self.node.field(key,self.pc).map_err(|e|e.to_string())?.ok_or("Int absent")?.integer().map_err(|e|e.to_string())}
    fn resolve_kind(&mut self,value:&str)->Result<i32>{let kind=registered_kind(value);if kind>=0{Ok(kind)}else{Err(format!("unregistered kind {value}"))}}
    fn validate_damage_filter(&mut self,filter:i32)->Result<()>{
        if self.registry.contains(filter){Ok(())}else{Err(invalid_filter(&super::super::text(self.node,"ids",self.pc)?,self.actor,filter))}
    }
    fn begin_fx(&mut self)->Result<()>{Ok(())}
    fn append_fx(&mut self,_:&mut (),_:FxList)->Result<usize>{Ok(0)}
    fn permanent(&mut self,_:&(),_:usize)->Result<bool>{Ok(false)}
    fn timed_count(&mut self)->Result<usize>{self.cues=self.node.children("WhenDoAtTime").map_err(|e|e.to_string())?.len();Ok(self.cues)}
    fn timed_time(&mut self,index:usize)->Result<u32>{
        let cues=self.node.children("WhenDoAtTime").map_err(|e|e.to_string())?;
        super::super::number(&cues[index],"time",self.pc,aa_rules::native::ScalarType::Fixed)
    }
    fn finish_fx(&mut self,_:&mut ())->Result<()>{Ok(())}
}
const CENSUS:&[&str]=&["ger_engineer","ger_infantry_amphib","ger_infantry","ger_infantry_at","ger_infantry_co","ger_infantry_mg","ger_infantry_mortar","ger_paratrooper","ger_paratrooper_co","ger_paratrooper_mg","ger_paratrooper_mortar",
    "gbr_engineer","gbr_infantry","gbr_infantry_amphib","gbr_infantry_at","gbr_infantry_co","gbr_infantry_mg","gbr_infantry_mortar","gbr_paratrooper","gbr_paratrooper_co","gbr_paratrooper_mg","gbr_paratrooper_mortar","sas_commando","sas_commando_co","sas_sniper",
    "jpn_engineer","jpn_infantry","jpn_infantry_amphib","jpn_infantry_at","jpn_infantry_banzai","jpn_infantry_co","jpn_infantry_mg","jpn_infantry_mortar","jpn_infantry_sniper","jpn_paratrooper","jpn_infantry_paratrooper_banzai","jpn_paratrooper_co","jpn_paratrooper_mg","jpn_paratrooper_mortar",
    "rus_conscript","rus_engineer","rus_infantry_amphib","rus_infantry","rus_infantry_at","rus_infantry_co","rus_infantry_mg","rus_infantry_mortar","rus_infantry_sniper","rus_paratrooper","rus_paratrooper_co","rus_paratrooper_mg","rus_paratrooper_mortar",
    "usa_engineer","usa_infantry","usa_infantry_amphib","usa_infantry_assault","usa_infantry_at","usa_infantry_co","usa_infantry_flame","usa_infantry_marine","usa_infantry_mg","usa_infantry_mortar","usa_paratrooper","usa_paratrooper_co","usa_paratrooper_mg","usa_paratrooper_mortar",
    "saboteur","secret_agent"];
#[test]
#[ignore = "requires AA_GAME_ROOT actual original/v14 source files"]
fn actual_v14_census_death_filters_resolve_in_native_damage_type_vector(){
    use aa_rules::sources::{SourceLimits,V14Sources};
    let doc=fixture();let out=&doc["rows"][1]["output"];assert_eq!(doc["rows"][1]["precision"],"Double53");
    let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();let precision=Precision::Double53;
    let source=actors.sources.iter().find(|s|s.path=="Game/damage_types.tgi").unwrap();
    assert_eq!(source.sha256,doc["registry_sources"]["real"]["sha256"].as_str().unwrap());
    let native:Vec<Entry>=out["registries"]["real"]["entries"].as_array().unwrap().iter().map(|e|Entry{ids:e["ids"].as_str().unwrap().into(),index:e["index"].as_u64().unwrap()as u32,filter:e["raw_2c"].as_i64().unwrap()as i32}).collect();
    let outcome=|filter:i32|out["lookups"].as_array().unwrap().iter().find(|c|c["registry"]=="real"&&c["input"]["type"]=="die"&&c["input"]["damage_filter"]==filter).map(|c|c["status"]=="ok");
    let mut deaths=0;let mut listed_definitions=0;
    assert_eq!(CENSUS.len(),68);
    for id in CENSUS{
        let definition=actors.find(id).unwrap_or_else(||panic!("{id}"));
        let registry=Registry::at_actor_load(&actors,&definition.source,precision).unwrap_or_else(|e|panic!("{id}: {e}"));
        assert_eq!(registry.entries,native,"{id}");
        let actor=actors.resolved_actor(*id).unwrap();
        let character=actor.child("CharacterComponent").unwrap().unwrap();
        let base=character.children("Animation").unwrap();
        let mut sets=vec![(Set::<(),()>::constructed(),base.iter().map(|n|(n.clone(),false)).collect::<Vec<_>>())];
        for scope in character.children("AspectAnimationSet").unwrap(){
            let mut nodes:Vec<_>=scope.children("Animation").unwrap().into_iter().map(|n|(n,false)).collect();
            nodes.extend(base.iter().map(|n|(n.clone(),true)));sets.push((Set::constructed(),nodes));
        }
        for (set,nodes) in &mut sets{
            for (node,inherit) in nodes.iter(){
                let mut real=Real{node,pc:precision,registry:&registry,actor:id,cues:0};
                set.load("prefix_",*inherit,&mut real,precision).unwrap_or_else(|e|panic!("{id}: {e}"));
            }
            for a in set.rows.iter().filter(|a|a.kind==3){
                deaths+=1;
                assert!(matches!(a.damage_filter,-1|0|1|2|666),"{id} {}",a.damage_filter);
                if !matches!(a.damage_filter,-1|666){assert_eq!(outcome(a.damage_filter),Some(true),"{id} {}",a.damage_filter);}
            }
        }
        //The producer's authored real-definition lists are the base set's
        //death Animations in source order.
        if let Some(listed)=out["real_definitions"].as_array().unwrap().iter().find(|d|d["source"].as_str().unwrap()==definition.source.path){
            let native:Vec<(String,i64)>=listed["animations"].as_array().unwrap().iter().map(|a|(a["input"]["ids"].as_str().unwrap().into(),a["input"]["damage_filter"].as_i64().unwrap())).collect();
            let rust:Vec<(String,i64)>=sets[0].0.rows.iter().filter(|a|a.kind==3).map(|a|(a.ids.clone(),a.damage_filter as i64)).collect();
            assert_eq!(rust,native,"{id}");listed_definitions+=1;
        }
        for (set,_) in &mut sets{set.finalize(precision).unwrap_or_else(|e|panic!("{id}: {e}"));}
    }
    assert_eq!(listed_definitions,3);
    assert!(deaths>68*20,"{deaths}");
}
