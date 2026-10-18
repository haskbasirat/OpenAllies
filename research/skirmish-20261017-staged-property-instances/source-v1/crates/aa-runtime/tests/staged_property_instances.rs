//! Real saved-prefix boxes moved into ActorStore, then explicit property calls.
//! This does not claim a fully initialized selected mission or natural grant.
use aa_actors::components::ComponentSlot as Slot;
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
use aa_runtime::{actor_world::{ActorStore,ObjectKey,RuntimeComponent},catalog::Catalog,
    load_property_instances::State,localization::Localization,restoration::{Attempt,LoadBindings},
    restore_lifecycle::{ScopedManagedLoad,TaskKind},settings::Settings,
    staged_property_instances::{self as live,Context}};
use serde_json::{json,Value};
use std::sync::Arc;
fn props(store:&ActorStore,key:ObjectKey)->&aa_body::properties::PropertiesState{
    match store.object(key).unwrap().actor.as_ref().unwrap().components.get(Slot::Properties){Some(RuntimeComponent::Properties(v))=>v,_=>panic!()}
}
fn snapshot(store:&ActorStore,key:ObjectKey)->Value{
    let state=&store.property_allocations;
    json!({"properties":props(store,key),"instances":state.instances(),"constructors":state.constructors().iter().map(|(h,p)|json!({"handle":h.0,"definition":p.definition.0,"kind":p.kind,"source_id":p.source_id})).collect::<Vec<_>>()})
}
fn members(store:&ActorStore,key:ObjectKey)->Option<Vec<Value>>{
    match store.object(key).unwrap().actor.as_ref().unwrap().components.get(Slot::Organization){
        Some(RuntimeComponent::Organization(org))=>Some(org.members.iter().map(|id|{
            if id.is_null(){json!({"id":0,"properties":false,"experience":false})}else{
                let a=store.object(store.key(*id).unwrap()).unwrap().actor.as_ref().unwrap();
                json!({"id":id.raw(),"properties":a.components.present(Slot::Properties),"experience":a.components.present(Slot::Experience)})
            }
        }).collect()),_=>None,
    }
}
#[test]
#[ignore="requires AA_GAME_ROOT original/v14 sources; default immutable replay"]
fn actual_moved_property_owner_and_bank_preserve_insertion_prefix_all_pc(){
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let settings=Settings::load_v14(&mut files,Limits::default()).unwrap();
    let text=Localization::load_v14(&mut files,Limits::default()).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();
    let resources=Arc::new(ResourceCatalog::load_v14(&mut files).unwrap());
    let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap()).unwrap();
    let inactive=ScopedManagedLoad::from_sources(&mut files,0).unwrap().begin(TaskKind::Normal).unwrap();
    let attempt=Attempt::load_with_source_setup(&map,&actors,settings.clone(),text,Precision::Double53,u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
            let r=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut files)?;
            aa_runtime::combat_source_completion::install(catalog,&mut files,r.context(),r.csv(),r.globals())?;
            aa_runtime::combat_source_completion::compute(catalog)
        }).unwrap();
    let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);catalog.bind_resource_catalog(resources).unwrap();
    *catalog.definition_bank.borrow_mut()=attempt.definitions;
    let mut variables=aa_runtime::load_view_variables::State::reset_from_fresh_source(&catalog,aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let mut bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let mut store=ActorStore::from_restored_with_abilities(attempt.registry,attempt.abilities).unwrap();
    let clock=attempt.clock;
    store.property_allocations.bind_in_bank(&catalog,&mut bank).unwrap();
    assert!(catalog.definition_bank.borrow().combat_source().is_err(),"moved Bank must not be reinstalled");
    assert!(store.property_allocations.require_catalog(&catalog).is_err());
    store.property_allocations.require_in_bank(&catalog,&bank).unwrap();
    let global=store.property_allocations.definitions().keys().copied().find(|h|store.property_allocations.facts(*h).unwrap().id()=="mobile").unwrap();
    let infantry=store.property_allocations.definitions().keys().copied().find(|h|store.property_allocations.facts(*h).unwrap().id()=="infantry").unwrap();
    let mut selected=vec![];
    for performer in [false,true]{
        selected.extend(store.identities().iter().filter_map(|e|{
            let o=store.object(e.value).ok()?;let a=o.actor.as_ref()?;
            (a.components.present(Slot::Properties)&&!a.components.present(Slot::Organization)&&!a.components.present(Slot::Settlement)
                &&a.components.present(Slot::Performer)==performer).then_some(e.value)
        }).take(3));
    }
    let org=store.identities().iter().find_map(|e|{
        let o=store.object(e.value).ok()?;let a=o.actor.as_ref()?;
        (a.components.present(Slot::Properties)&&a.components.present(Slot::Organization)&&!a.components.present(Slot::Performer)).then_some(e.value)
    }).unwrap();selected.push(org);
    assert_eq!(selected.len(),7);
    let mut rows=vec![];
    for (ordinal,key) in selected.into_iter().enumerate(){
        let pc=[Precision::Single24,Precision::Double53,Precision::Extended64][ordinal%3];
        for (phase,definition) in [("success",global),("duplicate",global),("numeric",infantry)]{
            //The numeric probe is on Props-only current Kingdoms. Positive
            //numeric Actor children remain required and aren't synthesized.
            if phase=="numeric"&&store.object(key).unwrap().definition.native_class!=4{continue}
            let id=store.id(key).unwrap();let a=store.object(key).unwrap().actor.as_ref().unwrap();
            let reference=store.property_allocations.facts(definition).unwrap().reference();
            let input=json!({"owner":id.raw(),"id":store.object(key).unwrap().definition.proof.id,
                "native_class":store.object(key).unwrap().definition.native_class,"components":a.components.iter().map(|(s,_)|s.index()).collect::<Vec<_>>(),
                "organization":members(&store,key),"allegiance":a.words.get(&0xe8),"clock":clock.begin.0,"phase":phase,
                "property":definition.0,"kind":if phase=="duplicate"{7}else{0},"source_id":if phase=="duplicate"{id.raw()}else{0},
                "minimum_multiply":variables.modifier_clamp_multiply_bits(),"before":snapshot(&store,key),
                "definitions":store.property_allocations.definitions(),"pass_members":store.property_allocations.definitions().keys().map(|h|(h.0.to_string(),store.property_allocations.facts(*h).unwrap().pass_down_members())).collect::<std::collections::BTreeMap<_,_>>()});
            let p=props(&store,key)as*const _ as usize;
            let s=&store.property_allocations as*const State as usize;
            let result=live::add_on_owners(&mut Context{actors:&mut store,bank:&mut bank,variables:&mut variables,clock:&clock},&catalog,key,reference,
                if phase=="duplicate"{7}else{0},(phase=="duplicate").then_some(id),pc);
            let error=result.err();
            if phase=="duplicate"{assert!(error.is_none(),"{error:?}")}else if store.object(key).unwrap().actor.as_ref().unwrap().components.present(Slot::Performer){
                assert!(error.as_deref().unwrap().contains("57F4C0/57F1E0"));
            }else{assert!(error.is_none(),"{error:?}")}
            assert_eq!(props(&store,key)as*const _ as usize,p);assert_eq!(&store.property_allocations as*const State as usize,s);
            rows.push(json!({"precision":format!("{pc:?}"),"input":input,"output":snapshot(&store,key),"error":error}));
        }
    }
    let data=catalog.layout_data(Precision::Double53).unwrap();
    let initial=aa_rules::native::layout_data::native_actor_properties(&actors,&data,"gbr_corps",Precision::Double53).unwrap();
    let report=json!({"scope":"actual tutorial02 saved-prefix boxes moved once into ActorStore; direct grant probes, not initialized mission or natural grant","source_precision":"Double53","sources":actors.sources,
        "gbr_corps_initial_source_properties":initial,"catalog_bank_remains_empty":catalog.definition_bank.borrow().combat_source().is_err(),"rows":rows});
    if let Some(path)=std::env::var_os("AA_STAGED_PROPERTY_CAPTURE"){
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&report).unwrap();
    }else{
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-staged-property-instances");
        let expected:Value=serde_json::from_slice(&std::fs::read(root.join("actual-inputs.json")).unwrap()).unwrap();assert_eq!(report,expected);
        let native:Value=serde_json::from_slice(&std::fs::read(root.join("native-stored-properties.json")).unwrap()).unwrap();
        for (rust,native) in rows.iter().zip(native["rows"].as_array().unwrap()){
            assert_eq!(rust["output"],native["output"]["state"]);
        }
    }
}
