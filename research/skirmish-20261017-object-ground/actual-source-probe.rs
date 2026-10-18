pub use aa_runtime::{catalog,definitions,live_object_init};
#[path="E:/Games/Axis and Alies/rust-rebuild/crates/aa-runtime/src/object_ground_source.rs"] mod ground;
use aa_rules::{sources::{V14Sources,SourceLimits},tgi::Limits};
fn main(){
 let game=std::path::PathBuf::from(std::env::var_os("AA_GAME_ROOT").unwrap());
 let mut files=V14Sources::open(game.join("Data.rwd"),game.join("reconstruction/native-game/runtime-expanded-v14"),SourceLimits{max_member_bytes:u64::MAX}).unwrap();
 let actors=aa_rules::actors::ActorCatalog::load_v14(&mut files).unwrap();
 let mut catalog=catalog::Catalog::new(&actors);
 catalog.settings=Some(aa_runtime::settings::Settings::load_v14(&mut files,Limits::default()).unwrap());
 let receipt=aa_runtime::combat_source_receipt::VerifiedStartupReceipt::verify_embedded_reference(&catalog,&mut files).unwrap();
 aa_runtime::combat_source_completion::install(&catalog,&mut files,receipt.context(),receipt.csv(),receipt.globals()).unwrap();
 aa_runtime::combat_source_completion::compute(&catalog).unwrap();
 for id in ["ger_corps","usa_corps","gbr_corps"]{
  let binding=catalog.actor_bindings(id,aa_numeric::Precision::Double53).unwrap();
  let bank=catalog.definition_bank.borrow();
  let source=ground::Source::bind(&catalog,&bank,&binding.definition).unwrap();
  println!("{} {:?}",id,source);
  for i in 0..actors.native_data_registry("Biome").unwrap().entries.len(){assert_eq!(source.variation(i).unwrap(),None);}
  assert_eq!(source.padding(),1f32.to_bits());
 }
}
