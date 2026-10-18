use super::*;
use crate::{localization::Localization, restoration::{Attempt, LoadBindings},
    restore_lifecycle::{ScopedManagedLoad, TaskKind}, settings::Settings};
use aa_mapworld::restore::RestoredRecord;
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog, resources::ResourceCatalog,
    sources::{SourceLimits, V14Sources}, tgi::Limits};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Arc};
use position::World;

fn evidence(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../research/skirmish-20261016-saved-organization-position").join(name)
}
fn outcome(result: &std::result::Result<(), String>) -> &'static str {
    match result {
        Ok(()) => "ok",
        Err(e) if e.contains("virtual+64") => "mind_64",
        Err(e) if e.contains("4EEB40") => "relocate",
        Err(e) => panic!("unexpected saved follow boundary: {e}"),
    }
}
fn capture(world: &Borrowed<'_, '_, '_>, member: ActorId, name: &str, scope: &str) -> Value {
    let selected = world.selected_member(world.context.owner).unwrap();
    let mut ids: Vec<_> = [Some(world.context.owner), Some(member), selected]
        .into_iter().flatten().filter(|id| !id.is_null()).collect();
    ids.sort_by_key(|id| id.raw()); ids.dedup();
    let actors: Vec<_> = ids.into_iter().filter_map(|id| {
        let actor = world.actor(id).ok()?;
        Some(json!({"id":id.raw(),"flags":actor.flags,
            "position":actor.position.map(|v|v.0),"mind":actor.mind.is_some()}))
    }).collect();
    json!({"name":name,"scope":scope,"world_mode":world.world_mode().unwrap(),
        "owner":world.context.owner.raw(),"member":member.raw(),
        "selected":selected.map(|id|id.raw()),
        "goal":world.context.organization.position_xy.map(|v|v.0),"actors":actors})
}
fn check(world: &mut Borrowed<'_, '_, '_>, member: ActorId, name: &str, scope: &str,
         oracle: Option<&Value>, captured: &mut Vec<Value>) {
    let input = capture(world, member, name, scope);
    let goal = world.context.organization.position_xy;
    let words = world.context.organization.words.clone();
    let members = world.context.organization.members.clone();
    let offsets = world.context.organization.offsets_78.clone();
    let rng = world.context.rng.state;
    let messages = world.context.native_messages.len();
    let result = follow_leader(world.catalog, OrganizationLeaderContext {
        owner: world.context.owner,
        owner_actor: world.context.owner_actor,
        owner_thing: world.context.owner_thing,
        organization: &mut *world.context.organization,
        registry: world.context.registry,
        rng: &mut *world.context.rng,
        native_messages: &mut *world.context.native_messages,
        source_ordinal: world.context.source_ordinal,
        precision: world.context.precision,
    }, member);
    let output = json!({"outcome":outcome(&result),
        "goal":world.context.organization.position_xy.map(|v|v.0)});
    if let Some(oracle) = oracle {
        let matched: Vec<_> = oracle["cases"].as_array().unwrap().iter()
            .filter(|row|row["input"]==input && row["precision"]==format!("{:?}",world.context.precision)).collect();
        assert_eq!(matched.len(),2,"both executable images: {input}");
        for row in matched { assert_eq!(row["output"],output,"{input}"); }
    }
    assert_eq!(world.context.organization.words,words);
    assert_eq!(world.context.organization.members,members);
    assert_eq!(world.context.organization.offsets_78,offsets);
    assert_eq!(world.context.rng.state,rng);
    assert_eq!(world.context.native_messages.len(),messages);
    if outcome(&result)!="relocate" { assert_eq!(world.context.organization.position_xy,goal); }
    captured.push(json!({"precision":format!("{:?}",world.context.precision),"input":input}));
}

