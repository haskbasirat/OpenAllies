//!Actual source-only lab; no Actor allocation, World, FX execution or battle.
use aa_numeric::Precision;
use aa_rules::{actors::ActorCatalog,sources::{SourceLimits,V14Sources},tgi::Limits};
use aa_runtime::{catalog::Catalog,combat_source_receipt::VerifiedStartupReceipt,definition_fx::{ListKind,Source}};
fn main(){
    let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").expect("AA_GAME_ROOT"));
    let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
    let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
    catalog.settings=Some(aa_runtime::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
    let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
    aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
    aa_runtime::combat_source_completion::compute(&catalog).unwrap();
    let binding=catalog.actor_bindings("ger_corps",Precision::Double53).unwrap();
    let bank=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
    assert!(catalog.definition_bank.borrow().combat_source().is_err());
    let source=Source::bind_in_bank(&catalog,&bank,&binding.definition).unwrap();
    let pc=bank.combat_source().unwrap().source_precision();bank.combat_source().unwrap().completed_means().unwrap();
    assert_eq!(source.source_precision(),pc);assert_eq!(source.canonical_id(),"ger_corps");
    let rows=[ListKind::Created,ListKind::Constructed].into_iter().map(|kind|{
        let list=source.list(kind);assert!(list.is_certified_empty());assert!(list.authored_entries().is_empty());
        serde_json::json!({"list":kind.node_name(),"native_offset":list.native_offset(),"certified_null_head":true,"source_entries":list.authored_entries().len()})
    }).collect::<Vec<_>>();
    println!("{}",serde_json::json!({"scope":"Actual ger_corps source definition and both null-list certificates from same completed source Bank moved out of Catalog. No Actor/World/FX publication or natural fresh mission executed.","canonical_id":source.canonical_id(),"definition_source":source.definition_source(),"source_precision":format!("{pc:?}"),"native_class":binding.definition.native_class,"component_mask":binding.definition.component_mask,"catalog_bank_empty_after_move":true,"lists":rows}));
}
