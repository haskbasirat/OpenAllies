//! External integration test: the real client current-thread sampler remains
//! outside aa-runtime's forbid(unsafe_code) library. Input readers below only
//! observe source-backed fixture inputs; production refresh performs its own
//! full owner/source/identity guards.
use aa_runtime::{catalog::Catalog,live_object_init::Record,load_combat_variables::Field,
    load_performer_value_refresh::{refresh,Context}};
use aa_actors::components::ComponentSlot;
use aa_body::{F32,credit::Object};
use aa_combat::combat_value::AbilityLocal;
use aa_mapworld::restore::RestoredComponent;
use aa_numeric::Precision;
use serde_json::{Value,json};
use aa_mapworld::restore::RestoredRecord;
#[path="../../aa-client/src/runtime_fp.rs"]
mod actual_fp;

use aa_actors::actor_creation::Error;
fn error(e:impl std::fmt::Display)->Error {Error::World(e.to_string())}
fn unavailable_environment()->Result<aa_numeric::crt_pow::CrtEnvironment,String>{Err("actual current-thread CRT service is unbound".into())}
fn local_ability(catalog:&Catalog<'_>,handle:u32,source_pc:Precision)->std::result::Result<AbilityLocal,aa_actors::actor_creation::Error> {
    let (local_name,definition,local_token)={
        let arena=catalog.ability_arena.try_borrow().map_err(error)?;let ability=arena.get(handle).map_err(error)?;
        let token=ability.local_definition.ok_or(Error::NullDereference("57E430 Ability+8 local definition"))?;
        let bank=catalog.definition_bank.try_borrow().map_err(error)?;let entry=bank.entry(token).map_err(error)?;
        if entry.key.domain!="actorlocalability" {return Err(Error::Unsupported("57E430 local Ability token belongs to another manager"))}
        (entry.key.name.clone(),ability.definition,token)
    };
    let (id,index,defender)=if let Some((id,index))=local_name.rsplit_once("/ability/"){(id,index,false)}
        else if let Some((id,index))=local_name.rsplit_once("/defender/"){(id,index,true)}
        else{return Err(Error::Unsupported("57E430 actual local Ability source identity producer unavailable"))};
    let index=index.parse::<usize>().map_err(error)?;
    let binding=catalog.actor_bindings(id,source_pc).map_err(error)?;
    let defender_source;
    let loaded=if defender {
        defender_source=catalog.actors.native_loaded_defender(id,source_pc,binding.character.as_ref().map(|c|c as &dyn aa_rules::abilities::AnimationResolver)).map_err(error)?;
        defender_source.as_ref().ok_or(Error::Unsupported("57E430 Defender local source missing"))?.abilities().get(index)
    }else {binding.performer.as_ref().ok_or(Error::Unsupported("57E430 Performer local source missing"))?.abilities().get(index)}
        .ok_or(Error::Unsupported("57E430 local Ability source ordinal missing"))?;
    let bank=catalog.definition_bank.try_borrow().map_err(error)?;
    if bank.token("ActorLocalAbility",&local_name)!=Some(local_token){return Err(Error::Unsupported("57E430 actual local Ability source token differs"))}
    let expected=match &loaded.authored.shared_definition_id {
        None=>bank.token("ActorLocalAbilityDefinition",&local_name),
        Some(aa_rules::native::TypedValue::String{value,..})=>bank.token("AbilityDefinition",value),
        _=>return Err(Error::Unsupported("57E430 shared Ability source type differs")),
    };
    if expected!=Some(definition){return Err(Error::Unsupported("57E430 actual Ability shared-definition token differs"))}
    let mut local=aa_runtime::combat_value_source::ability_input(loaded).map_err(error)?;
    if let Some(bits)=bank.entry(definition).map_err(error)?.range_override_bits {
        //The same Bank's explicit constructor/faction range write, not an
        //authored fallback. Other unbound shared field writers still refuse.
        local.definition.range_24=F32(bits);
    }
    Ok(local)
}

