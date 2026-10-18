//! Actual saved-prefix allocations moved once; direct4D1350 calls only.
//! These are not fully initialized selected-mission or fresh-HQ lifecycle tests.
use aa_actors::components::ComponentSlot as Slot;
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
use aa_runtime::{actor_world::{ActorStore,ObjectKey,RuntimeComponent,Spatial,PlacementState},catalog::Catalog,
    localization::Localization,restoration::{Attempt,LoadBindings},restore_lifecycle::{ScopedManagedLoad,TaskKind},settings::Settings};
use serde_json::{json,Value};
use std::sync::Arc;
struct NoPublished;
impl Spatial for NoPublished{
    fn placement(&self,_:aa_actors::ActorId)->aa_runtime::actor_world::Result<PlacementState>{panic!("direct saved-prefix actors remain staged")}
    fn publish(&mut self,_:aa_actors::ActorId,_:PlacementState)->aa_runtime::actor_world::Result<()>{panic!("4D1350 cannot publish placement")}
}
fn words(store:&ActorStore,key:ObjectKey)->[u32;2]{let a=store.object(key).unwrap().actor.as_ref().unwrap();[a.words[&0xf0],a.words[&0xf4]]}
fn outcome(result:Result<(),String>)->String{match result{
    Ok(())=>"ok".into(),Err(e)if e.contains("5827B0")=>"recruit_required".into(),
    Err(e)if e.contains("Mind same-arena")=>"mind_required".into(),
    Err(e)if e.contains("76C7F0")=>"owner_views_required".into(),Err(e)if e.contains("renderer link")=>"renderer_required".into(),
    Err(e)=>panic!("unclassified actual4D1350 boundary: {e}"),
}}

