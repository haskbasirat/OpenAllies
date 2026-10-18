use super::*;
use serde_json::Value;

struct Supplied {starting:u32,maximum:u32,reads:usize,fail:Option<usize>}
impl Values for Supplied {
    fn starting_stockpile(&mut self)->Result<u32> {Ok(self.starting)}
    fn maximum(&mut self)->Result<u32> {let n=self.reads;self.reads+=1;if self.fail==Some(n){Err("maximum_required".into())}else{Ok(self.maximum)}}
}

#[test]
fn existing_native_no_commander_start_rows_use_same_vector_and_status_words() {
    let fixture:Value=serde_json::from_str(include_str!("../../aa-economy/tests/fixtures/native-start-mission-fixtures.json")).unwrap();
    let mut compared=0;
    for row in fixture["fixtures"].as_array().unwrap() {
        let c=&row["case"];
        if !c["commander"].is_null() || !c["profile"].is_null() || c["start_technologies"].as_array().unwrap().iter().any(|v|v.as_u64()!=Some(0)) {continue}
        let precision=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
        let stock:Vec<u32>=serde_json::from_value(c["stock"].clone()).unwrap();
        let mut shell=SpecialShell::kingdom(aa_actors::ActorId::from_raw(1),stock.len() as u32).unwrap();
        shell.vectors.get_mut(&0x1c4).unwrap().values=stock;
        shell.vectors.get_mut(&0x258).unwrap().values=serde_json::from_value(c["starting_resources"].clone()).unwrap();
        store_byte(&mut shell,0x23a,u8::from(c["reset"].as_bool().unwrap()));
        let ptr=shell.vectors.get(&0x1c4).unwrap().values.as_ptr();
        let mut values=Supplied{starting:c["starting_stockpile"].as_u64().unwrap() as u32,maximum:c["maximum"].as_u64().unwrap() as u32,reads:0,fail:None};
        start(&mut shell,&mut values,precision).unwrap();
        assert_eq!(ptr,shell.vectors.get(&0x1c4).unwrap().values.as_ptr());
        assert_eq!(serde_json::json!(shell.vectors.get(&0x1c4).unwrap().values),row["expected"]["stock"]);
        assert_eq!(shell.words.get(&0x19c),Some(&0));assert_eq!(byte(&shell,0x23b).unwrap(),1);
        compared+=1;
    }
    assert_eq!(compared,24);
}

#[test]
fn required_maximum_preserves_first_native_store() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-mission-start/native-kingdom-prefix.json")).unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(),36);
    for row in fixture["cases"].as_array().unwrap() {
        let c=&row["input"];let n=&row["output"];
        let precision=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,"Extended64"=>Precision::Extended64,_=>panic!()};
        let mut shell=SpecialShell::kingdom(aa_actors::ActorId::from_raw(1),2).unwrap();
        store_byte(&mut shell,0x23a,c["reset"].as_u64().unwrap() as u8);shell.words.insert(0x19c,9);store_byte(&mut shell,0x23b,0);
        shell.vectors.get_mut(&0x1c4).unwrap().values=serde_json::from_value(c["stock"].clone()).unwrap();
        shell.vectors.get_mut(&0x258).unwrap().values=serde_json::from_value(c["delta"].clone()).unwrap();
        if c["nation"]==1 {
            shell.words.remove(&0x234);shell.references.insert(0x234,Some(aa_mapworld::restore::NamedDefinition{id:"explicit-native-Nation-4730D0-null-Commander-fixture".into(),index:None}));
        }
        let mut values=Supplied{starting:c["starting"].as_u64().unwrap() as u32,maximum:c["maximum"].as_u64().unwrap() as u32,reads:0,fail:c["fail_max"].as_u64().map(|v|v as usize)};
        let result=start(&mut shell,&mut values,precision);
        assert_eq!(result.is_ok(),n["outcome"]=="ok","{row}");
        if let Err(e)=result {assert_eq!(e,n["outcome"].as_str().unwrap())}
        assert_eq!(serde_json::json!(shell.vectors[&0x1c4].values),n["stock"],"{row}");
        assert_eq!(values.reads,n["maximum_reads"].as_u64().unwrap() as usize);
        assert_eq!(shell.words[&0x19c],n["status"].as_u64().unwrap() as u32);
        assert_eq!(byte(&shell,0x23b).unwrap(),n["started"].as_u64().unwrap() as u8);
    }
}

