use super::*;
use serde_json::{Value,json};
use aa_actors::{components::StateBits,loading::LoadShell};
use aa_mapworld::bootstrap::DefinitionProof;
use std::sync::Arc;

fn activate(shell:&mut LoadShell<DefinitionProof,RestoredComponent>,shift:u32) {
    let cell=[0,1].map(|i|aa_numeric::Binary::from_f32_bits(shell.thing.position_bits[i]).unwrap().truncate_i32().unwrap()>>shift);
    shell.thing.flags&=!((1<<29)|(1<<12));
    let actor=shell.actor.as_mut().unwrap();
    actor.components.state.low&=!((1<<24)|(1<<25)|(1<<26));
    actor.components.state.high&=!0x14;
    actor.words.insert(0xF8,0);
    let object=shell.object.as_mut().unwrap();object.word_44=cell[0]as u32;object.word_48=cell[1]as u32;
}
type Before=(u32,StateBits,u32,u32,u32,usize);
fn before(shell:&LoadShell<DefinitionProof,RestoredComponent>)->Before {
    let actor=shell.actor.as_ref().unwrap();let object=shell.object.as_ref().unwrap();
    (shell.thing.flags,actor.components.state,*actor.words.get(&0xF8).unwrap(),object.word_44,object.word_48,actor as *const _ as usize)
}
fn restore(shell:&mut LoadShell<DefinitionProof,RestoredComponent>,b:Before) {
    shell.thing.flags=b.0;let a=shell.actor.as_mut().unwrap();a.components.state=b.1;a.words.insert(0xF8,b.2);
    assert_eq!(a as *const _ as usize,b.5);
    let o=shell.object.as_mut().unwrap();o.word_44=b.3;o.word_48=b.4;
}
#[test]
#[ignore="requires AA_GAME_ROOT and verified capture03"]
fn actual_organization_terrain_uses_same_held_parent_and_restores_failed_member() {
    use crate::{
        battle_terrain, combat_source_completion as completion,
        combat_source_receipt::{TargetFiles, VerifiedStartupReceipt},
        localization::Localization,
        restoration::{Attempt, LoadBindings},
        restore_lifecycle::{ScopedManagedLoad, TaskKind},
        settings::Settings,
    };
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::{
        actors::ActorCatalog, resources::ResourceCatalog,
        sources::{SourceLimits, V14Sources, sha256}, tgi::Limits,
    };
    let game = std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native = game.join("reconstruction/native-game/runtime-expanded-v14");
    let capture = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/development10/native-source-capture-03");
    let exe = std::fs::read(native.join("AA.exe")).unwrap();
    let dll = std::fs::read(native.join("D3D9.dll")).unwrap();
    let config = std::fs::read(native.join("dgVoodoo.conf")).unwrap();
    let receipt = std::fs::read(capture.join("capture.json")).unwrap();
    let manifest = std::fs::read(capture.join("runtime-files.json")).unwrap();
    let map_bytes = std::fs::read(game.join("reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP")).unwrap();
    let map = aa_mapworld::MapArchive::from_bytes(map_bytes.clone()).unwrap();
    let mut sources = V14Sources::open(game.join("Data.rwd"), &native,
        SourceLimits { max_member_bytes:64*1024*1024 }).unwrap();
    let settings = Settings::load_v14(&mut sources, Limits::default()).unwrap();
    let text = Localization::load_v14(&mut sources, Limits::default()).unwrap();
    let actors = ActorCatalog::load_v14(&mut sources).unwrap();
    let resources = Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let inactive = ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
    let mut attempt = Attempt::load_with_source_setup(&map, &actors, settings.clone(), text,
        Precision::Double53, u64::MAX, RestoreMode::InspectPartial,
        LoadBindings { native_load_mode:Some(1), inactive_load:Some(inactive), resources:Some(resources) },
        |catalog| {
            let proof = VerifiedStartupReceipt::verify(catalog, &mut sources, TargetFiles {
                native_exe:&exe, renderer_dll:&dll, renderer_config:&config,
                capture_receipt:&receipt, file_manifest_receipt:&manifest })?;
            completion::install(catalog, &mut sources, proof.context(), proof.csv(), proof.globals())?;
            completion::compute(catalog)
        }).unwrap();
    let terrain_source = battle_terrain::Source::bind(&mut sources,&actors,&settings,Precision::Double53).unwrap();
    let terrain = BattleTerrain::load(&map,&terrain_source).unwrap();
    let catalog = Catalog::new(&actors);
    *catalog.definition_bank.borrow_mut() = attempt.definitions;
    let shift = terrain.bound.layout.coordinate_shift;
    let parent=attempt.registry.iter().find_map(|entry| {
        let RestoredRecord::Actor(value)=&entry.value else{return None};
        let Some(RestoredComponent::Organization(org))=value.shell.actor.as_ref()?.components.get(ComponentSlot::Organization)else{return None};
        let mut ready=false;let mut count=0;
        for id in org.members.iter().copied().filter(|id|!id.is_null()) {
            let Some(RestoredRecord::Actor(child))=attempt.registry.get(id).map(|e|&e.value)else{return None};
            let Some(RestoredComponent::Element(element))=child.shell.actor.as_ref()?.components.get(ComponentSlot::Element)else{return None};
            if element.header.owner!=id{return None}
            count+=1;ready|=element.flag_1c&&element.organization==Some(entry.id);
        }
        (count>0&&ready).then_some(entry.id)
    }).unwrap_or_else(|| {
        let observed=attempt.registry.iter().filter_map(|entry| {
            let RestoredRecord::Actor(v)=&entry.value else{return None};
            let Some(RestoredComponent::Organization(org))=v.shell.actor.as_ref()?.components.get(ComponentSlot::Organization)else{return None};
            let members=org.members.iter().map(|id| {
                let value=attempt.registry.get(*id).map(|e|&e.value);
                let element=match value {Some(RestoredRecord::Actor(c))=>match c.shell.actor.as_ref().and_then(|a|a.components.get(ComponentSlot::Element)) {
                    Some(RestoredComponent::Element(e))=>json!({"owner":e.header.owner.raw(),"parent":e.organization.map(|v|v.raw()),"ready":e.flag_1c}),
                    other=>json!({"element_payload":format!("{:?}",other.map(|_|"wrong payload")),"failure":c.failure}),
                },other=>json!({"record":match other{Some(RestoredRecord::Loading)=>"Loading",Some(_)=>"other",None=>"missing"}})};
                json!({"id":id.raw(),"element":element})
            }).collect::<Vec<_>>();
            Some(json!({"owner":entry.id.raw(),"definition":v.definition.proof.id,"failure":v.failure,"members":members}))
        }).take(8).collect::<Vec<_>>();
        eprintln!("observed actual diagnostic Organizations: {}",json!(observed));
        panic!("actual loaded Organization with real ready member/parent relation required")
    });
    let mut held=std::mem::replace(attempt.registry.value_mut(parent).unwrap(),RestoredRecord::Loading);
    let RestoredRecord::Actor(value)=&mut held else{unreachable!()};
    let mut record=Record{owner:parent,shell:&mut value.shell,definition:&value.definition,registry:&mut attempt.registry};
    let ids=match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Organization).unwrap() {
        RestoredComponent::Organization(org)=>org.members.iter().copied().filter(|id|!id.is_null()).collect::<Vec<_>>(),_=>unreachable!()
    };
    let parent_before=before(record.shell);
    let mut saved=std::collections::BTreeMap::new();
    for id in &ids {
        let RestoredRecord::Actor(child)=&record.registry.get(*id).unwrap().value else{unreachable!()};
        saved.insert(*id,before(&child.shell));
    }
    let mut rows=vec![];
    for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
        activate(record.shell,shift);
        for id in &ids {
            let RestoredRecord::Actor(child)=record.registry.value_mut(*id).unwrap()else{unreachable!()};
            activate(&mut child.shell,shift);
        }
        let default_error=crate::load_actor_terrain_refresh::refresh(&catalog,&mut record,&terrain,pc,
            &mut crate::load_actor_terrain_refresh::RequiredEffects).unwrap_err();
        assert!(default_error.to_string().contains("579EB0"));
        assert_ne!(*record.shell.actor.as_ref().unwrap().words.get(&0xF8).unwrap(),0);
        for id in &ids {
            let RestoredRecord::Actor(child)=&record.registry.get(*id).unwrap().value else{unreachable!()};
            assert_eq!(*child.shell.actor.as_ref().unwrap().words.get(&0xF8).unwrap(),0);
        }
        activate(record.shell,shift);
        refresh(&catalog,&mut record,&terrain,pc).unwrap();
        let parent_f8=*record.shell.actor.as_ref().unwrap().words.get(&0xF8).unwrap();
        assert_ne!(parent_f8,0);
        let mut children=vec![];let mut inherited=0;
        for id in &ids {
            let RestoredRecord::Actor(child)=&record.registry.get(*id).unwrap().value else{panic!("member must return to its own registry entry")};
            let a=child.shell.actor.as_ref().unwrap();
            assert_eq!(a as *const _ as usize,saved[id].5);
            let Some(RestoredComponent::Element(element))=a.components.get(ComponentSlot::Element)else{unreachable!()};
            let f8=*a.words.get(&0xF8).unwrap();
            if element.flag_1c&&element.organization==Some(parent){assert_eq!(f8,parent_f8);inherited+=1}
            children.push(json!({"id":id.raw(),"f8":f8,"ready":element.flag_1c,"parent":element.organization.map(|v|v.raw())}));
        }
        assert!(inherited>0);
        rows.push(json!({"pc":pc,"parent_f8":parent_f8,"children":children,"same_allocations":true}));
        restore(record.shell,parent_before);
        for (id,b) in &saved {
            let RestoredRecord::Actor(child)=record.registry.value_mut(*id).unwrap()else{unreachable!()};
            restore(&mut child.shell,*b);
        }
    }
    // An actual non-null member with its Element explicitly removed must fail,
    // retain the preceding parent F8 store, and return that same child shell.
    let first=ids[0];
    let removed={let RestoredRecord::Actor(child)=record.registry.value_mut(first).unwrap()else{unreachable!()};
        child.shell.actor.as_mut().unwrap().components.remove(ComponentSlot::Element).unwrap()};
    activate(record.shell,shift);
    let error=refresh(&catalog,&mut record,&terrain,Precision::Double53).unwrap_err();
    assert!(error.to_string().contains("570550"));
    assert_ne!(*record.shell.actor.as_ref().unwrap().words.get(&0xF8).unwrap(),0);
    {let RestoredRecord::Actor(child)=record.registry.value_mut(first).unwrap()else{panic!("failed member not restored")};
        assert_eq!(child.shell.actor.as_ref().unwrap()as *const _ as usize,saved[&first].5);
        child.shell.actor.as_mut().unwrap().components.insert(ComponentSlot::Element,removed);}
    restore(record.shell,parent_before);
    let result=json!({"scope":"actual original Kursk Organization/member allocations/source relationships; explicit active gate/cached-cell setup; missing-Element fault injection; not untouched ActorInit or battle readiness",
        "map_sha256":sha256(&map_bytes),"source_precision":Precision::Double53,"parent":parent.raw(),"definition":record.definition.proof.id,"rows":rows,"missing_element_prefix_retained":true,"default_required_prefix_preserved":true});
    if let Some(path)=std::env::var_os("AA_ORG_TERRAIN_CAPTURE") {
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&result).unwrap();
    }else{
        let p=std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261016-organization-terrain-refresh/actual-organization-terrain.json");
        let expected:Value=serde_json::from_slice(&std::fs::read(p).unwrap()).unwrap();assert_eq!(result,expected);
    }
    *attempt.registry.value_mut(parent).unwrap()=held;
}