#[test]
#[ignore="requires AA_GAME_ROOT original/v14 source data"]
fn actual_saved_follow_leader_and_explicit_storage_probes_match_native() {
    let root=PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut src=V14Sources::open(root.join("Data.rwd"),root.join("reconstruction/native-game/runtime-expanded-v14"),
        SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let settings=Settings::load_v14(&mut src,Limits::default()).unwrap();
    let text=Localization::load_v14(&mut src,Limits::default()).unwrap();
    let actors=ActorCatalog::load_v14(&mut src).unwrap();
    let resources=Arc::new(ResourceCatalog::load_v14(&mut src).unwrap());
    let token=ScopedManagedLoad::from_sources(&mut src,0).unwrap().begin(TaskKind::Normal).unwrap();
    let map=aa_mapworld::MapArchive::from_bytes(std::fs::read(root.join(
        "reconstruction/assets/original/Data/Maps/(2v2)Kursk.RMP")).unwrap()).unwrap();
    let mut attempt=Attempt::load_with_bindings(&map,&actors,settings,text,Precision::Double53,u64::MAX,
        aa_mapworld::restore::RestoreMode::InspectPartial,LoadBindings{native_load_mode:Some(1),inactive_load:Some(token),resources:Some(resources)}).unwrap();
    let mut catalog=Catalog::new(&actors);
    let ids:Vec<_>=attempt.registry.iter().filter_map(|entry| {
        let actor=entry.value.loaded()?.shell.actor.as_ref()?;
        match actor.components.get(ComponentSlot::Organization)? {
            RestoredComponent::Organization(org) if !actor.components.present(ComponentSlot::Mind)
                && org.members.iter().any(|id|!id.is_null())=>Some(entry.id), _=>None,
        }
    }).take(3).collect();
    assert_eq!(ids.len(),3);
    let capturing=std::env::var_os("AA_SAVED_POSITION_CAPTURE").is_some();
    let oracle:Option<Value>=(!capturing).then(||serde_json::from_slice(
        &std::fs::read(evidence("native-saved-position.json")).unwrap()).unwrap());
    let mut captured=vec![];
    for owner in ids {
        let record=std::mem::replace(attempt.registry.value_mut(owner).unwrap(),RestoredRecord::Loading);
        let RestoredRecord::Actor(mut entity)=record else {panic!("Actor")};
        let actor=entity.shell.actor.as_mut().unwrap();
        let Some(RestoredComponent::Organization(mut org))=actor.components.remove(ComponentSlot::Organization)
            else {panic!("Organization")};
        let original_goal=org.position_xy;
        let original_selected=org.words[&0x40];
        let original_flags=entity.shell.thing.flags;
        let notified=org.members.iter().copied().find(|id|!id.is_null()).unwrap();
        let address=org.as_ref() as *const _;
        for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
            for name in ["actual","nonselected","mode6","selected_probe","dual_flag30_probe"] {
                org.position_xy=original_goal; org.words.insert(0x40,original_selected);
                entity.shell.thing.flags=original_flags; catalog.native_load_mode=Some(if name=="mode6"{6}else{1});
                let member=if name=="nonselected"{ActorId::from_raw(0xdead0000)}else{notified};
                let mut member_flags=None;
                if matches!(name,"selected_probe"|"dual_flag30_probe") { org.words.insert(0x40,notified.raw()); }
                if name=="dual_flag30_probe" {
                    entity.shell.thing.flags|=1<<30;
                    let target=attempt.registry.value_mut(notified).unwrap().loaded_mut().unwrap();
                    member_flags=Some(target.shell.thing.flags); target.shell.thing.flags|=1<<30;
                }
                let mut rng=attempt.rng; let mut messages=vec![];
                let mut world=Borrowed{catalog:&catalog,context:OrganizationLeaderContext{
                    owner,owner_actor:actor,owner_thing:&entity.shell.thing,organization:&mut org,
                    registry:&attempt.registry,rng:&mut rng,native_messages:&mut messages,source_ordinal:434,precision:pc}};
                check(&mut world,member,&format!("kursk_{:x}_{name}",owner.raw()),
                    if name=="actual"{"actual Rust partial saved storage"}else{"explicit test substitution in actual saved allocations"},
                    oracle.as_ref(),&mut captured);
                if let Some(flags)=member_flags {attempt.registry.value_mut(notified).unwrap().loaded_mut().unwrap().shell.thing.flags=flags;}
            }
        }
        assert_eq!(org.as_ref() as *const _,address,"actual allocation retained");
        org.position_xy=original_goal;org.words.insert(0x40,original_selected);entity.shell.thing.flags=original_flags;
        actor.components.insert(ComponentSlot::Organization,RestoredComponent::Organization(org));
        *attempt.registry.value_mut(owner).unwrap()=RestoredRecord::Actor(entity);
    }
    assert_eq!(captured.len(),45);
    if capturing {std::fs::write(evidence("actual-saved-inputs.json"),serde_json::to_vec_pretty(&captured).unwrap()).unwrap();}
}