struct CommanderValues {base:Supplied,experience:u32}
impl Values for CommanderValues {
    fn starting_stockpile(&mut self)->Result<u32>{self.base.starting_stockpile()}
    fn maximum(&mut self)->Result<u32>{self.base.maximum()}
    fn commander_properties(&mut self,_:u32)->Result<Vec<u32>>{Ok(Vec::new())}
    fn faction_allowed(&mut self,_:&SpecialShell)->Result<bool>{Ok(true)}
    fn starting_experience(&mut self)->Result<u32>{Ok(self.experience)}
}
#[test]
fn existing_native_positive_commander_empty_property_rows_keep_same_experience_owner() {
    let fixture:Value=serde_json::from_str(include_str!("../../aa-economy/tests/fixtures/native-start-mission-fixtures.json")).unwrap();
    let mut compared=0;
    for row in fixture["fixtures"].as_array().unwrap() {
        let c=&row["case"];
        if c["commander"].is_null()||!c["commander"]["technologies"].as_array().unwrap().is_empty()
            ||!c["profile"].is_null()||c["start_technologies"].as_array().unwrap().iter().any(|v|v.as_u64()!=Some(0))
            ||!row["expected"]["events"].as_array().unwrap().is_empty(){continue}
        let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,_=>Precision::Extended64};
        let stock:Vec<u32>=serde_json::from_value(c["stock"].clone()).unwrap();
        let mut shell=SpecialShell::kingdom(aa_actors::ActorId::from_raw(1),stock.len()as u32).unwrap();
        shell.vectors.get_mut(&0x1c4).unwrap().values=stock;
        shell.vectors.get_mut(&0x258).unwrap().values=serde_json::from_value(c["starting_resources"].clone()).unwrap();
        store_byte(&mut shell,0x23a,u8::from(c["reset"].as_bool().unwrap()));
        shell.commander=Some(aa_combat::commander::State::with_maximum(&aa_combat::commander::Definition{id:99,abilities:Vec::new()},aa_body::F32(c["commander"]["maximum"].as_u64().unwrap()as u32),1).unwrap());
        shell.commander.as_mut().unwrap().experience.balance=F32Bits(c["commander"]["balance"].as_u64().unwrap()as u32);
        shell.words.remove(&0x230);
        let before=&shell.commander.as_ref().unwrap().experience as*const _ as usize;
        let mut values=CommanderValues{base:Supplied{starting:c["starting_stockpile"].as_u64().unwrap()as u32,maximum:c["maximum"].as_u64().unwrap()as u32,reads:0,fail:None},experience:c["commander_start"].as_u64().unwrap()as u32};
        start(&mut shell,&mut values,pc).unwrap();
        assert_eq!(&shell.commander.as_ref().unwrap().experience as*const _ as usize,before);
        assert_eq!(shell.commander.as_ref().unwrap().experience.balance.0,row["expected"]["commander_balance"].as_u64().unwrap()as u32,"{row}");
        assert_eq!(serde_json::json!(shell.vectors[&0x1c4].values),row["expected"]["stock"],"{row}");
        assert_eq!(shell.words[&0x19c],0);assert_eq!(byte(&shell,0x23b).unwrap(),1);compared+=1;
    }
    assert_eq!(compared,9);
}

