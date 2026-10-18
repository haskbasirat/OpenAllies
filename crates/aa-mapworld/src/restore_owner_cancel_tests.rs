//! Portable owner-branch/catch replay. These synthetic predecoded records are
//! deliberately not source-created complete Worlds or definitions. Native proof
//! scope is the owner slice, RTTI link and catch body with a destruction leaf.
use super::*;
use crate::{IdManagerState, SourceIdentity, WorldState, inventory::{ActorPrefix, ByteSpan, Inventory, ThingState}};
use serde_json::Value;

struct Definitions { flags:u32 }
impl RestoreDefinitions for Definitions {
    fn resolve(&self, _: &[u16], _:Precision)->std::result::Result<RuntimeDefinition,String> {
        Ok(RuntimeDefinition {
            proof:DefinitionProof {
                id:"owner-cancellation-laboratory".into(),
                source:aa_rules::resources::Provenance { path:"controlled-native-owner-slice".into(),sha256:"laboratory".into(),span:aa_rules::tgi::Span {start:0,end:0}},
                template_chain:vec![],
                category:*aa_rules::native::CATEGORIES.iter().find(|c|c.runtime_class==3).unwrap(),
                base_flag_inputs:vec![],
            },
            native_class:3,flags:self.flags,object:None,component_mask:Some(0),ground:GroundDefinitions::default(),
        })
    }
}
fn inventory()->WorldInventory {
    let source=SourceIdentity::of(b"controlled owner-prefix fixture, no native asset");
    WorldInventory {
        map_source:source.clone(), header:None,header_error:None,
        teams:None,teams_error:None,team_generation_mismatches:vec![],kingdoms:None,kingdoms_error:None,kingdom_generation_mismatches:vec![],ownership:None,
        world:WorldState {source:source.clone(),version:25,rng_state:1,word_dc:0,begin_time_bits:0,word_e4:0,delta_bits:0,word_140:0,word_184:None,word_194:None,word_198:None,word_44:None,word_48:None,trailing:vec![]},
        id_manager:IdManagerState {source:source.clone(),next_slot:1,highest_slot:0,capacity:65536,serialized_generations:vec![]},
        dynamic_things:Inventory {map_source:source.clone(),data_source:source.clone(),index_source:source,data_header_word:0,placements:vec![],unsupported:vec![]},
        generation_mismatches:vec![],opaque_initialization:vec![],absent_sections:vec![],
    }
}
fn number(row:&Value,key:&str)->u32 {row[key].as_u64().unwrap().try_into().unwrap()}

