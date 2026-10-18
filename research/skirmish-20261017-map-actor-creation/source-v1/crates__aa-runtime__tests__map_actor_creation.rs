use aa_actors::{ActorId,creation::{ActorCreationRecord,CreationRecord,ThingDefinition}};
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
use aa_runtime::{actor_world::{ActorStore,Source},catalog::Catalog,localization::Localization,settings::Settings};
use aa_simulation::NativeRng;
use serde_json::{json,Value};
use std::sync::Arc;

fn snapshot(store:&ActorStore,key:aa_runtime::actor_world::ObjectKey,rng:&NativeRng)->Value {
    let object=store.object(key).unwrap();let p=store.staged_placement(key).unwrap();
    let o=object.object.as_ref().unwrap();let a=object.actor.as_ref().unwrap();
    let Source::New{record,..}=&store.entity(key).unwrap().source else{panic!()};
    json!({"flags":p.flags,"position":p.position_bits,"facing":p.facing_bits,
        "time":object.thing.word_1c,"word_c":object.thing.word_c,"word_10":object.thing.word_10,
        "name":object.thing.name_utf16,"scale":o.scale_bits,"variant":o.variant,"byte4c":o.byte_4c,
        "words50":o.words_50_to_54,"owner":a.words[&0xe8],"required":a.words[&0x158],
        "state":[a.components.state.low,a.components.state.high],"actions":[a.words[&0xf0],a.words[&0xf4]],
        "components":a.components.iter().count(),"list108":a.actor_list_108.head().is_some(),
        "zones":a.zones_148.zones().slots.len(),"id":store.id(key).map(|v|v.raw()),
        "params_direction":record.base.direction_bits,"params_owner":record.owner_word,"rng":rng.state})
}