#[test]
#[ignore="requires AA_GAME_ROOT actual source and original/v14 oracle; direct saved-prefix scope"]
fn actual_stored_command_masks_use_same_components_moved_bank_and_native_prefix_all_pc(){
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let settings=Settings::load_v14(&mut files,Limits::default()).unwrap();let text=Localization::load_v14(&mut files,Limits::default()).unwrap();
    let source=ActorCatalog::load_v14(&mut files).unwrap();let resources=Arc::new(ResourceCatalog::load_v14(&mut files).unwrap());
    let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap()).unwrap();
    let inactive=ScopedManagedLoad::from_sources(&mut files,0).unwrap().begin(TaskKind::Normal).unwrap();
    let attempt=Attempt::load_with_source_setup(&map,&source,settings.clone(),text,Precision::Double53,u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
            let r=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut files)?;
            aa_runtime::combat_source_completion::install(catalog,&mut files,r.context(),r.csv(),r.globals())?;
            aa_runtime::combat_source_completion::compute(catalog)
        }).unwrap();
    let mut catalog=Catalog::new(&source);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);catalog.bind_resource_catalog(resources).unwrap();
    let mut bank=attempt.definitions;
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    let inventory=attempt.inventory;
    let mut store=ActorStore::from_restored_with_abilities(attempt.registry,attempt.abilities).unwrap();
    let mut selected:Vec<_>=store.identities().iter().filter_map(|e|{
        let o=store.object(e.value).ok()?;let a=o.actor.as_ref()?;
        (o.definition.native_class==3&&a.components.present(Slot::Construction)
            &&!a.components.present(Slot::Organization))
            .then_some(e.value)
    }).collect();
    selected.sort_by_key(|k|match store.object(*k).unwrap().definition.proof.id.as_str(){"usa_corps"=>0,"ger_bunker"=>1,_=>2});
    selected.truncate(3);
    assert!(!selected.is_empty(),"real tutorial02 ordinary Construction actors");
    assert_eq!(store.object(selected[0]).unwrap().definition.proof.id,"usa_corps");
    let state_types:Vec<_>=aa_actors::state::DESCRIPTORS.iter().map(|d|
        aa_rules::native::recruit_data::actor_state_type(&source,d.name,Precision::Double53).unwrap()).collect();
    let mut rows=vec![];
    for key in selected{
        let id=store.id(key).unwrap();
        let object_ptr=store.object(key).unwrap()as*const _ as usize;
        let component_ptrs:Vec<_>=store.object(key).unwrap().actor.as_ref().unwrap().components.iter().map(|(s,v)|(s.index(),v as*const _ as usize)).collect();
        //Invalid binding is refused before the coordinator's mandatory clear.
        let before=words(&store,key);
        let old=store.object(key).unwrap().definition.flags;
        store.object_mut(key).unwrap().definition.flags^=1;
        assert!(aa_runtime::stored_actor_actions::rebuild(&catalog,&mut store,&mut bank,key,&inventory,&NoPublished,Precision::Single24).unwrap_err().contains("proof differs"));
        assert_eq!(words(&store,key),before);store.object_mut(key).unwrap().definition.flags=old;
        for pc in [Precision::Single24,Precision::Double53,Precision::Extended64]{
            //Explicit incoming action words are native fixture inputs. Actual
            //state, source, components and ownership are left as loaded.
            store.set_action_bits(id,0x123456789a).unwrap();
            let o=store.object(key).unwrap();let a=o.actor.as_ref().unwrap();let d=&o.definition;
            let mut input=json!({"id":id.raw(),"source":d.proof.id,"flags":store.staged_placement(key).unwrap().flags,
                "state":[a.components.state.low,a.components.state.high],"actions":words(&store,key),
                "owner":a.words[&0xe8],"object4c":o.object.as_ref().unwrap().byte_4c,"renderer":o.thing.word_c!=0,
                "components":a.components.iter().map(|(s,_)|s.index()).collect::<Vec<_>>(),"types":state_types,
                "properties_flags":null,"structure":null,"recruit":null,"research":null,"denizen":null,
                "element":null,"mind":null,"mobile":null,"body":null,"abilities":null,
                "word108":a.actor_list_108.head().is_some(),"word118":a.words[&0x118],
                "construction":source.native_construction_flags(&d.proof.id,Precision::Double53).unwrap(),
                "allow_sell_under_siege":catalog.settings.as_ref().unwrap().allow_sell_under_siege().unwrap().0});
            for (slot,c)in a.components.iter(){match c{
                RuntimeComponent::Body(v)=>input["body"]=json!([v.body.health.0,v.body.maximum.0]),
                RuntimeComponent::Element(v)=>input["element"]=json!({"definition":v.element_definition.is_some(),"organization":v.organization.map(|v|v.raw()),"ready":v.flag_1c}),
                RuntimeComponent::Mind(v)=>input["mind"]=json!({"kind":format!("{:?}",v.kind)}),
                RuntimeComponent::Performer(v)=>{
                    let mut abilities=vec![];
                    for h in &v.state.abilities{
                        let current=&store.abilities[h];
                        if !current.enabled{abilities.push(json!({"enabled":false,"flags":0,"required_targets":0,"effect_kinds":[]}));continue}
                        let entry=bank.entry(current.local_definition.unwrap()).unwrap();
                        let (actor,index)=entry.key.name.rsplit_once("/ability/").unwrap();
                        let b=catalog.actor_bindings(actor,Precision::Double53).unwrap();
                        let loaded=&b.performer.as_ref().unwrap().abilities()[index.parse::<usize>().unwrap()];
                        abilities.push(json!({"enabled":true,"flags":loaded.flags,"required_targets":loaded.targets.required,"effect_kinds":loaded.effects.iter().map(|e|e.kind).collect::<Vec<_>>()}));
                    }
                    input["abilities"]=json!(abilities);
                }
                RuntimeComponent::MobileDeployment(_)=>{
                    let mobile=catalog.native_mobile_deployment_data(Precision::Double53).unwrap();let m=mobile.definition(&d.proof.id).unwrap();
                    input["mobile"]=json!({"pack":m.pack_to.is_some(),"unpack":m.unpack_to.is_some(),"health_min":m.pack_health_min_bits});
                }
                RuntimeComponent::Properties(v)=>input["properties_flags"]=json!(v.modifier.flags),
                RuntimeComponent::Structure(v)=>input["structure"]=json!({"parent":v.object_handle_10,"queue":v.queue_14.len()}),
                RuntimeComponent::Recruit(v)=>{
                    let targets=aa_rules::native::recruit_data::actor_recruit_targets(&source,&d.proof.id).unwrap().ids;
                    let data=catalog.layout_data(Precision::Double53).unwrap();
                    let entries:Vec<_>=targets.into_iter().map(|target|json!({"id":target,
                        "location":aa_rules::native::recruit_data::actor_transport_class(&source,&target).unwrap(),
                        "layout":data.organization_component(&source,&target,Precision::Double53).unwrap().is_some()})).collect();
                    input["recruit"]=json!({"byte10":v.byte_10,"byte11":v.byte_11,"entries":entries});
                }
                RuntimeComponent::Research(_)=>input["research"]=json!({"head":!d.ground.research_items.as_ref().unwrap().is_empty()}),
                RuntimeComponent::Denizen(v)=>input["denizen"]=json!({"byte20":v.byte_20,"byte21":v.byte_21,"flags38":v.words[&0x38],"flags3c":v.words[&0x3c]}),
                _=>{let _=slot;},
            }}
            let result=outcome(aa_runtime::stored_actor_actions::rebuild(&catalog,&mut store,&mut bank,key,&inventory,&NoPublished,pc));
            let expected_words=words(&store,key);
            //Synchronous nested Mind callbacks already hold these owners.
            //Reborrow the same Arena and ActorAccess; do not move out/replace
            //the active Mind or build a second ActorStore around its fields.
            store.set_action_bits(id,0x123456789a).unwrap();
            let arena_ptr=&store.minds as*const _ as usize;
            let ability_ptr=&store.abilities as*const _ as usize;
            {
                let (arena,mut access)=store.split_minds();
                assert_eq!(arena as*const _ as usize,arena_ptr);
                assert_eq!(&*access.abilities as*const _ as usize,ability_ptr);
                let from_split=outcome(aa_runtime::stored_actor_actions::rebuild_on_owners(
                    &catalog,&mut access,arena,&mut bank,key,&inventory,&NoPublished,None,pc));
                assert_eq!(from_split,result);
                let actor=access.object(key).unwrap().actor.as_ref().unwrap();
                assert_eq!([actor.words[&0xf0],actor.words[&0xf4]],expected_words);
                assert_eq!(access.object(key).unwrap()as*const _ as usize,object_ptr);
                assert_eq!(arena as*const _ as usize,arena_ptr);
            }
            assert_eq!(store.object(key).unwrap()as*const _ as usize,object_ptr);
            assert_eq!(store.object(key).unwrap().actor.as_ref().unwrap().components.iter().map(|(s,v)|(s.index(),v as*const _ as usize)).collect::<Vec<_>>(),component_ptrs);
            rows.push(json!({"precision":format!("{pc:?}"),"input":input,"output":{"outcome":result,"actions":words(&store,key)}}));
        }
    }
    assert!(catalog.definition_bank.borrow().combat_source().is_err(),"same moved Bank was never reinstalled");
    let report=json!({"scope":"actual tutorial02 saved-prefix allocations moved once; direct4D1350, not initialized mission or natural fresh grant","source_precision":"Double53","sources":source.sources,"rows":rows});
    if let Some(path)=std::env::var_os("AA_STORED_ACTIONS_CAPTURE"){
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&report).unwrap();
    }else{
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-stored-actions");
        let expected:Value=serde_json::from_slice(&std::fs::read(root.join("actual-inputs-v3.json")).unwrap()).unwrap();assert_eq!(report,expected);
        let native:Value=serde_json::from_slice(&std::fs::read(root.join("native-stored-actions-v3.json")).unwrap()).unwrap();
        for(r,n)in rows.iter().zip(native["rows"].as_array().unwrap()){
            assert_eq!(r["output"],n["output"],"{}",r["input"]["source"]);
        }
    }
}
