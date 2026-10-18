"""Run the production test's factory/prefix portion without linking all unit tests."""
from pathlib import Path
import hashlib,json
folder=Path(__file__).resolve().parent;root=folder.parents[1]
header=(folder/'bundle_extra_factory_harness.rs').read_text().split('\nfn main()')[0]
header+='\npub use aa_runtime::combat_source_completion;\n'
test=(root/'crates/aa-runtime/src/view_startup_inputs_tests.rs').read_text()
body=test[test.index('    let game ='):test.index('    let startup =')]
body=body.replace('crate::settings::Settings::load_v14','crate::settings::load_current_v14')
program=header+'''\nfn main(){
use aa_rules::{actors::ActorCatalog,sources::{V14Sources,SourceLimits},tgi::Limits};
use crate::{catalog::Catalog,combat_source_receipt::VerifiedStartupReceipt,load_view_variables::NativeOverrides,view_startup_inputs::Inputs};
'''+body+'''
println!("PASS: actual five-owner Inputs factory and early/Grid/late-Alias retained failure prefixes; camera v14zoom/current aliases verified; no Application-install, VWorld or battle execution");
}
'''
(folder/'bundle5_factory_harness.rs').write_text(program)
pins={str(Path('crates/aa-runtime/src')/name):hashlib.sha256((root/'crates/aa-runtime/src'/name).read_bytes()).hexdigest()for name in ['view_startup_inputs.rs','view_startup_inputs_tests.rs','native_camera_variables.rs','view_asset_aliases.rs','settings.rs']}
(folder/'bundle5-source-pins.json').write_text(json.dumps(pins,indent=2)+'\n')
print('five-owner harness generated from current production test factory portion')