#[test]
#[ignore="actual source constructor laboratory, requires AA_GAME_ROOT"]
fn actual_source_bound_constructor_keeps_completion_and_live_precision_separate() {
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let settings=Settings::load_v14(&mut files,Limits::default()).unwrap();
    let text=Localization::load_v14(&mut files,Limits::default()).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();
    let resources=Arc::new(ResourceCatalog::load_v14(&mut files).unwrap());
    let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings);catalog.definition_text=Some(text.clone());*catalog.localization.borrow_mut()=Some(text);catalog.bind_resource_catalog(resources).unwrap();
    let mut rows=vec![];let mut layouts=vec![];let mut defenders=vec![];let mut actions=vec![];
    for name in ["gbr_corps","regiment_gbr_hq_infantry","gbr_truck_corps"] {
        let binding=catalog.actor_bindings(name,Precision::Double53).unwrap();
        if let Some(org)=&binding.definition.ground.organization {
            let data=catalog.layout_data(Precision::Double53).unwrap();
            let layout=data.layouts.iter().find(|l|l.ids==org.layout.definition).unwrap();
            let selected=aa_runtime::map_actor_creation::first_defined_slot(layout.primary_slot,&layout.slot_groups,&org.layout.parameters).unwrap();
            layouts.push(json!({"actor":name,"layout":layout.ids,"primary":layout.primary_slot,"groups":layout.slot_groups,"parameters":org.layout.parameters,"selected":selected,"mask":binding.definition.component_mask}));
        }
        for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
            for direction in [[0,(-1f32).to_bits()],[2f32.to_bits(),0]] {
                //Explicit unowned/direct constructor probe: this does not claim
                //the codec-only replacement queue is an initialized mission.
                let record=ActorCreationRecord{base:CreationRecord{definition:ThingDefinition{key:name.into(),flags:0xdeadbeef},position_bits:[1109393408,1129840640,1086590007],direction_bits:direction,name_utf16:vec![],saved_id:ActorId::NULL},variant:0,owner_word:0,word_30:0,request_nonblocking:false};
                let mut store=ActorStore::new();let mut rng=NativeRng::new(0x12345678);
                let forged=Arc::new(binding.as_ref().clone());
                assert!(store.construct_new_bound(&catalog,forged,record.clone(),0,&mut rng,Precision::Single24,pc).unwrap_err().to_string().contains("actual source Catalog allocation"));
                assert_eq!(rng.state,0x12345678);assert_eq!(store.identities().len(),0);
                let key=store.construct_new_bound(&catalog,Arc::clone(&binding),record,0,&mut rng,Precision::Single24,pc).unwrap();
                let before=snapshot(&store,key,&rng);
                let id=store.initialize_object(key,&mut rng,pc).unwrap();assert_eq!(id.raw(),1);
                let after=snapshot(&store,key,&rng);
                let object=binding.definition.object.as_ref().unwrap();
                rows.push(json!({"precision":pc,"input":{"id":name,"source_precision":"Double53","direction_table_precision":"Single24","direction":direction,"position":[1109393408u32,1129840640u32,1086590007u32],"flags":binding.definition.flags,"variations":object.variations_enabled,"weights":object.variation_weights_bits,"total_weight":object.total_variation_weight_bits,"scale":[object.scale_min_bits,object.scale_max_bits],"variant":0,"seed":0x12345678u32,"owner":0,"required":0,"clock":0},"before":before,"after":after}));
                if binding.definition.component_mask.unwrap()&(1<<15)!=0 {
                    store.construct_ground(key,aa_actors::components::ComponentSlot::Construction,aa_body::F32::ZERO,pc).unwrap();
                    //Explicit fresh contributor-entry state. The native caller
                    //sets normal27 before assembly; this probes its source bind.
                    store.object_mut(key).unwrap().components_mut().unwrap().state.low=1<<27;
                    let f=actors.native_construction_flags(name,Precision::Double53).unwrap().unwrap();
                    let allow=catalog.settings.as_ref().unwrap().allow_sell_under_siege().unwrap().0;
                    let before=snapshot(&store,key,&rng);
                    store.rebuild_source_construction_actions(&catalog,id).unwrap();
                    let after=snapshot(&store,key,&rng);
                    actions.push(json!({"precision":pc,"input":{"actor":name,"source_precision":"Double53","flags":before["flags"],"state":before["state"],"actions":before["actions"],"owner":0,"object_byte4c":0,"thing_wordc":0,"can_sell":f.can_sell,"can_raze":f.can_raze,"upgrade_count":f.upgrade_count,"allow_sell_under_siege":allow},"actions":after["actions"]}));
                    let foreign=Catalog::new(&actors);
                    assert!(store.rebuild_source_construction_actions(&foreign,id).unwrap_err().to_string()
                        .contains("Construction actual source Catalog allocation"));
                    //4D1350 clears both current action words before the lazy
                    //source bind refuses a foreign allocation. No rollback.
                    let actual=&store.object(key).unwrap().actor.as_ref().unwrap().words;
                    assert_eq!([actual[&0xf0],actual[&0xf4]],[0,0]);
                    store.rebuild_source_construction_actions(&catalog,id).unwrap();
                }
                if binding.definition.component_mask.unwrap()&(1<<20)!=0 {
                    let loaded=actors.native_loaded_defender(name,Precision::Double53,binding.character.as_ref().map(|v|v as &dyn aa_rules::abilities::AnimationResolver)).unwrap().unwrap();
                    let mut bank=aa_runtime::definitions::Bank::default();
                    store.construct_defender(key,&catalog,&mut bank).unwrap();
                    let aa_runtime::actor_world::RuntimeComponent::Defender(d)=store.component(id,aa_actors::components::ComponentSlot::Defender).unwrap().unwrap()else{panic!()};
                    let instances=d.abilities.iter().map(|h|{let a=&store.abilities[h];json!({"handle":h,"definition":a.definition,"local":a.local_definition,"use_count":a.use_count,"last_use":a.last_use.0,"word14":a.word14,"enabled":a.enabled,"recharge_bypass":a.recharge_bypass,"fuzzy_range":a.fuzzy_range.0,"power":a.power.0,"summoned":a.summoned_ids,"word28":a.word28})}).collect::<Vec<_>>();
                    defenders.push(json!({"precision":pc,"input":{"actor":name,"source_precision":"Double53","owner":id.raw(),"powers":loaded.abilities().iter().map(|a|a.power_bits).collect::<Vec<_>>(),"definitions":instances.iter().map(|a|a["definition"].clone()).collect::<Vec<_>>(),"locals":instances.iter().map(|a|a["local"].clone()).collect::<Vec<_>>()},"state":{"owner":d.header.owner.raw(),"scheduled":d.header.scheduled_head,"abilities":instances,"word1c":d.word_1c,"word20":d.word_20,"entries":d.entries,"word2c":d.word_2c,"byte30":d.byte_30}}));
                }
            }
        }
    }
    let doc=json!({"scope":"actual source, explicit unowned constructor laboratory; not an initialized Staged mission or successful organization creation","rows":rows,"layouts":layouts,"defenders":defenders,"construction_actions":actions});
    if let Some(path)=std::env::var_os("AA_CREATION_CONSTRUCTOR_CAPTURE") {
        use std::io::Write;let mut f=std::fs::OpenOptions::new().create_new(true).write(true).open(path).unwrap();f.write_all(serde_json::to_string_pretty(&doc).unwrap().as_bytes()).unwrap();
    } else {
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-map-actor-creation");
        let expected:Value=serde_json::from_slice(&std::fs::read(root.join("actual-constructor-inputs.json")).unwrap()).unwrap();assert_eq!(doc,expected);
        let native:Value=serde_json::from_slice(&std::fs::read(root.join("native-constructor.json")).unwrap()).unwrap();
        assert_eq!(doc["rows"],native["rows"]);
        let native:Value=serde_json::from_slice(&std::fs::read(root.join("native-defender.json")).unwrap()).unwrap();
        assert_eq!(doc["defenders"],native["rows"]);
        let native:Value=serde_json::from_slice(&std::fs::read(root.join("native-construction-actions.json")).unwrap()).unwrap();
        assert_eq!(doc["construction_actions"],native["rows"]);
    }
}

