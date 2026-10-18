use crate::{catalog::Catalog,live_object_init::Record};
use aa_actors::components::ComponentSlot;
use aa_mapworld::restore::{RestoredRecord,RestoredComponent};
use aa_numeric::Precision;
use serde_json::{Value,json};

fn check_organization(catalog:&Catalog<'_>,registry:&mut aa_actors::IdentityRegistry<RestoredRecord>,source_pc:Precision)->Value {
    let id=registry.iter().find_map(|entry| {
        let RestoredRecord::Actor(v)=&entry.value else{return None};
        let Some(RestoredComponent::Organization(org))=v.shell.actor.as_ref()?.components.get(ComponentSlot::Organization)else{return None};
        (!org.members.is_empty()).then_some(entry.id)
    }).expect("real tutorial02 nonempty Organization");
    let mut held=std::mem::replace(registry.value_mut(id).unwrap(),RestoredRecord::Loading);
    let RestoredRecord::Actor(v)=&mut held else{unreachable!()};
    let mut record=Record{owner:id,shell:&mut v.shell,definition:&v.definition,registry:&mut *registry};
    let (address,old,input)={
        let Some(RestoredComponent::Organization(org))=record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Organization)else{unreachable!()};
        let info=aa_mapworld::restore::RestoreDefinitions::organization_layout_info(catalog,&org.layout.definition,source_pc).unwrap();
        let mut members=vec![];
        for member in &org.members {
            if member.is_null(){members.push(Value::Null);continue}
            let actor=if *member==id{record.shell.actor.as_ref().unwrap()}else{record.registry.get(*member).unwrap().value.loaded().unwrap().shell.actor.as_ref().unwrap()};
            let performer=match actor.components.get(ComponentSlot::Performer) {
                None=>Value::Null,
                Some(RestoredComponent::Performer(p))=> {
                    assert_eq!(p.state.owner,member.raw());
                    let body=match actor.components.get(ComponentSlot::Body){None=>Value::Null,Some(RestoredComponent::Body(b))=>{assert_eq!(b.header.owner,*member);json!([b.body.health.0,b.body.maximum.0])},_=>panic!("Body payload")};
                    json!({"owner":p.state.owner,"value":p.word60,"body":body})
                },_=>panic!("Performer payload"),
            };
            members.push(json!({"id":member.raw(),"performer":performer}));
        }
        let parameters=org.layout.parameters.iter().map(|p|p.as_ref().map(|target|{
            let flags=catalog.canonical_component_mask(target,source_pc).unwrap();
            let value=if flags&0x20!=0{catalog.completed_actor_combat_values(target).unwrap().value_1e4.0}else{0};
            json!({"id":target,"flags":flags,"value":value})
        })).collect::<Vec<_>>();
        (org.as_ref()as *const _ as usize,[org.words[&0x68],org.words[&0x6C]],json!({"name":"actual-immediate-wrapper-tutorial02","owner":id.raw(),"layout":org.layout.definition,"groups":info.slot_groups,"parameters":parameters,"members":members,"current":org.words[&0x68],"maximum":org.words[&0x6C]}))
    };
    let mut rows=vec![];
    for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
        crate::load_organization_value_refresh::refresh(catalog,&mut record,pc).unwrap();
        let Some(RestoredComponent::Organization(org))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Organization)else{unreachable!()};
        assert_eq!(org.as_ref()as *const _ as usize,address);
        rows.push(json!({"precision":format!("{pc:?}"),"input":input,"output":{"outcome":"ok","current":org.words[&0x68],"maximum":org.words[&0x6C]}}));
        org.words.insert(0x68,old[0]);org.words.insert(0x6C,old[1]);
    }
    //Explicit malformed layout probe is caught after native reset stores;
    //it does not rewrite saved relationships or supply an empty parameter.
    let old_definition={let Some(RestoredComponent::Organization(org))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Organization)else{unreachable!()};std::mem::replace(&mut org.layout.definition,"missing_layout_for_fault_probe".into())};
    let member_before={let Some(RestoredComponent::Organization(org))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Organization)else{unreachable!()};let value=org.members[0];org.members[0]=aa_actors::ActorId::NULL;value};
    let error=crate::load_organization_value_refresh::refresh(catalog,&mut record,Precision::Double53).unwrap_err();
    {let Some(RestoredComponent::Organization(org))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Organization)else{unreachable!()};
        assert_eq!(org.as_ref()as *const _ as usize,address);assert_eq!([org.words[&0x68],org.words[&0x6C]],[0,0]);org.layout.definition=old_definition;org.members[0]=member_before;org.words.insert(0x68,old[0]);org.words.insert(0x6C,old[1]);}
    let result=json!({"owner":id.raw(),"definition":record.definition.proof.id,"rows":rows,"same_allocation":true,"error_prefix":error.to_string()});
    drop(record);
    *registry.value_mut(id).unwrap()=held;
    result
}
#[test]
#[ignore="requires AA_GAME_ROOT and verified capture03"]
fn actual_tutorial_denizen_values_use_same_held_allocation_and_completed_sources() {
    use crate::{combat_source_completion as completion,combat_source_receipt::{TargetFiles,VerifiedStartupReceipt},
        localization::Localization,restoration::{Attempt,LoadBindings},restore_lifecycle::{ScopedManagedLoad,TaskKind},settings::Settings};
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{SourceLimits,V14Sources,sha256},tgi::Limits};
    use std::sync::Arc;
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native=game.join("reconstruction/native-game/runtime-expanded-v14");
    let capture=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/development10/native-source-capture-03");
    let exe=std::fs::read(native.join("AA.exe")).unwrap();let dll=std::fs::read(native.join("D3D9.dll")).unwrap();
    let config=std::fs::read(native.join("dgVoodoo.conf")).unwrap();let receipt=std::fs::read(capture.join("capture.json")).unwrap();
    let manifest=std::fs::read(capture.join("runtime-files.json")).unwrap();
    let map_bytes=std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap();
    let map=aa_mapworld::MapArchive::from_bytes(map_bytes.clone()).unwrap();
    let mut sources=V14Sources::open(game.join("Data.rwd"),&native,SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();let resources=Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
    let mut attempt=Attempt::load_with_source_setup(&map,&actors,settings,text,Precision::Double53,u64::MAX,RestoreMode::InspectPartial,
        LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources)},|catalog|{
            let proof=VerifiedStartupReceipt::verify(catalog,&mut sources,TargetFiles{native_exe:&exe,renderer_dll:&dll,renderer_config:&config,capture_receipt:&receipt,file_manifest_receipt:&manifest})?;
            completion::install(catalog,&mut sources,proof.context(),proof.csv(),proof.globals())?;completion::compute(catalog)
        }).unwrap();
    let catalog=Catalog::new(&actors);*catalog.definition_bank.borrow_mut()=attempt.definitions;
    let id=attempt.registry.iter().find_map(|entry|{
        let RestoredRecord::Actor(v)=&entry.value else{return None};
        (v.definition.proof.id=="usa_airfield"&&v.shell.actor.as_ref()?.components.present(ComponentSlot::Denizen)).then_some(entry.id)
    }).expect("actual tutorial02 usa_airfield Denizen");
    let mut held=std::mem::replace(attempt.registry.value_mut(id).unwrap(),RestoredRecord::Loading);
    let RestoredRecord::Actor(v)=&mut held else{unreachable!()};
    let mut record=Record{owner:id,shell:&mut v.shell,definition:&v.definition,registry:&mut attempt.registry};
    let source_pc=crate::live_object_init::Source::bind(&catalog,record.definition).unwrap().source_precision();
    let (address,old,slots)={
        let Some(RestoredComponent::Denizen(d))=record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Denizen)else{unreachable!()};
        let mut slots=vec![];
        for s in &d.inventory {
            //Real tutorial payload has null occupants. Do not fabricate a
            //successful fallback for a currently occupied slot in this capture.
            assert_eq!(s.word_00,0);
            let flags=catalog.canonical_component_mask(&s.target,source_pc).unwrap();
            let health=if flags&1!=0{catalog.actors.native_ground_definition(&s.target,source_pc).unwrap().attributes.body.unwrap().health.bits}else{0};
            let value=if flags&0x20!=0{catalog.completed_actor_combat_values(&s.target).unwrap().value_1e4.0}else{0};
            slots.push(json!({"occupant":Value::Null,"target":{"flags":flags,"health":health,"value":value},"slot_value":s.value_08.0}));
        }
        (d.as_ref()as *const _ as usize,[d.words[&0x10],d.words[&0x14]],slots)
    };
    assert_eq!(slots.len(),6);
    let mut rows=vec![];
    for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
        crate::load_denizen_value_refresh::refresh(&catalog,&mut record,pc).unwrap();
        let Some(RestoredComponent::Denizen(d))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Denizen)else{unreachable!()};
        assert_eq!(d.as_ref()as *const _ as usize,address);
        rows.push(json!({"precision":format!("{pc:?}"),"input":{"slots":slots,"old_current":old[0],"old_maximum":old[1]},"output":{"current":d.words[&0x10],"maximum":d.words[&0x14]}}));
        d.words.insert(0x10,old[0]);d.words.insert(0x14,old[1]);
    }
    //Failure after the native resets must retain them in that same Box.
    let original_target={let Some(RestoredComponent::Denizen(d))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Denizen)else{unreachable!()};std::mem::replace(&mut d.inventory[0].target,"missing_actual_definition_for_fault_probe".into())};
    let error=crate::load_denizen_value_refresh::refresh(&catalog,&mut record,Precision::Double53).unwrap_err();
    {let Some(RestoredComponent::Denizen(d))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Denizen)else{unreachable!()};
        assert_eq!(d.as_ref()as *const _ as usize,address);assert_eq!([d.words[&0x10],d.words[&0x14]],[0,0]);d.inventory[0].target=original_target;d.words.insert(0x10,old[0]);d.words.insert(0x14,old[1]);}
    let definition=record.definition.proof.id.clone();
    drop(record);
    *attempt.registry.value_mut(id).unwrap()=held;
    let organization=check_organization(&catalog,&mut attempt.registry,source_pc);
    let result=json!({"scope":"Actual diagnostic tutorial02 usa_airfield allocation, real six saved inventory slots, Organization allocation/member values and completed Bank source; refresh-only all3PC plus explicit source/layout faults, not full Init or battle readiness",
        "owner":id.raw(),"definition":definition,"source_precision":source_pc,"map_sha256":sha256(&map_bytes),"rows":rows,"same_allocation":true,"error_prefix":error.to_string(),"organization":organization});
    if let Some(path)=std::env::var_os("AA_DENIZEN_VALUE_CAPTURE") {
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&result).unwrap();
    }else{
        let p=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-actor-value-refresh/actual-denizen-value.json");
        let expected:Value=serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();assert_eq!(result,expected);
    }
}