struct PrefixValues {base:Supplied,experience:u32,property:bool,allowed:bool,fail_experience:bool,reads:usize,events:Vec<&'static str>}
impl Values for PrefixValues {
    fn starting_stockpile(&mut self)->Result<u32>{self.base.starting_stockpile()}
    fn maximum(&mut self)->Result<u32>{self.base.maximum()}
    fn commander_properties(&mut self,_:u32)->Result<Vec<u32>>{Ok(if self.property{vec![3]}else{Vec::new()})}
    fn add_property(&mut self,_:&mut SpecialShell,_:u32)->Result<()>{self.events.push("property");Err("property_required".into())}
    fn faction_allowed(&mut self,_:&SpecialShell)->Result<bool>{Ok(self.allowed)}
    fn invalid_faction(&mut self)->Result<()>{self.events.push("warning");Err("warning_required".into())}
    fn starting_experience(&mut self)->Result<u32>{self.reads+=1;if self.fail_experience{Err("experience_required".into())}else{Ok(self.experience)}}
}
#[test]
fn original_positive_commander_whole_path_and_required_prefixes_match_both_images() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-kingdom-start-positive/native-commander-start.json")).unwrap();
    assert_eq!(fixture["rows"].as_array().unwrap().len(),384);
    for row in fixture["rows"].as_array().unwrap() {
        let c=&row["input"];let expected=&row["output"];
        let pc=match row["precision"].as_str().unwrap(){"Single24"=>Precision::Single24,"Double53"=>Precision::Double53,_=>Precision::Extended64};
        let mut shell=SpecialShell::kingdom(aa_actors::ActorId::from_raw(1),2).unwrap();
        shell.vectors.get_mut(&0x1c4).unwrap().values=vec![5f32.to_bits(),8f32.to_bits()];
        shell.vectors.get_mut(&0x258).unwrap().values=vec![2f32.to_bits(),3f32.to_bits()];
        store_byte(&mut shell,0x23a,c["reset"].as_u64().unwrap()as u8);shell.words.insert(0x19c,9);store_byte(&mut shell,0x23b,0);
        shell.commander=Some(aa_combat::commander::State::with_maximum(&aa_combat::commander::Definition{id:99,abilities:Vec::new()},aa_body::F32(c["maximum"].as_u64().unwrap()as u32),1).unwrap());
        shell.commander.as_mut().unwrap().experience.balance=F32Bits(7f32.to_bits());shell.words.remove(&0x230);
        let before=&shell.commander.as_ref().unwrap().experience as*const _ as usize;
        let mut values=PrefixValues{base:Supplied{starting:10f32.to_bits(),maximum:50f32.to_bits(),reads:0,fail:None},experience:c["experience"].as_u64().unwrap()as u32,property:c["property"]==1,allowed:c["nation"]!=3,fail_experience:c["fail_experience"].as_bool().unwrap(),reads:0,events:Vec::new()};
        let result=start(&mut shell,&mut values,pc);let outcome=result.err().unwrap_or_else(||"ok".into());
        assert_eq!(outcome,expected["outcome"].as_str().unwrap(),"{row}");
        assert_eq!(serde_json::json!(shell.vectors[&0x1c4].values),expected["stock"],"{row}");
        assert_eq!(shell.commander.as_ref().unwrap().experience.balance.0,expected["experience"].as_u64().unwrap()as u32,"{row}");
        assert_eq!(&shell.commander.as_ref().unwrap().experience as*const _ as usize,before);
        assert_eq!(values.reads,expected["experience_reads"].as_u64().unwrap()as usize,"{row}");
        assert_eq!(serde_json::json!(values.events),expected["events"],"{row}");
        assert_eq!(shell.words[&0x19c],expected["status"].as_u64().unwrap()as u32,"{row}");
        assert_eq!(byte(&shell,0x23b).unwrap(),expected["started"].as_u64().unwrap()as u8,"{row}");
    }
}