#[test]
fn native_owner_branch_cancellation_reaches_existing_catch_and_null_result() {
    let fixture:Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-actor-owner-cancel/native-owner-cancel.json")).unwrap();
    assert_eq!(fixture["cases"].as_array().unwrap().len(),60);
    let inventory=inventory();
    let mut branch_rows=0;let mut canceled=0;let mut handler_rows=0;
    for row in fixture["cases"].as_array().unwrap() {
        if row.get("handler").is_some() {
            assert_eq!(row["handler"]["events"],serde_json::json!(["destroy4F5490"]));
            assert_eq!(row["handler"]["output_null"],true);
            handler_rows+=1; continue;
        }
        branch_rows+=1;
        let c=&row["case"];let flags=number(c,"flags");let owner=number(c,"owner");let generation=number(c,"generation");
        let slot=(owner&0xffff) as usize;
        let highest=slot.max(77)+1;
        let mut generations=vec![0u16;highest];generations[slot]=generation as u16;
        let mut registry=IdentityRegistry::from_load_metadata((highest+1)as u32,highest as u32,&generations).unwrap();
        if c["registered"].as_bool().unwrap() {
            let current=ActorId::from_raw((generation<<16)|(owner&0xffff));
            registry.register_saved(current,true,RestoredRecord::Loading).unwrap();
        }
        let before=registry.len();
        let source=Placement {
            record_ordinal:0,source_span:ByteSpan {offset:0,length:0},thing_id_utf16:vec![],class_code:3,serialized_id:77,
            thing:ThingState {version:2,name_utf16:vec![],word_1c:0,position_bits:[0;3],facing_bits:[0;3],serialized_flags:0,legacy_version_one_word:None},
            object:None,actor:Some(ActorPrefix {version:14,owner_handle:owner,state_low:0x12345678,state_high:Some(0x87654321)}),
            // A continued owner branch must reach this actual missing tail;
            // cancel must happen first without consuming or publishing it.
            opaque_tail_span:ByteSpan {offset:0,length:0},opaque_tail:vec![],
        };
        let pc:Precision=serde_json::from_value(row["precision"].clone()).unwrap();
        let mut rng=NativeRng::new(1);let mut regs=vec![];let mut effects=vec![];let mut skipped=vec![];let mut messages=vec![];let mut lifecycle=vec![];let mut held=vec![];let mut version=25;let mut latch=false;
        let mut cx=LoadContext {inventory:&inventory,rng:&mut rng,pc,updatable_registrations:&mut regs,presentation_effects:&mut effects,skipped:&mut skipped,native_messages:&mut messages,lifecycle:&mut lifecycle,lifecycle_detached:&mut held,world_version:&mut version,assert_latch:&mut latch,source_ordinal:0};
        let (registered,completion,result)=load_record(&mut registry,&source,0,false,&Definitions {flags},&mut cx);
        assert!(registered,"{c:?}");
        assert!(regs.is_empty()&&effects.is_empty()&&messages.is_empty()&&held.is_empty());
        if row["native"]["terminal"]=="load_cancel" {
            canceled+=1;
            assert_eq!(completion,Some(LoadCompletion::ThingLoaded));
            assert!(matches!(result,Ok(None)),"{c:?}: cancellation must yield null");
            assert_eq!(registry.len(),before);
            assert!(registry.get(ActorId::from_raw(77)).is_none());
            assert_eq!(registry.generation_at(77),1);
            assert_eq!(skipped.len(),1);
            assert!(matches!(&skipped[0].cause,SkipCause::LoadException {cancel:true,message} if message.starts_with("Map Load Canceled:")));
            assert!(lifecycle.is_empty(),"successful caught destruction is a native NULL skip, never a failed or completed immediate tail");
            // Same actual registry accepts the following record, rather than
            // reviving the removed Kingdom or marking failed77 as loaded.
            registry.register_saved(ActorId::from_raw(78),false,RestoredRecord::Loading).unwrap();
            assert!(registry.get(ActorId::from_raw(78)).is_some());
            //The same classified exception must remain a real failure when
            //its catch/destruction service cannot finish. Nested stream
            //position is deliberately unported; no prior skip is rolled back.
            let mut nested_source=source.clone();nested_source.serialized_id=79;
            let mut retry=LoadContext {inventory:&inventory,rng:&mut rng,pc,
                updatable_registrations:&mut regs,presentation_effects:&mut effects,
                skipped:&mut skipped,native_messages:&mut messages,lifecycle:&mut lifecycle,
                lifecycle_detached:&mut held,world_version:&mut version,assert_latch:&mut latch,
                source_ordinal:0};
            let (_,_,failure)=load_record(&mut registry,&nested_source,0,true,&Definitions {flags},&mut retry);
            let error=failure.err().expect("unported catch/destruction remains required");
            assert!(error.contains("enclosing stream position"));
            assert!(registry.get(ActorId::from_raw(79)).is_some());
            assert_eq!(skipped.len(),1);
            assert_eq!(lifecycle.len(),1);
            assert!(!lifecycle[0].attempted&&!lifecycle[0].completed&&!lifecycle[0].omitted);
            assert_eq!(lifecycle[0].output_id,Some(79));
            assert!(lifecycle[0].error.as_ref().unwrap().contains(&error));
        } else {
            assert_eq!(completion,Some(LoadCompletion::ActorPrefixLoaded));
            let error=result.err().expect("empty Actor tail must remain a reached error");
            assert!(caught_load_exception(&error).is_none(),"{c:?}: {error}");
            assert!(skipped.is_empty());
            assert_eq!(lifecycle.len(),1);
            assert!(!lifecycle[0].attempted&&!lifecycle[0].completed&&!lifecycle[0].omitted);
            assert!(lifecycle[0].error.as_ref().unwrap().contains(&error));
            let actual=registry.get(ActorId::from_raw(77)).unwrap().value.loaded().unwrap().shell.actor.as_ref().unwrap().words[&0xe8];
            assert_eq!(actual!=0,row["native"]["owner_present"].as_bool().unwrap(),"{c:?}");
        }
    }
    assert_eq!((branch_rows,canceled,handler_rows),(54,24,6));
}

#[test]
fn unrelated_missing_service_errors_remain_uncaught() {
    assert_eq!(caught_load_exception("Actor owner is unavailable"),None);
    assert_eq!(caught_load_exception("required native service5A0910 not bound"),None);
    assert_eq!(caught_load_exception("Map Load Canceled: owner not found"),Some(true));
    assert_eq!(caught_load_exception(&load_warning_exception("source changed")),Some(false));
}