fn local_json(a:AbilityLocal)->Value {
    json!({"definition":{"flags_54":a.definition.flags_54,"range_24":a.definition.range_24.0,"area_38":a.definition.area_38.0,"projectile_3c":a.definition.projectile_3c,"power_64":a.definition.power_64.0},
        "local":{"value_04":a.value_04.0,"recharge_18":a.recharge_18.0,"first_entry_10":a.first_entry_10.map(|v|v.0)}})
}
fn capture(catalog:&Catalog<'_>,record:&Record<'_>,variables:&aa_runtime::load_view_variables::State)->Value {
    let source_pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let actor=record.shell.actor.as_ref().unwrap();
    let Some(RestoredComponent::Performer(p))=actor.components.get(ComponentSlot::Performer)else{unreachable!()};
    let table=|handles:&[u32]|handles.iter().map(|id| {
        let enabled=catalog.ability_arena.borrow().get(*id).unwrap().enabled;
        json!({"handle":id,"enabled":enabled,"source":local_json(local_ability(catalog,*id,source_pc).unwrap())})
    }).collect::<Vec<_>>();
    let body=match actor.components.get(ComponentSlot::Body){None=>Value::Null,Some(RestoredComponent::Body(b))=>json!([b.body.defense.0,b.body.maximum.0]),_=>unreachable!()};
    let speed=match actor.components.get(ComponentSlot::Mover){None=>Value::Null,Some(RestoredComponent::Mover(m))=>json!(m.speed.0),_=>unreachable!()};
    let (property_values,modifier)=match actor.components.get(ComponentSlot::Properties){None=>(Value::Null,Value::Null),Some(RestoredComponent::Properties(p))=>{
        assert!(p.effects.is_empty(),"this actual source capture requires real empty effects, never a fabricated arena");
        (json!([]),json!(p.modifier.scalars.map(|v|v.0)))},_=>unreachable!()};
    let secondary=match actor.components.get(ComponentSlot::Defender){None=>Value::Null,Some(RestoredComponent::Defender(d))=>json!(table(&d.abilities)),_=>unreachable!()};
    let binding=catalog.actor_bindings(&record.definition.proof.id,source_pc).unwrap();
    let bias=binding.performer.as_ref().unwrap().authored.fields["cv_bias"].fixed_bits().unwrap();
    let bank=catalog.definition_bank.borrow();let manager=bank.combat_source().unwrap();let means=manager.completed_means().unwrap();
    let env=actual_fp::sample().unwrap();
    json!({"owner":record.owner.raw(),"definition":record.definition.proof.id,"source_precision":format!("{source_pc:?}"),
        "property_values":property_values,"modifier":modifier,"body":body,"speed":speed,"bonus":bias,
        "abilities":table(&p.state.abilities),"secondary":secondary,"before":p.word60,
        "current_words":aa_runtime::load_combat_variables::FIELDS.map(|f|variables.combat_values().bits(f)),
        "cached_svar_guards":manager.current_cached_svars(),
        "means":{"maximum_74":means.maximum_74.0,"attack":means.attack.0,"attack_rate":means.attack_rate.0,"defense":means.defense.0,"movement":means.movement.0,"health":means.health.0,"lifetime":means.lifetime.0,"offense":means.offense.0},
        "environment_controls":{"sse2":env.sse2_available,"mxcsr":env.mxcsr&!0x3F,"x87_control":env.x87_control}})
}

