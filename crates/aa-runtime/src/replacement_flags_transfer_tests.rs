//! Cold post-CV source suffix -> real SourceTransfer -> first readonly Bank
//! read. No Catalog lazy getter or separate State::bind prewarms the cache.
use super::*;
use std::sync::Arc;
#[test]
#[ignore = "requires AA_GAME_ROOT actual source-completed Bank and accepted v14 receipt"]
fn cold_source_suffix_survives_actual_transfer_before_first_in_bank_read() {
    use crate::{combat_source_completion as completion,combat_source_receipt::VerifiedStartupReceipt};
    use aa_rules::{actors::ActorCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits {max_member_bytes:u64::MAX}).unwrap();
    let actors=Arc::new(ActorCatalog::load_v14(&mut files).unwrap());
    let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(crate::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
    let proof=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    assert!(complete_source_suffix(&catalog).is_err());
    completion::install(&catalog,&mut files,proof.context(),proof.csv(),proof.globals()).unwrap();
    assert!(complete_source_suffix(&catalog).is_err());
    completion::compute(&catalog).unwrap();
    let pc=catalog.definition_bank.borrow().combat_source().unwrap().source_precision();
    let fields=actors.native_source_requirements(pc).unwrap();
    {
        let bank=catalog.definition_bank.borrow();
        assert!(bank.replacement_flags.is_none());
        assert!(catalog.verified_placeholder_flags_in_bank(&bank,"placeholder_airfield",&fields).unwrap_err().contains("no completed Placeholder"));
    }
    complete_source_suffix(&catalog).unwrap();
    let (cv_allocation,bound_allocation,token)= {
        let bank=catalog.definition_bank.borrow();
        let cache=bank.replacement_flags.as_ref().unwrap();
        assert_eq!(cache.placeholder_count(),6);assert_eq!(cache.verified_pairs(),30);
        (bank.combat_source().unwrap() as *const _ as usize,
         cache.placeholders.get("placeholder_airfield").unwrap() as *const Bound as usize,
         bank.token("ActorDefinition","placeholder_airfield").unwrap())
    };
    complete_source_suffix(&catalog).unwrap(); // idempotent owner validation
    let transfer=match catalog.into_source_transfer(Arc::clone(&actors)) {Ok(v)=>v,Err((e,_))=>panic!("{e}")};
    let (same_actors,data)=transfer.into_parts();
    assert!(Arc::ptr_eq(&same_actors,&actors));
    let catalog=match data.bind(&same_actors) {Ok(v)=>v,Err((e,_))=>panic!("{e}")};
    let bank=catalog.definition_bank.borrow();
    //This is the first actual cached flag read after source construction. The
    //getter is immutable and cannot hide a fresh State::bind or replacement.
    assert_eq!(catalog.verified_placeholder_flags_in_bank(&bank,"placeholder_airfield",&fields).unwrap(),0x20816500);
    assert_eq!(bank.combat_source().unwrap() as *const _ as usize,cv_allocation);
    assert_eq!(bank.replacement_flags.as_ref().unwrap().placeholders.get("placeholder_airfield").unwrap() as *const Bound as usize,bound_allocation);
    assert_eq!(bank.token("ActorDefinition","placeholder_airfield"),Some(token));
    let native:serde_json::Value=serde_json::from_str(include_str!("../../../research/skirmish-20261017-placeholder-flags/native-placeholder-flags.json")).unwrap();
    let mut comparisons=0;
    for row in native["actual"].as_array().unwrap() {
        if row["pc"].as_str().unwrap()!=format!("{pc:?}") {continue}
        let id=row["input"]["placeholder"].as_str().unwrap();
        assert_eq!(catalog.verified_placeholder_flags_in_bank(&bank,id,&fields).unwrap(),row["native"]["flags"].as_u64().unwrap() as u32);
        comparisons+=1;
    }
    assert_eq!(comparisons,30);
    let wrong_pc=if pc==Precision::Single24 {Precision::Double53}else{Precision::Single24};
    let wrong=actors.native_source_requirements(wrong_pc).unwrap();
    assert!(catalog.verified_placeholder_flags_in_bank(&bank,"placeholder_airfield",&wrong).is_err());
    assert!(catalog.verified_placeholder_flags_in_bank(&bank,"gbr_corps",&fields).is_err());
    println!("cold source suffix + real SourceTransfer:6 Placeholders/30 native pairs; SAME CV box/cache nodes/tokens; no prewarm");
}
