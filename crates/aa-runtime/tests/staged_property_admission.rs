//! Actual moved saved-prefix owners, explicit read-only eligibility probes.
use aa_actors::{ActorId,components::ComponentSlot as Slot};
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
use aa_runtime::{actor_world::{ActorStore,ObjectKey,RuntimeComponent},catalog::Catalog,definitions::Bank,
    localization::Localization,restoration::{Attempt,LoadBindings},restore_lifecycle::{ScopedManagedLoad,TaskKind},
    settings::Settings,staged_property_admission,staged_property_instances::{self,Context}};
use serde_json::{json,Value};
use std::{collections::BTreeMap,sync::Arc};
fn projection(catalog:&Catalog<'_>,actors:&ActorStore,root:ActorId,property:aa_body::credit::Object,pc:Precision)->Value{
    fn visit(catalog:&Catalog<'_>,actors:&ActorStore,id:ActorId,pc:Precision,
        nodes:&mut BTreeMap<String,Value>,definitions:&mut BTreeMap<String,Value>){
        if nodes.contains_key(&id.raw().to_string()){return}
        let a=actors.object(actors.key(id).unwrap()).unwrap().actor.as_ref().unwrap();
        let properties=match a.components.get(Slot::Properties){Some(RuntimeComponent::Properties(p))=>Some(p),None=>None,_=>panic!()};
        let available=properties.map(|p|p.effects.iter().map(|h|{
            let v=&actors.property_allocations.instances()[h];json!({"definition":v.definition.0,"disabled":v.disabled})
        }).collect::<Vec<_>>()).unwrap_or_default();
        let organization=match a.components.get(Slot::Organization){
            Some(RuntimeComponent::Organization(org))=>{
                let data=catalog.layout_data(pc).unwrap();let layout=data.find_layout(&org.layout.definition).unwrap();
                for name in org.layout.parameters.iter().flatten(){
                    let binding=catalog.actor_bindings(name,pc).unwrap();
                    let properties=aa_rules::native::layout_data::native_actor_properties(catalog.actors,&data,name,pc).unwrap();
                    definitions.insert(name.clone(),json!({"mask":binding.definition.component_mask.unwrap(),
                        "available":properties.into_iter().map(|p|actors.property_allocations.definition_for_reference(p).unwrap().0).collect::<Vec<_>>()}));
                }
                Some(json!({"members":org.members.iter().map(|v|v.raw()).collect::<Vec<_>>(),"groups":layout.slot_groups,"parameters":org.layout.parameters}))
            },None=>None,_=>panic!(),
        };
        nodes.insert(id.raw().to_string(),json!({"properties":properties.is_some(),"available":available,"organization":organization}));
        if let Some(RuntimeComponent::Organization(org))=a.components.get(Slot::Organization){for member in &org.members{if !member.is_null(){visit(catalog,actors,*member,pc,nodes,definitions)}}}
    }
    let mut nodes=BTreeMap::new();let mut definitions=BTreeMap::new();visit(catalog,actors,root,pc,&mut nodes,&mut definitions);
    json!({"root":root.raw(),"actors":nodes,"definitions":definitions,
        "requires":actors.property_allocations.facts(property).unwrap().required().iter().map(|v|v.0).collect::<Vec<_>>()})
}
#[test]
#[ignore="requires AA_GAME_ROOT original/v14 sources; default is immutable replay"]
fn actual_stored_applicability_uses_current_instances_and_vacant_source(){
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let settings=Settings::load_v14(&mut files,Limits::default()).unwrap();let text=Localization::load_v14(&mut files,Limits::default()).unwrap();
    let source=ActorCatalog::load_v14(&mut files).unwrap();let resources=Arc::new(ResourceCatalog::load_v14(&mut files).unwrap());
    let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap()).unwrap();
    let inactive=ScopedManagedLoad::from_sources(&mut files,0).unwrap().begin(TaskKind::Normal).unwrap();
    let attempt=Attempt::load_with_source_setup(&map,&source,settings.clone(),text,Precision::Double53,u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
            let proof=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut files)?;
            aa_runtime::combat_source_completion::install(catalog,&mut files,proof.context(),proof.csv(),proof.globals())?;
            aa_runtime::combat_source_completion::compute(catalog)
        }).unwrap();
    let mut catalog=Catalog::new(&source);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);catalog.bind_resource_catalog(resources).unwrap();
    *catalog.definition_bank.borrow_mut()=attempt.definitions;
    let mut variables=aa_runtime::load_view_variables::State::reset_from_fresh_source(&catalog,aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let mut bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let mut actors=ActorStore::from_restored_with_abilities(attempt.registry,attempt.abilities).unwrap();
    actors.property_allocations.bind_in_bank(&catalog,&mut bank).unwrap();
    let token=|actors:&ActorStore,name:&str|actors.property_allocations.definitions().keys().copied().find(|p|actors.property_allocations.facts(*p).unwrap().id()==name).unwrap();
    let mobile=token(&actors,"mobile");let entrench=token(&actors,"infantry_entrench");let infantry=token(&actors,"infantry");
    let kingdom=actors.identities().iter().find_map(|e|(actors.object(e.value).ok()?.definition.native_class==4).then_some(e.value)).unwrap();
    let org=actors.identities().iter().find_map(|e|actors.object(e.value).ok()?.actor.as_ref()?.components.present(Slot::Organization).then_some(e.value)).unwrap();
    let ordinary=actors.identities().iter().find_map(|e|{
        let o=actors.object(e.value).ok()?;let a=o.actor.as_ref()?;
        (o.definition.native_class==3&&!a.components.present(Slot::Organization)&&a.components.present(Slot::Properties)).then_some(e.value)
    }).unwrap();
    let mut rows=vec![];
    let mut check=|actors:&mut ActorStore,bank:&mut Bank,key:ObjectKey,property,phase:&str|{
        let id=actors.id(key).unwrap();let reference=actors.property_allocations.facts(property).unwrap().reference();
        let input=projection(&catalog,actors,id,property,Precision::Double53);
        let count=actors.property_allocations.instances().len();let address=&actors.property_allocations as*const _ as usize;
        let result=staged_property_admission::applicable_on_owners(&catalog,actors,bank,key,reference).unwrap();
        assert_eq!(actors.property_allocations.instances().len(),count);assert_eq!(&actors.property_allocations as*const _ as usize,address);
        rows.push(json!({"phase":phase,"input":input,"result":result}));result
    };
    for key in [kingdom,ordinary,org]{assert!(check(&mut actors,&mut bank,key,mobile,"empty_requirements"));check(&mut actors,&mut bank,key,entrench,"actual_requirement");}
    assert!(!check(&mut actors,&mut bank,kingdom,entrench,"missing_dependency"));
    let reference=actors.property_allocations.facts(infantry).unwrap().reference();
    for _ in 0..2{staged_property_instances::add_on_owners(&mut Context{actors:&mut actors,bank:&mut bank,variables:&mut variables,clock:&attempt.clock},&catalog,kingdom,reference,0,None,Precision::Double53).unwrap();}
    assert!(check(&mut actors,&mut bank,kingdom,entrench,"added_dependency"));
    //Explicit same-instance disabled mutation probe, not a natural save claim.
    for instance in actors.property_allocations.instances_mut().values_mut(){if instance.definition==infantry{instance.disabled=true}}
    assert!(check(&mut actors,&mut bank,kingdom,entrench,"disabled_dependencies_still_match"));
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    let report=json!({"scope":"actual moved tutorial02 saved-prefix state; direct predicate and explicit disabled-instance probes; not initialized mission","rows":rows});
    if let Some(path)=std::env::var_os("AA_STORED_ADMISSION_CAPTURE"){
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&report).unwrap();
    }else{
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-staged-property-instances");
        let expected:Value=serde_json::from_slice(&std::fs::read(root.join("admission-actual-inputs.json")).unwrap()).unwrap();assert_eq!(report,expected);
        let native:Value=serde_json::from_slice(&std::fs::read(root.join("native-stored-admission.json")).unwrap()).unwrap();
        for (rust,native) in rows.iter().zip(native["rows"].as_array().unwrap()){assert_eq!(rust["result"],native["result"]);}
    }
}

