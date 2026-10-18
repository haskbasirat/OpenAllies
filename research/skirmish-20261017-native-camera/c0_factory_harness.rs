// Exact production C0 module plus a mechanically retained private numeric guard.
// All data/Bank/public Settings are real current compiled runtime owners.
pub use aa_runtime::{catalog,definitions,combat_value_source_context,load_view_variables,settings};
mod combat_source_receipt {
 pub use aa_runtime::combat_source_receipt::*;
 use aa_runtime::combat_value_source_context as context;
pub(crate) fn require_reference_numeric(numeric: &context::NumericBindings<'_, '_>) -> Result<()> {
    let expected = [
        ("CombatDamageMinimum", 0x3f800000),
        ("CombatValueRangePower", 0x3e19999a),
        ("CombatValueAreaPower", 0x3f800000),
        ("CombatValueOffenseWeight", 0x3f000000),
        ("CombatValueDefenseWeight", 0x3f000000),
        ("CombatValueMovementWeight", 0x3e4ccccd),
        ("CombatValueMovementDefault", 0x3f800000),
        ("CombatValueMeanValuesDiscardPercent", 0x3e4ccccd),
        ("CombatValueRangedMinimum", 0x41400000),
        ("CombatValueDefenseWeightRanged", 0x3e800000),
    ];
    if numeric.values().len() != expected.len() {
        return Err("reference numeric setting set differs".into());
    }
    for (name, bits) in expected {
        let actual = numeric.value(name)?;
        if actual.native_kind != 2 || actual.bits != bits {
            return Err(format!("typed reference numeric setting {name} differs"));
        }
    }
    Ok(())
}
}

#[path="../../crates/aa-runtime/src/native_camera_render_source.rs"] mod native_camera_render_source;
fn main(){
 use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
 use aa_runtime::{catalog::Catalog,settings::Settings,load_view_variables::NativeOverrides};
 use combat_source_receipt::VerifiedStartupReceipt;
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=ActorCatalog::load_v14(&mut files).unwrap();let mut catalog=Catalog::new(&actors);
 catalog.settings=Some(Settings::load_v14(&mut files,Limits::default()).unwrap());
 let receipt=VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
 aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
 aa_runtime::combat_source_completion::compute(&catalog).unwrap();
 let bank=catalog.definition_bank.borrow();
 let mut color=native_camera_render_source::State::from_fresh_sources_retained(&catalog,&bank,&mut files,NativeOverrides::ControlledNone).unwrap();
 assert_eq!(color.current(&catalog,&bank).unwrap(),[0x3f333333,0x3f333333,0x3f19999a]);
 assert_eq!(color.source_files().len(),5);
 color.assign(&catalog,&bank,[0,0x3f800000,0x40000000]).unwrap();
 assert_eq!(color.current(&catalog,&bank).unwrap(),[0,0x3f800000,0x40000000]);
 assert!(color.assign(&catalog,&bank,[0x7f800000,0,0]).is_err());
 assert_eq!(color.current(&catalog,&bank).unwrap(),[0,0x3f800000,0x40000000]);
 drop(bank);
 let moved=std::mem::take(&mut*catalog.definition_bank.borrow_mut());
 color.validate(&catalog,&moved).unwrap();
 let row=&mut catalog.settings.as_mut().unwrap().app_variables.get_mut("stencilshadowcolormultiplier").unwrap()[0];row.raw="0.7,0.7,0.7".into();
 assert!(color.validate(&catalog,&moved).unwrap_err().contains("public source declaration differs"));
 println!("PASS: actual five-file source C0 [.7,.7,.6], sourcePC/completed real Bank guard, same mutable tuple/rejected nonfinite prefix, same Bank move, changed public source row refused. No camera construction/VWorld/activation.");
}
