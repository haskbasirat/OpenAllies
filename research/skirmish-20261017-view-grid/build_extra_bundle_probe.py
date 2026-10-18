"""Actual source acceptance for the two disjoint approved extra owners."""
from pathlib import Path
import json,hashlib,sys,textwrap
folder=Path(__file__).resolve().parent;root=folder.parents[1]
header=(folder/'bundle_factory_harness.rs').read_text().split('\nfn main()')[0]
header+='''
pub use aa_runtime::view_init_assets;
#[path="../../crates/aa-runtime/src/native_camera_variables.rs"] mod native_camera_variables;
#[path="bundle_alias_source.rs"] mod view_asset_aliases;
'''
ordering=(root/'crates/aa-rules/src/native_field_order.rs').read_text()
functions=ordering[ordering.index('fn hash('):ordering.index("impl<'a> ResolvedNode")]
functions+=ordering[ordering.index('pub fn native_fresh_field_order('):]
header+='mod native_field_metadata { use aa_rules::native::{Error,Result}; use std::collections::BTreeSet;\n'+functions+'\n}\n'
settings=(root/'crates/aa-runtime/src/settings.rs').read_text()
constant=settings[settings.index('pub const APP_VARIABLE_FILES:'):settings.index('impl Settings {')]
load=settings[settings.index('    pub fn load_v14('):settings.index('    /// The LanguageIDS')]
load=textwrap.dedent(load).replace('pub fn load_v14(','pub fn load_current_v14(').replace('Result<Self>','Result<Settings>').replace('Self::from_files','Settings::from_files')
collect=settings[settings.index('fn collect('):]
header+='mod settings { pub use aa_runtime::settings::{Settings,Declaration,Error,Result};\nuse std::collections::BTreeMap;\nuse aa_rules::{resources::Provenance,sources::{ResolvedFile,V14Sources},tgi::{Document,Item,Limits}};\n'+constant+load+collect+'\n}\n'
alias=(root/'crates/aa-runtime/src/view_asset_aliases.rs').read_text()
assert alias.count('    actors::native_fresh_field_order,')==1
alias=alias.replace('    actors::native_fresh_field_order,','')
alias=alias.replace('use aa_rules::{','use crate::native_field_metadata::native_fresh_field_order;\nuse aa_rules::{',1)
(folder/'bundle_alias_source.rs').write_text(alias)
main='''
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
 println!("CameraHeight source {:?}",catalog.settings.as_ref().unwrap().app_variables.get("cameraheight"));
 let mut camera=native_camera_variables::State::from_fresh_sources_retained(&catalog,&bank,&mut files,NativeOverrides::ControlledNone).unwrap();
 assert_eq!(camera.word(&catalog,&bank,0x24).unwrap(),0.1f32.to_bits());
 assert_eq!(camera.word(&catalog,&bank,0x28).unwrap(),1.5f32.to_bits());
 assert_eq!(camera.word(&catalog,&bank,0x2c).unwrap(),1.0f32.to_bits());
 assert_eq!(camera.word(&catalog,&bank,0x40).unwrap(),2.1f32.to_bits());
 let mut called=false;
 assert!(!camera.assign(&catalog,&bank,0x40,||{called=true;Err("unreached proposed value".into())}).unwrap());assert!(!called);
 camera.assign(&catalog,&bank,0x2c,||Ok(9.0f32.to_bits())).unwrap();
 assert_eq!(camera.word(&catalog,&bank,0x2c).unwrap(),2.5f32.to_bits());
 drop(bank);
 let mut aliases=view_asset_aliases::State::from_fresh_sources_retained(&catalog,&mut files,NativeOverrides::ControlledNone).unwrap();
 let bank=catalog.definition_bank.borrow();
 let mut keys=0;for key in view_init_assets::alias_keys(){
  let value=aliases.alias_410cb0(&catalog,&bank,key).unwrap();
  let rows=catalog.settings.as_ref().unwrap().app_variables.get(&key.to_ascii_lowercase()).unwrap();
  let raw=rows.last().unwrap().raw.trim();let literal=raw.strip_prefix('"').and_then(|v|v.strip_suffix('"')).unwrap_or(raw);
  assert_eq!(value.units(),literal.encode_utf16().collect::<Vec<_>>());keys+=1;
 }assert_eq!(keys,22);
 let key=view_init_assets::alias_keys().next().unwrap();
 let prior=aliases.alias_410cb0(&catalog,&bank,key).unwrap();let expected=prior.units().to_vec();
 aliases.assign(&catalog,&bank,key,&[0x58,0,0xd800]).unwrap();
 assert_eq!(prior.units(),expected);assert_eq!(aliases.alias_410cb0(&catalog,&bank,key).unwrap().units(),[0x58,0,0xd800]);
 drop(bank);
 let moved=std::mem::take(&mut *catalog.definition_bank.borrow_mut());
 aliases.validate_in_bank(&catalog,&moved).unwrap();camera.validate(&catalog,&moved).unwrap();
 assert_eq!(camera.word(&catalog,&moved,0x2c).unwrap(),2.5f32.to_bits());
 assert_eq!(aliases.alias_410cb0(&catalog,&moved,key).unwrap().units(),[0x58,0,0xd800]);
 assert!(view_asset_aliases::State::from_fresh_sources_retained(&catalog,&mut files,NativeOverrides::ControlledNone).is_err());
 println!("PASS: actual source camera current17cells/v14zoom/read-only lazy gate/clamp and all22 aliases, counted alias survives reassignment, same Bank move; no camera/VWorld/scene activation");
}
'''
main=main.replace('Settings::load_v14(&mut files,Limits::default())','crate::settings::load_current_v14(&mut files,Limits::default())')
alias_only='--alias-only' in sys.argv
if alias_only:
 start=main.index(' let mut camera=')
 end=main.index(' drop(bank);',start)
 main=main[:start]+main[end:]
 main=main.replace('aliases.validate_in_bank(&catalog,&moved).unwrap();camera.validate(&catalog,&moved).unwrap();','aliases.validate_in_bank(&catalog,&moved).unwrap();')
 main=main.replace(' assert_eq!(camera.word(&catalog,&moved,0x2c).unwrap(),2.5f32.to_bits());\n','')
 main=main.replace('actual source camera current17cells/v14zoom/read-only lazy gate/clamp and all22 aliases','actual source all22 aliases')
(folder/('alias_factory_harness.rs' if alias_only else 'bundle_extra_factory_harness.rs')).write_text(header+main)
files=['native_camera_variables.rs','view_asset_aliases.rs','view_init_assets.rs']
pins={str(Path('crates/aa-runtime/src')/p):hashlib.sha256((root/'crates/aa-runtime/src'/p).read_bytes()).hexdigest()for p in files}
pins['mechanical_hash_order_functions_sha256']=hashlib.sha256(functions.encode()).hexdigest()
pins['generated_alias_source_sha256']=hashlib.sha256(alias.encode()).hexdigest()
pins['settings_current_source_sha256']=hashlib.sha256(settings.encode()).hexdigest()
pins['settings_current_loader_collect_sha256']=hashlib.sha256((constant+load+collect).encode()).hexdigest()
(folder/'bundle-extra-source-pins.json').write_text(json.dumps(pins,indent=2)+'\n')
print('extra actual source harness generated')