fn list_values(actors:&ActorStore,key:ObjectKey)->(Vec<String>,usize,bool,bool){
    let list=&actors.object(key).unwrap().actor.as_ref().unwrap().actor_list_108;
    let mut values=vec![];let mut node=list.head();let mut visited=std::collections::BTreeSet::new();
    while let Some(id)=node{assert!(visited.insert(id));values.push(list.value(id).unwrap().clone());node=list.next(id).unwrap();}
    (values,list.allocated_nodes(),list.head().is_some(),list.tail().is_some())
}
#[test]
#[ignore="requires AA_GAME_ROOT original/v14 sources; direct constructor probe, not initialized mission"]
fn actual_builder_constructor_nodes_remain_same_owner_across_refresh_errors(){
    use aa_actors::creation::{ActorCreationRecord,CreationRecord,ThingDefinition};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let settings=Settings::load_v14(&mut files,Limits::default()).unwrap();let text=Localization::load_v14(&mut files,Limits::default()).unwrap();
    let source=ActorCatalog::load_v14(&mut files).unwrap();let resources=Arc::new(ResourceCatalog::load_v14(&mut files).unwrap());
    let mut catalog=Catalog::new(&source);catalog.settings=Some(settings);catalog.definition_text=Some(text.clone());*catalog.localization.borrow_mut()=Some(text);catalog.bind_resource_catalog(resources).unwrap();
    let receipt=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    let startup=aa_runtime::actor_creation_startup::StartupInputs::from_verified_startup_reference(&receipt).unwrap();
    aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
    aa_runtime::combat_source_completion::compute(&catalog).unwrap();
    let bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    let mut actors=ActorStore::new();let mut rng=aa_simulation::NativeRng::new(0x12345678);let mut rows=vec![];
    let data=catalog.native_builder_data(Precision::Double53).unwrap();
    for name in ["gbr_engineer","gbr_corps"]{
        let binding=catalog.actor_bindings(name,bank.combat_source().unwrap().source_precision()).unwrap();
        let record=ActorCreationRecord{base:CreationRecord{definition:ThingDefinition{key:name.into(),flags:binding.definition.flags},position_bits:[0;3],direction_bits:[0,(-1f32).to_bits()],name_utf16:vec![],saved_id:ActorId::NULL},variant:0,owner_word:0,word_30:0,request_nonblocking:false};
        let key=actors.construct_new_bound(&catalog,binding.clone(),record,0,&mut rng,startup.startup_precision(),Precision::Double53).unwrap();
        let id=actors.initialize_object(key,&mut rng,Precision::Double53).unwrap();
        if name=="gbr_engineer"{assert!(binding.definition.component_mask.unwrap()&(1<<18)!=0);actors.construct_ground(key,Slot::Builder,aa_body::F32::ZERO,Precision::Double53).unwrap();}
        let expected_source=data.definition(name).unwrap().ids.clone();
        if name=="gbr_engineer"{assert!(!expected_source.is_empty());}
        for phase in if name=="gbr_engineer"{vec!["first","repeat","missing_source_bank"]}else{vec!["absent_builder_missing_bank"]}{
            let a=actors.object(key).unwrap().actor.as_ref().unwrap();let list_address=&a.actor_list_108 as*const _ as usize;
            let input=json!({"operation":"refresh","actor":0,"actors":[{"list":list_values(&actors,key).0,
                "source":expected_source,"builder":a.components.present(Slot::Builder),"flags":binding.definition.component_mask.unwrap(),"members":null,"element":false,"parent":null}]});
            let missing=Bank::default();let source_bank=if phase.contains("missing"){&missing}else{&bank};
            let error=aa_runtime::stored_actor_properties::refresh(&catalog,&mut actors,source_bank,key).err();
            if phase=="missing_source_bank"{assert!(error.as_deref().unwrap().contains("combat source State is not installed"));}
            else{assert!(error.is_none(),"{error:?}")}
            let a=actors.object(key).unwrap().actor.as_ref().unwrap();assert_eq!(&a.actor_list_108 as*const _ as usize,list_address);
            let (values,allocated,head,tail)=list_values(&actors,key);
            assert_eq!(values.len(),allocated);assert_eq!(head,!values.is_empty());assert_eq!(tail,!values.is_empty());
            if name=="gbr_engineer"&&!phase.contains("missing"){assert_eq!(values,expected_source.iter().rev().cloned().collect::<Vec<_>>());}
            else{assert!(values.is_empty());}
            rows.push(json!({"phase":phase,"owner":id.raw(),"definition":name,"input":input,"output":{"values":values,"allocated_nodes":allocated,"head":head,"tail":tail},"error":error}));
        }
    }
    let report=json!({"scope":"actual gbr_engineer source/constructor+Builder box and gbr_corps constructor; direct4D4CF0 probes, no InitCreate/initialized mission claim","source_precision":"Double53","startup_precision":startup.startup_precision(),"rows":rows});
    if let Some(path)=std::env::var_os("AA_STORED_ACTOR_PROPERTIES_CAPTURE"){
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&report).unwrap();
    }else{
        let root=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-stored-actor-properties");
        let expected:Value=serde_json::from_slice(&std::fs::read(root.join("actual-constructor-inputs.json")).unwrap()).unwrap();assert_eq!(report,expected);
        let native:Value=serde_json::from_slice(&std::fs::read(root.join("native-stored-actor-properties.json")).unwrap()).unwrap();
        for (rust,native) in rows.iter().zip(native["rows"].as_array().unwrap()){assert_eq!(rust["output"],native["output"]);}
    }
}
