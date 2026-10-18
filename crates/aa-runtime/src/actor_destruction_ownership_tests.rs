//!Storage-only fixtures: no real map/source/Actor lifecycle certificate is
//!minted. The native retirement rows independently supply expected raw fields.
use super::*;
use serde_json::Value;
fn add(store:&mut ActorStore,id:ActorId)->ObjectKey {
    let proof=DefinitionProof {id:"explicit destruction storage fixture".into(),
        source:aa_rules::resources::Provenance {path:"laboratory only".into(),sha256:String::new(),span:aa_rules::tgi::Span {start:0,end:0}},
        template_chain:vec![],category:aa_rules::native::CategoryDescriptor {name:"fixture",registry_index:2,native_kind:2,runtime_class:3,view_type:3},base_flag_inputs:vec![]};
    let definition=RuntimeDefinition {proof:proof.clone(),native_class:3,flags:0,object:None,component_mask:None,ground:Default::default()};
    let shell=aa_actors::loading::construct_shell_with_components::<_,RuntimeComponent>(LoadKind::Actor,proof.clone());
    let record=ActorCreationRecord {base:creation::CreationRecord {definition:creation::ThingDefinition {key:proof,flags:0},position_bits:[0;3],direction_bits:[0;2],name_utf16:vec![],saved_id:id},variant:0,owner_word:0,word_30:0,request_nonblocking:false};
    let key=store.allocate_key(Entity {runtime:Some(RuntimeObject::from_shell(shell,definition)),
        source:Source::New {record:Box::new(record),phase:NewPhase::ObjectInitialized,failure:None},bindings:None}).unwrap();
    store.identities.seed_saved_generation(id).unwrap();
    store.identities.register_saved(id,true,key).unwrap();key
}
#[test]
fn same_allocation_detaches_before_native_identity_retirement() {
    let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-map-replacement-drain/native-detach-destructor.json")).unwrap();
    let mut checked=0;
    for row in doc["rows"].as_array().unwrap().iter().filter(|r|r["kind"]=="detach"&&r["input"]["retain"]==0&&r["input"].get("cutoff").is_none()) {
        let pc=match row["precision"].as_str().unwrap() {"Single"=>Precision::Single24,"Double"=>Precision::Double53,"Extended"=>Precision::Extended64,_=>panic!()};
        let mut store=ActorStore::new();add(&mut store,ActorId::from_raw(0x10005));
        let id=ActorId::from_raw(0x10007);let key=add(&mut store,id);
        let address=store.entity(key).unwrap() as *const Entity;
        let mut spatial=crate::spatial_world::SpatialWorld::laboratory_destruction_member(key,id,row["input"]["flags"].as_u64().unwrap()as u32,pc);
        store.object_mut(key).unwrap().placement=PlacementOwner::Spatial;
        let old=store.placement(key,&spatial).unwrap();
        assert!(store.retire_identity(key).is_err()); //must not lose the sole pose owner
        assert!(spatial.detach_for_destruction(key,ActorId::from_raw(0x20007),pc).is_err());
        assert_eq!(store.placement(key,&spatial).unwrap(),old);
        store.detach_spatial_for_destruction(key,&mut spatial,pc).unwrap();
        assert!(spatial.key(key).is_err());assert!(spatial.index().resolve_reference(id.raw()).is_none());
        let detached=store.staged_placement(key).unwrap();assert_eq!(detached.position_bits,old.position_bits);
        assert_eq!(detached.flags,row["output"]["flags"].as_u64().unwrap()as u32);
        assert_eq!(store.key(id),Some(key));
        store.retire_identity(key).unwrap();
        assert!(store.key(id).is_none());assert!(store.id(key).is_none());
        assert_eq!(store.native_identity_word(key).unwrap().raw(),row["output"]["raw_id"].as_u64().unwrap()as u32);
        assert_eq!(store.identities().len(),row["output"]["active"].as_u64().unwrap()as usize);
        assert_eq!(store.identities().generation_at(7),row["output"]["generation"].as_u64().unwrap()as u16);
        assert!(store.retire_identity(key).is_err());
        assert_eq!(store.entity(key).unwrap() as *const Entity,address); //no E0 completion, no drop
        checked+=1;
    }
    assert_eq!(checked,18);
}
#[test]
fn exact_empty_actor_destructor_and_positive_entry_boundaries() {
    let doc:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-map-replacement-drain/native-detach-destructor.json")).unwrap();
    let mut checked=0;
    for row in doc["rows"].as_array().unwrap().iter().filter(|r|r["kind"]=="destructor"&&!r["input"]["view"].as_bool().unwrap_or(false)) {
        let mut store=ActorStore::new();let key=add(&mut store,ActorId::from_raw(0x10007));
        let runtime=store.object_mut(key).unwrap();
        let actor=runtime.actor.as_mut().unwrap();
        actor.words.insert(0xe8,row["input"]["owner"].as_u64().unwrap()as u32);
        actor.words.insert(0x154,u32::from(row["input"]["sai_actor"].as_bool().unwrap_or(false)));
        for off in [0x130,0x134,0x128,0x12c,0x120,0x124,0x118,0x11c] {actor.words.insert(off,0);}
        actor.string_dc_utf16="owned DC".encode_utf16().collect();actor.string_e0_utf16="owned E0".encode_utf16().collect();
        let map=aa_mind::sai::manager::SaiMapLifecycle {analysed:row["input"]["analysed"]==1,dynamic:row["input"]["dynamic"]==1,..Default::default()};
        let result=crate::map_replacement_drain::actor_destructor_fields(runtime,&map);
        match row["output"]["error"].as_str() {
            None=>{result.unwrap();assert!(runtime.actor.as_ref().unwrap().string_dc_utf16.is_empty());},
            Some("map_notify")=>assert!(result.unwrap_err().contains("4282A0")),
            Some("sai_actor")=>assert!(result.unwrap_err().contains("SAIActor154")),
            other=>panic!("{other:?}"),
        }
        let zones=&runtime.actor.as_ref().unwrap().zones_148;
        assert_eq!(serde_json::json!([u32::from(!zones.is_empty()),zones.len(),zones.native_capacity()]),row["output"]["zone_words"]);
        checked+=1;
    }
    assert_eq!(checked,54);
}
