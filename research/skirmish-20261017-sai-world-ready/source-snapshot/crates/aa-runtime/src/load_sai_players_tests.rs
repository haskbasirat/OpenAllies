use super::*;
use serde_json::{Value,json};
#[derive(Debug)]struct ExplicitLiveAssociation;
impl SessionPlayer for ExplicitLiveAssociation {fn validate(&self)->Result<()> {Ok(())}}
fn fixture(name:&str)->Value {
    let p=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-sai-world-ready").join(name);
    serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap()
}
fn fixture_profile(v:&Value)->Profile {
    let mut p=SaiProfile::new("explicit-native-layout-fixture");
    p.difficulty=v["difficulty"].as_i64().unwrap()as i32;p.public=v["public"].as_bool().unwrap();
    p.nations=v["nations"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
    p.factions=v["factions"].as_array().unwrap().iter().map(|v|v.as_u64().unwrap()as u32).collect();
    Profile {fields:p,source:Provenance {path:"explicit-native-layout-fixture".into(),sha256:String::new(),span:aa_rules::tgi::Span {start:0,end:0}},initial_ego:3}
}
#[test]
fn native_pick_profile_filters_fallback_and_same_private_rng() {
    let f=fixture("native-pick-ai-profile.json");assert_eq!(f["rows"].as_array().unwrap().len(),48);
    for row in f["rows"].as_array().unwrap() {
        let c=&row["input"];let profiles=c["profiles"].as_array().unwrap().iter().map(fixture_profile).collect::<Vec<_>>();
        let mut rng=aa_mind::sai::manager::SaiRng {state:c["seed"].as_u64().unwrap()as u32};
        let word=|n:&str|{let w=c[n].as_u64().unwrap()as u32;(w!=0).then_some(w)};
        let selected=choose_profile(&profiles,word("nation"),word("faction"),&mut rng).unwrap();
        assert_eq!(json!({"selected":selected,"seed":rng.state}),json!({"selected":row["output"]["selected"],"seed":row["output"]["seed"]}));
    }
    let mut rng=aa_mind::sai::manager::SaiRng {state:1};
    let mut expected=rng.clone();expected.next_u32();
    assert!(choose_profile(&[],None,None,&mut rng).is_err());assert_eq!(rng,expected);
}
#[test]
fn native_player_constructor_and_slot_worklists() {
    let f=fixture("native-ai-player-constructor.json");assert_eq!(f["rows"].as_array().unwrap().len(),9);
    for row in f["rows"].as_array().unwrap() {
        let c=&row["input"];let output=&row["output"];
        let p=Player::new(c["slot"].as_u64().unwrap()as u32,ActorId::from_raw(782),0,3,Box::new(ExplicitLiveAssociation)).unwrap();
        assert_eq!(json!(p.slot),output["slot"]);assert!(!p.ready());assert_eq!(p.initial_ego,3);
        assert!(p.startup.controller.is_none());
        assert_eq!(json!([p.startup.word10,p.startup.word14]),output["words10_14"]);
        assert!(p.startup.region_kingdoms.is_empty());assert!(p.startup.region_kingdoms.capacity()>=4096);
        assert!(p.startup.script_words.is_empty()&&p.startup.script_words.capacity()>=1);
        for hashes in output["hashes"].as_array().unwrap() {
            let h=&p.hashes[&(hashes["offset"].as_u64().unwrap()as u32)];
            assert_eq!(json!(h.buckets.len()),hashes["capacity"]);assert_eq!(h.count,0);assert!(h.grow);
        }
        assert_eq!(p.hashes.len(),15);
        assert!(p.buckets.iter().all(|b|b.len()==17&&b.iter().all(VecDeque::is_empty)));
        assert_eq!(p.command_counts,[0,0]);
        for (cells,native) in p.commands.iter().zip(output["commands"].as_array().unwrap()) {
            assert_eq!(cells.len(),97);
            for (cell,native) in cells.iter().zip(native.as_array().unwrap()) {
                assert!(cell.data.actors.is_empty()&&cell.data.player.is_none()&&cell.data.command.is_none()&&cell.data.ability.is_none());
                assert_eq!(json!([0,cell.word_c,0,0]),native["data_words"]);
            }
        }
    }
    let f=fixture("native-add-ai-player.json");assert_eq!(f["rows"].as_array().unwrap().len(),18);
    for row in f["rows"].as_array().unwrap().iter().filter(|r|r["output"]["result"]!=u32::MAX) {
        let c=&row["input"];let o=&row["output"];let mut arena=Arena::constructed();let mut manager=aa_mind::sai::manager::Sai::new(1);
        for (i,present) in c["slots"].as_array().unwrap().iter().enumerate() {
            if present.is_null() {manager.players.push(None);continue}
            let p=Box::new(Player::new(i as u32,ActorId::from_raw(900+i as u32),0,3,Box::new(ExplicitLiveAssociation)).unwrap());
            let handle=arena.next;arena.next+=1;arena.allocations.insert(handle,p);manager.players.push(Some(handle));
        }
        manager.players_4c=VecDeque::from([8,9]);manager.players_54=VecDeque::from([4]);
        let slot=manager.players.iter().position(Option::is_none).unwrap_or(manager.players.len());
        arena.publish(&mut manager,Box::new(Player::new(slot as u32,ActorId::from_raw(782),0,3,Box::new(ExplicitLiveAssociation)).unwrap())).unwrap();
        assert_eq!(json!(slot),o["result"]);assert_eq!(json!(manager.players_4c),o["list4c"]);assert_eq!(json!(manager.players_54),o["list54"]);
        assert_eq!(manager.players.len(),o["slots"].as_array().unwrap().len());
    }
}
#[test]
fn same_region_and_manager90_list_owners_match_full_native_category0() {
    let f=fixture("native-actor-map-lists.json");let rows=f["rows"].as_array().unwrap();assert_eq!(rows.len(),36);
    for row in rows {
        let c=&row["input"];let output=&row["output"];
        let word=|name:&str|c[name].as_u64().unwrap()as u32;
        let water=crate::load_sai_lists::water(word("flags"),word("low"),word("high"))||c["members"].as_array().unwrap().iter().any(|v|v==true);
        assert_eq!(water,c["water"].as_bool().unwrap());
        let mut region=crate::load_sai_lists::RegionLists::constructed().unwrap();
        let mut arena=Arena::constructed();let mut vector=Vec::new();let index=word("owner_index")as usize;
        for _ in 0..if c["twice"].as_bool().unwrap(){2}else{1} {
            region.prepend(water,index,word("actor_id")).unwrap();
            arena.prepend_manager_actor(&mut vector,index,word("actor_id")).unwrap();
        }
        assert_eq!(json!(region.land),output["land"]);assert_eq!(json!(region.water),output["water"]);
        assert_eq!(json!(vector.iter().map(|id|&arena.manager_lists[id]).collect::<Vec<_>>()),output["manager90"]);
        assert!(!arena.is_empty());assert!(!arena.has_player_allocations(),"real list allocations are separate from AIPlayer allocations");
        let prior=vector.clone();let mut foreign=vec![u32::MAX];
        assert!(arena.prepend_manager_actor(&mut foreign,0,word("actor_id")).is_err());assert_eq!(foreign,[u32::MAX]);assert_eq!(vector,prior);
    }
}

#[test]
#[ignore="requires actual original/v14 source and verified source Bank"]
fn actual_source_profiles_and_ego_heads() {
    use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut sources=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits {max_member_bytes:64*1024*1024}).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();let settings=crate::settings::Settings::load_v14(&mut sources,Limits::default()).unwrap();
    let resources=std::sync::Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings);catalog.bind_resource_catalog(resources).unwrap();
    catalog.definition_text=Some(crate::localization::Localization::load_v14(&mut sources,Limits::default()).unwrap());
    let receipt=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut sources).unwrap();
    crate::combat_source_completion::install(&catalog,&mut sources,receipt.context(),receipt.csv(),receipt.globals()).unwrap();crate::combat_source_completion::compute(&catalog).unwrap();
    let bank=catalog.definition_bank.borrow();let mut arena=Arena::constructed();arena.bind_profiles(&catalog,&bank).unwrap();
    assert_eq!(arena.profiles.len(),25);assert!(!arena.ego_ids.is_empty());assert!(arena.allocations.is_empty());
    let prior:Value=serde_json::from_str(include_str!("../../../research/phase8-ai/native-saidata-profile.json")).unwrap();
    for profile in &arena.profiles {
        let rows=prior["cases"].as_array().unwrap().iter().filter(|r|r["input"]["name"].as_str().unwrap().starts_with("shipped_")
            &&r["input"]["ids"].as_str().unwrap().eq_ignore_ascii_case(&profile.fields.ids)).collect::<Vec<_>>();assert_eq!(rows.len(),1);
        let o=&rows[0]["output"];let p=&profile.fields;
        assert_eq!(json!([p.difficulty as u32,p.command_max_delay_ms as u32,p.command_max_total_delay_ms as u32,p.max_ge_commands as u32,p.max_rai_commands as u32,p.ego_fuzziness]),
            json!([o["difficulty"],o["command_max_delay_ms"],o["command_max_total_delay_ms"],o["max_ge_commands"],o["max_rai_commands"],o["ego_fuzziness"]]));
        assert_eq!(json!(p.ego_refs),o["ego_refs"]);assert_eq!(arena.ego_ids[profile.initial_ego],p.ego_refs[0]);
    }
    let native_egos:Value=serde_json::from_str(include_str!("../../../research/phase8-ai/native-saidata-ego.json")).unwrap();
    for (i,id) in arena.ego_ids.iter().enumerate() {
        let matching=native_egos["cases"].as_array().unwrap().iter().filter(|row|row["ego"]==i&&row["precision"]=="Double53"&&row["decoded"].is_object()).collect::<Vec<_>>();
        let [row]=matching.as_slice() else {panic!("one frozen native Ego for actual ordinal {i}/{id}")};
        let image=row["decoded"]["image"].as_str().unwrap();
        let word=|offset:usize|i32::from_le_bytes(std::array::from_fn(|j|u8::from_str_radix(&image[2*(offset+j)..2*(offset+j)+2],16).unwrap()));
        assert_eq!(ego_ability_counts(&catalog,aa_numeric::Precision::Double53,id).unwrap(),[word(0x464),word(0x528),word(0x5ec)],"{id}");
        assert_eq!(ego_neighbor_depth(&catalog,aa_numeric::Precision::Double53,id).unwrap(),word(0x3f4),"{id}");
    }
    if let Some(path)=std::env::var_os("AA_SAI_PROFILE_CAPTURE") {
        let file=std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap();
        serde_json::to_writer_pretty(file,&json!({"source_precision":arena.source_precision,"sources":arena.source_files,"ego_ids":arena.ego_ids,
            "profiles":arena.profiles.iter().map(|p|json!({"ids":p.ids(),"name":p.name(),"source":p.source,"difficulty":p.fields.difficulty,"public":p.fields.public,
                "nations":p.fields.nations,"factions":p.fields.factions,"ego_refs":p.fields.ego_refs,"egos":p.fields.egos,"head_ego":p.initial_ego})).collect::<Vec<_>>()})).unwrap();
    }
}