#[test]
#[ignore="requires AA_GAME_ROOT original tutorial02, verified source reference and actual current-thread CRT"]
fn actual_held_performers_use_current_variables_and_real_abilities_all_pc() {
    use aa_runtime::{combat_source_completion as completion,combat_source_receipt::VerifiedStartupReceipt,
        localization::Localization,restoration::{Attempt,LoadBindings},restore_lifecycle::{ScopedManagedLoad,TaskKind},settings::Settings};
    use aa_mapworld::restore::RestoreMode;
    use aa_rules::{actors::ActorCatalog,resources::ResourceCatalog,sources::{V14Sources,SourceLimits,sha256},tgi::Limits};
    use std::sync::Arc;
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let native=game.join("reconstruction/native-game/runtime-expanded-v14");
    let mut sources=V14Sources::open(game.join("Data.rwd"),&native,SourceLimits{max_member_bytes:64*1024*1024}).unwrap();
    let settings=Settings::load_v14(&mut sources,Limits::default()).unwrap();let text=Localization::load_v14(&mut sources,Limits::default()).unwrap();
    let actors=ActorCatalog::load_v14(&mut sources).unwrap();let resources=Arc::new(ResourceCatalog::load_v14(&mut sources).unwrap());
    let bytes=std::fs::read(game.join("reconstruction/assets/original/Data/Maps/Campaign/tutorial02.RMP")).unwrap();
    let map=aa_mapworld::MapArchive::from_bytes(bytes.clone()).unwrap();
    let inactive=ScopedManagedLoad::from_sources(&mut sources,0).unwrap().begin(TaskKind::Normal).unwrap();
    let mut attempt=Attempt::load_with_source_setup(&map,&actors,settings.clone(),text,Precision::Double53,u64::MAX,RestoreMode::InspectPartial,
        LoadBindings{native_load_mode:Some(1),inactive_load:Some(inactive),resources:Some(resources.clone())},|catalog|{
            let reference=VerifiedStartupReceipt::verify_embedded_reference(catalog,&mut sources)?;
            completion::install(catalog,&mut sources,reference.context(),reference.csv(),reference.globals())?;completion::compute(catalog)
        }).unwrap();
    let mut catalog=Catalog::new(&actors);catalog.settings=Some(settings);catalog.native_load_mode=Some(1);
    catalog.bind_resource_catalog(resources).unwrap();
    *catalog.definition_bank.borrow_mut()=attempt.definitions;*catalog.ability_arena.borrow_mut()=attempt.abilities;
    let source_fingerprint_before=catalog.definition_bank.borrow().combat_source().unwrap().result_fingerprint().unwrap();
    let mut variables=aa_runtime::load_view_variables::State::reset_from_fresh_source(&catalog,aa_runtime::load_view_variables::NativeOverrides::ControlledNone).unwrap();
    let inventory=aa_mapworld::WorldInventory::decode(&map,u64::MAX).unwrap();
    let rules=aa_runtime::load_world_rules::Owner::load(&catalog,&inventory).unwrap();variables.publish_loaded_rules(&catalog,&rules).unwrap();
    let ids=attempt.registry.iter().filter_map(|entry|{
        let RestoredRecord::Actor(v)=&entry.value else{return None};
        if v.failure.is_some(){return None}
        let Some(RestoredComponent::Performer(p))=v.shell.actor.as_ref()?.components.get(ComponentSlot::Performer)else{return None};
        let source_pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
        p.state.abilities.iter().any(|id|catalog.ability_arena.borrow().get(*id).is_ok_and(|a|a.enabled)
            && local_ability(&catalog,*id,source_pc).is_ok_and(|local|local.definition.flags_54&0x80==0
                &&f32::from_bits(local.definition.power_64.0)>0.0)).then_some(entry.id)
    }).take(4).collect::<Vec<_>>();assert!(!ids.is_empty(),"real restored enabled Performer required");
    let defaults=aa_runtime::load_combat_variables::FIELDS.map(|f|variables.combat_values().bits(f));
    let mut rows=vec![];let mut positive=0;let mut effects_error=false;let mut allocations=vec![];let mut environment_errors=vec![];
    for id in ids {
        let mut held=std::mem::replace(attempt.registry.value_mut(id).unwrap(),RestoredRecord::Loading);
        let RestoredRecord::Actor(entity)=&mut held else{unreachable!()};
        let mut record=Record{owner:id,shell:&mut entity.shell,definition:&entity.definition,registry:&mut attempt.registry};
        let (address,before)=match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Performer){Some(RestoredComponent::Performer(p))=>(p as *const _ as usize,p.word60),_=>unreachable!()};
        let modifier_before=match record.shell.actor.as_ref().unwrap().components.get(ComponentSlot::Properties){Some(RestoredComponent::Properties(p))=>Some(p.modifier.scalars),None=>None,_=>unreachable!()};
        for phase in ["default","changed_current_words","changed_modifier_words","negative_modifier_default_clamp","changed_current_clamp"] {
            if phase=="changed_current_words" {
                variables.assign_combat_bits(Field::RangePower,0.35f32.to_bits());variables.assign_combat_bits(Field::OffenseWeight,0.8f32.to_bits());
                variables.assign_combat_bits(Field::DefenseWeight,0.3f32.to_bits());variables.assign_combat_bits(Field::RangedMinimum,1f32.to_bits());
                variables.assign_combat_bits(Field::DefenseWeightRanged,0.6f32.to_bits());
            }
            if phase=="changed_modifier_words" {
                if let Some(RestoredComponent::Properties(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Properties) {
                    for (offset,value)in [(4,0.5f32),(8,1.0),(0xC,2.0),(0x28,1.5),(0x2C,-0.5),(0x64,1.25)] {p.modifier.scalars[offset/4-1]=F32(value.to_bits())}
                }
            }
            if phase=="negative_modifier_default_clamp"||phase=="changed_current_clamp" {
                if phase=="changed_current_clamp" {variables.assign_combat_bits(Field::ModifierClampAdd,30f32.to_bits());}
                if let Some(RestoredComponent::Properties(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Properties) {p.modifier.scalars[0]=F32((-10000f32).to_bits())}
            }
            for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
                let input=capture(&catalog,&record,&variables);
                refresh(&catalog,&mut record,&mut Context{variables:&mut variables,instances:None,current_environment:actual_fp::sample},pc).unwrap();
                let Some(RestoredComponent::Performer(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Performer)else{unreachable!()};
                assert_eq!(p as *const _ as usize,address);if f32::from_bits(p.word60)>0.0{positive+=1}
                rows.push(json!({"phase":phase,"precision":format!("{pc:?}"),"input":input,"output":p.word60}));p.word60=before;
            }
            for (i,f)in aa_runtime::load_combat_variables::FIELDS.iter().copied().enumerate(){variables.assign_combat_bits(f,defaults[i])}
            if let Some(before)=modifier_before {
                let Some(RestoredComponent::Properties(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Properties)else{unreachable!()};p.modifier.scalars=before;
            }
        }
        for pc in [Precision::Single24,Precision::Double53,Precision::Extended64] {
            let input=capture(&catalog,&record,&variables);
            let error=refresh(&catalog,&mut record,&mut Context{variables:&mut variables,instances:None,current_environment:unavailable_environment},pc).unwrap_err();
            assert!(error.to_string().contains("actual current-thread CRT service is unbound"));
            let Some(RestoredComponent::Performer(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Performer)else{unreachable!()};
            assert_eq!(p.word60,0);assert_eq!(p as *const _ as usize,address);
            environment_errors.push(json!({"precision":format!("{pc:?}"),"input":input,"output":p.word60}));p.word60=before;
        }
        //Explicit missing actual instance owner: the property-sum prefix fails
        //BEFORE native zeroes Performer+60. This does not fabricate an arena.
        if let Some(RestoredComponent::Properties(p))=record.shell.actor.as_mut().unwrap().components.get_mut(ComponentSlot::Properties) {
            assert!(p.effects.is_empty());p.effects.push(Object(0xF0000001));
            let error=refresh(&catalog,&mut record,&mut Context{variables:&mut variables,instances:None,current_environment:actual_fp::sample},Precision::Double53).unwrap_err();
            assert!(error.to_string().contains("actual pre-staging instance owner"));
            let actor=record.shell.actor.as_mut().unwrap();let Some(RestoredComponent::Performer(p))=actor.components.get(ComponentSlot::Performer)else{unreachable!()};
            assert_eq!(p.word60,before);assert_eq!(p as *const _ as usize,address);
            let Some(RestoredComponent::Properties(p))=actor.components.get_mut(ComponentSlot::Properties)else{unreachable!()};p.effects.pop();effects_error=true;
        }
        allocations.push(json!({"id":id.raw(),"same_performer_allocation":true}));
        drop(record);*attempt.registry.value_mut(id).unwrap()=held;
    }
    assert!(positive>0&&effects_error);
    let differs=rows.chunks_exact(15).any(|rows|rows[0]["output"]!=rows[3]["output"]);assert!(differs,"current SVar changes must affect observed positive Performer value");
    let clamp_differs=rows.chunks_exact(15).any(|rows|rows[9]["output"]!=rows[12]["output"]);assert!(clamp_differs,"changing only the current clamp cell must affect the same negative-modifier input");
    let source_fingerprint_after=catalog.definition_bank.borrow().combat_source().unwrap().result_fingerprint().unwrap();
    assert_eq!(source_fingerprint_before,source_fingerprint_after,"current runtime evaluations must preserve the immutable source completion results");
    let output=json!({"scope":"Real diagnostic tutorial02 held Performer/Body/Mover/Properties allocations and actual AbilityArena; default/current mutated SVar values at all3PC; refresh-only, no completed ActorInit/battle claim; positive instance-owner fault is explicit test input",
        "map_sha256":sha256(&bytes),"rows":rows,"environment_errors":environment_errors,"allocations":allocations,"positive_results":positive,"current_word_change_changes_result":differs,"current_clamp_only_change_changes_result":clamp_differs,"source_result_fingerprint_unchanged":source_fingerprint_before,"missing_instance_prefix_retained":effects_error});
    if let Some(path)=std::env::var_os("AA_PERFORMER_VALUE_CAPTURE"){
        serde_json::to_writer_pretty(std::fs::OpenOptions::new().write(true).create_new(true).open(path).unwrap(),&output).unwrap();
    }else{
        let expected:Value=serde_json::from_slice(&std::fs::read(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../research/skirmish-20261017-performer-value/actual-performer-values.json")).unwrap()).unwrap();assert_eq!(output,expected);
    }
}
