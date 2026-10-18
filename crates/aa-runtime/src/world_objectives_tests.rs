use super::*;
fn units(text:&str)->Vec<u16>{text.encode_utf16().collect()}
fn string(bytes:&mut Vec<u8>,text:&str) {let words=units(text);bytes.extend((words.len()as u16).to_le_bytes());
    bytes.extend(words.iter().flat_map(|w|w.to_le_bytes()));}
fn chunk(bytes:Vec<u8>)->Chunk {
    let identity=aa_mapworld::SourceIdentity::of(&bytes);
    Chunk {map_source:identity.clone(),payload_source:identity,encoding:aa_mapworld::archive::Encoding::Raw,
        directory:aa_mapworld::ChunkDirectory {name:"OBJECTIVES".into(),directory_ordinal:0,packed_offset:0,
            packed_bytes:bytes.len()as u64,source_timestamp:0,decoded_bytes:bytes.len()as u64},bytes}
}
fn row(row:&Objective)->serde_json::Value {serde_json::json!({"key":row.key,
    "display_key":row.display_key,"display_text":row.display_text,"flag":row.flag})}
#[test]
fn native_constructor_field_prefix_and_lookup() {
    let fixture:serde_json::Value=serde_json::from_str(include_str!(
        "../../../research/development10/native-world-objectives.json")).unwrap();
    for case in fixture["rows"].as_array().unwrap() {
        let input=&case["input"];let output=&case["output"];
        let mut state=State::from_world_constructor();
        match input["kind"].as_str().unwrap() {
            "constructor"=> {
                assert_eq!(state.records.len()as u64,output["world_count"]);
                assert_eq!(u64::from(state.word_25c),output["word_25c"]);
                assert_eq!(row(&Objective::constructor()),output["row"]);
                assert!(state.require_loaded().is_err());
            }
            "find"=> {
                state.records=input["keys"].as_array().unwrap().iter().map(|key| {
                    let mut row=Objective::constructor();row.key=units(key.as_str().unwrap());row
                }).collect();state.phase=Phase::Loaded;
                let query=units(input["query"].as_str().unwrap());
                let actual=state.find_handle(&query).unwrap();
                assert_eq!(actual.as_ref().map(|h|h.index).map(|n|n as u64),output["index"].as_u64());
                if let Some(handle)=actual {
                    assert_eq!(prefix(state.get(&handle).unwrap().key()),prefix(&query));
                    let mut other=State::from_world_constructor();other.phase=Phase::Loaded;
                    assert!(other.get(&handle).is_err());
                }
            }
            "load"=> {
                let stop=input["stop"].as_str().unwrap();
                let mut bytes=1u32.to_le_bytes().to_vec();
                if stop!="version" {bytes.extend(0u32.to_le_bytes());
                    if stop!="key" {string(&mut bytes,input["key"].as_str().unwrap());
                        if stop!="pair" {string(&mut bytes,input["display_key"].as_str().unwrap());
                            if stop!="flag" {bytes.push(input["flag"].as_u64().unwrap()as u8);bytes.extend(0u32.to_le_bytes());}
                        }
                    }
                }
                state.input=Some(Ok(Some(chunk(bytes))));
                let result=state.load_with_lookup(|key| {
                    assert_eq!(key,units(input["display_key"].as_str().unwrap()));
                    Ok(units(input["display_text"].as_str().unwrap()))
                });
                assert_eq!(result.is_ok(),stop=="none");
                let actual=if result.is_ok(){assert_eq!(state.count().unwrap(),1);&state.records[0]}
                    else {assert!(state.records.is_empty());assert!(state.require_loaded().is_err());state.inflight.as_ref().unwrap()};
                assert_eq!(row(actual),output["row"],"{input}");
            }
            _=>unreachable!(),
        }
    }
}

#[test]
#[ignore="requires original AA_GAME_ROOT source/maps"]
fn actual_three_map_objective_lists_keep_loaded_pairs() {
    use aa_rules::{sources::{SourceLimits,V14Sources},actors::ActorCatalog,tgi::Limits};
    let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits {max_member_bytes:u64::MAX}).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();
    let settings=crate::settings::Settings::load_v14(&mut sources,Limits::default()).unwrap();
    let text=crate::localization::Localization::load_v14(&mut sources,Limits::default()).unwrap();
    for (name,count) in [("Campaign/tutorial02",10),("DeveloperMaps/(3v3)Firefight",0),("(2v2)Berlin",1)] {
        let map=MapArchive::from_bytes(std::fs::read(root.join(format!("reconstruction/assets/original/Data/Maps/{name}.RMP"))).unwrap()).unwrap();
        let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings.clone());
        let mut current=text.clone();current.add_map_strings(&map.decode_chunk("MAPSTRINGS",u64::MAX).unwrap(),Limits::default()).unwrap();
        catalog.replace_current_text(current);catalog.definition_text=Some(text.clone());
        let proof=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut sources).unwrap();
        crate::combat_source_completion::install(&catalog,&mut sources,proof.context(),proof.csv(),proof.globals()).unwrap();
        crate::combat_source_completion::compute(&catalog).unwrap();
        let mut state=State::from_world_constructor();state.bind_source(&map,28).unwrap();
        state.load_after_teams(&catalog).unwrap();assert_eq!(state.count().unwrap(),count);
        for objective in &state.records {
            let handle=state.find_handle(&objective.key).unwrap().unwrap();
            assert_eq!(state.get(&handle).unwrap().display().text(),objective.display_text);
        }
    }
}