#[test]
fn guards_preserve_lazy_identity_reads_and_never_supply_a_default_world_mode() {
    use aa_actors::{IdentityRegistry, ground_states::{LayoutDefinition, OrganizationState},
        loading::{LoadKind, construct_shell_with_components}};
    let owner=ActorId::from_raw(0x10001);
    let missing=ActorId::from_raw(0x20002);
    let actors=ActorCatalog::from_files(vec![],Limits::default());
    let mut catalog=Catalog::new(&actors);
    let proof=aa_mapworld::bootstrap::DefinitionProof {
        id:"portable_guard_fixture".into(),
        source:aa_rules::resources::Provenance{path:"fixture".into(),sha256:String::new(),
            span:aa_rules::tgi::Span{start:0,end:0}},template_chain:vec![],
        category:*aa_rules::native::CATEGORIES.iter().find(|c|c.name=="ACTOR").unwrap(),
        base_flag_inputs:vec![],
    };
    let mut base=construct_shell_with_components::<_,RestoredComponent>(LoadKind::Actor,proof);
    let actor=base.actor.as_mut().unwrap();
    let mut org=OrganizationState::new(owner,LayoutDefinition{definition:"fixture".into(),parameters:vec![],slots:0},
        F32::ZERO,F32::ONE,[F32::ONE,F32::ZERO]).unwrap();
    let registry=IdentityRegistry::new();
    let mut rng=aa_simulation::NativeRng{state:0x12345678};
    let mut messages=vec![];
    macro_rules! run {($member:expr)=>{follow_leader(&catalog,OrganizationLeaderContext{
        owner,owner_actor:actor,owner_thing:&base.thing,organization:&mut org,registry:&registry,
        rng:&mut rng,native_messages:&mut messages,source_ordinal:0,precision:Precision::Double53},$member)}}
    let goal=org.position_xy;
    assert!(run!(missing).unwrap_err().contains("GWorld+D8 load mode is unavailable"));
    catalog.native_load_mode=Some(6);
    org.words.remove(&0x40);
    assert!(run!(missing).is_ok(),"mode6 does not read selected member");
    catalog.native_load_mode=Some(1);
    assert!(run!(missing).unwrap_err().contains("Organization+40 is uninitialized"));
    org.words.insert(0x40,missing.raw());
    assert!(run!(ActorId::from_raw(0x30003)).is_ok(),"nonselected unknown IDs remain unread");
    assert!(run!(missing).is_ok(),"no Mind returns before selected member position/flags");
    base.thing.flags|=1<<30;
    assert!(run!(missing).unwrap_err().contains("not a current restored Actor"));
    assert_eq!(org.position_xy,goal);
    base.thing.flags&=!(1<<30);
    actor.components.insert(ComponentSlot::Mind,RestoredComponent::Detection(Box::new(
        aa_actors::ground_states::DetectionState::new(owner,F32::ZERO))));
    assert!(run!(ActorId::from_raw(0x30003)).is_ok(),"nonselected skips malformed Mind too");
    assert!(run!(missing).unwrap_err().contains("required actual Mind slot"));
    catalog.native_load_mode=Some(6);
    assert!(run!(ActorId::NULL).is_ok());
    catalog.native_load_mode=Some(1);
    assert!(run!(ActorId::NULL).is_ok(),"null nonselected notification returns");
    org.words.insert(0x40,0);
    assert!(run!(ActorId::NULL).unwrap_err().contains("outside the bound saved caller domain"));
    assert_eq!(org.position_xy,goal);
    assert_eq!(rng.state,0x12345678);
    assert!(messages.is_empty());
}