#[test]
#[ignore="requires actual AA_GAME_ROOT v14 sources and sealed startup receipt"]
fn actual_source_factions_same_moved_bank_current_experience_and_property_state() {
    use aa_mapworld::restore::RestoreDefinitions;
    use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits}};
    let root=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,aa_rules::tgi::Limits::default()).unwrap());
    let proof=crate::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    crate::combat_source_completion::install(&catalog,&mut files,proof.context(),proof.csv(),proof.globals()).unwrap();
    crate::combat_source_completion::compute(&catalog).unwrap();catalog.native_load_mode=Some(1);
    let mut current=load_view_variables::State::reset_from_fresh_source(&catalog,load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(root.join("reconstruction/assets/original/Data/Maps/DeveloperMaps/(3v3)Firefight.RMP")).unwrap()).unwrap();
    let inventory=aa_mapworld::WorldInventory::decode(&map,u64::MAX).unwrap();
    let rules=crate::load_world_rules::Owner::load(&catalog,&inventory).unwrap();current.publish_loaded_rules(&catalog,&rules).unwrap();
    let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let factions=actors.native_data_registry("Faction").unwrap();
    let definitions=factions.entries.iter().map(|e|(e,catalog.resolve_commander(&e.ids.encode_utf16().collect::<Vec<_>>(),pc).unwrap().unwrap())).collect::<Vec<_>>();
    let mut store=crate::actor_world::ActorStore::new();
    store.property_allocations.bind_registered_sources(&catalog).unwrap();
    {
        let _shared=catalog.definition_bank.borrow();
        store.property_allocations.bind_registered_sources(&catalog).unwrap();
        store.property_allocations.require_catalog(&catalog).unwrap();
    }
    let mut bank=std::mem::take(&mut*catalog.definition_bank.borrow_mut());assert!(catalog.definition_bank.borrow().combat_source().is_err());
    let state_owner=&store.property_allocations as*const _ as usize;
    {
        let (properties,access)=store.split_property_state();
        properties.bind_in_bank(&catalog,&mut bank).unwrap();properties.require_in_bank(&catalog,&bank).unwrap();
        assert!(properties.require_catalog(&catalog).is_err());assert!(properties.instances().is_empty());
        assert_eq!(properties as*const _ as usize,state_owner);
        assert!(access.key(aa_actors::ActorId::from_raw(1)).is_none());
    }
    assert_eq!(&store.property_allocations as*const _ as usize,state_owner);
    let resources=actors.native_resource_registry(pc).unwrap().ids.len()as u32;let mut rows=Vec::new();
    for (entry,definition)in definitions {
        let source=actors.native_data_node(entry).unwrap();
        let raw=match source.field("properties",pc).unwrap().unwrap(){aa_rules::native::TypedValue::String{value,..}=>value,_=>panic!()};
        let list=crate::kingdom_start_source::properties(&catalog,&bank,definition.definition.id).unwrap();
        //This is an observed source census, never a substitute for a positive
        //grant. A changed/nonempty source must reach the actual grant adapter.
        assert!(list.is_empty(),"actual Faction {} has starting Property grants",entry.ids);
        let nation=actors.native_data_registry("Nation").unwrap().entries.iter().find_map(|n|{
            let node=actors.native_data_node(n).unwrap();
            let value=match node.field("factions",pc).unwrap().unwrap(){aa_rules::native::TypedValue::String{value,..}=>value,_=>panic!()};
            let named=aa_mapworld::restore::NamedDefinition{id:n.ids.clone(),index:None};
            (!aa_rules::abilities::required_property_tokens(&value).unwrap().is_empty()
                &&crate::kingdom_start_source::allowed(&catalog,&bank,Some(&named),Some(definition.definition.id)).unwrap()).then_some(named)
        });
        for live_pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
            let mut shell=SpecialShell::kingdom(aa_actors::ActorId::from_raw(1),resources).unwrap();
            shell.commander=Some(aa_combat::commander::State::with_maximum(&definition.definition,definition.maximum_experience,1).unwrap());shell.words.remove(&0x230);
            if let Some(nation)=&nation {shell.words.remove(&0x234);shell.references.insert(0x234,Some(nation.clone()));}
            let owner=&shell.commander.as_ref().unwrap().experience as*const _ as usize;
            start_existing(&catalog,&bank,&current,&mut shell,live_pc).unwrap();
            assert_eq!(&shell.commander.as_ref().unwrap().experience as*const _ as usize,owner);
            assert_eq!(shell.commander.as_ref().unwrap().experience.balance.0,current.starting_experience_bits());
            assert_eq!(shell.words[&0x19c],0);assert_eq!(byte(&shell,0x23b).unwrap(),1);
        }
        rows.push(serde_json::json!({"faction":entry.ids,"source":entry.source,"properties":raw,"property_tokens":list,"experience_max":definition.maximum_experience.0,"starting_experience":current.starting_experience_bits(),"nation":nation.map(|n|n.id)}));
    }
    assert!(!rows.is_empty());
    if let Some(path)=std::env::var_os("AA_KINGDOM_START_CAPTURE"){std::fs::write(path,serde_json::to_vec_pretty(&serde_json::json!({"source_precision":format!("{pc:?}"),"sources":actors.sources,"factions":rows})).unwrap()).unwrap();}
}