#[test]
fn native_first_slot_primary_fallback_and_empty_case_order() {
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-map-actor-creation");
    let d:Value=serde_json::from_slice(&std::fs::read(root.join("native-layout-slots.json")).unwrap()).unwrap();
    for row in d["rows"].as_array().unwrap(){
        let c=&row["input"];
        let groups:Vec<u32>=serde_json::from_value(c["groups"].clone()).unwrap();
        let parameters:Vec<Option<String>>=serde_json::from_value(c["parameters"].clone()).unwrap();
        assert_eq!(json!(aa_runtime::map_actor_creation::first_defined_slot(c["primary"].as_i64().unwrap()as i32,&groups,&parameters).unwrap()),row["selected"]);
    }
}

#[test]
fn native_component_requirements_include_current_structure_owner_state() {
    use aa_actors::{components::{ComponentSlot,StateBits},init_create::FactoryKind,leaf_init};
    let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-map-actor-creation");
    let d:Value=serde_json::from_slice(&std::fs::read(root.join("native-requirements.json")).unwrap()).unwrap();
    for row in d["rows"].as_array().unwrap(){
        let c=&row["input"];let slot=ComponentSlot::from_index(c["slot"].as_u64().unwrap()as usize).unwrap();
        let kind=match c["kind"].as_str().unwrap(){"standard"=>FactoryKind::Standard(slot),"organization"=>FactoryKind::OrganizationMind,"element"=>FactoryKind::ElementMind,"aircraft"=>FactoryKind::AircraftMind,_=>panic!()};
        let state=Some(StateBits{low:c["state"][0].as_u64().unwrap()as u32,high:c["state"][1].as_u64().unwrap()as u32});
        let value=leaf_init::ground_requirement_with_structure_state(kind,state).unwrap();
        assert_eq!(json!(value.map(|s|s as u8)),row["required"]);
    }
    assert!(leaf_init::ground_requirement_with_structure_state(FactoryKind::Standard(ComponentSlot::Structure),None).is_err());
}
