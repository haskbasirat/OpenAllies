"""Seal composition/check evidence; deliberately does not claim factory execution."""
from pathlib import Path
import hashlib,json,shutil
folder=Path(__file__).resolve().parent
root=folder.parents[1]
def digest(p):return hashlib.sha256(p.read_bytes()).hexdigest()
owned=['crates/aa-runtime/src/view_startup_inputs.rs','crates/aa-runtime/src/view_startup_inputs_tests.rs','crates/aa-runtime/src/view_grid_variables.rs','crates/aa-runtime/src/world_ready_services.rs']
dependencies=['crates/aa-runtime/src/view_grid_startup.rs','crates/aa-runtime/src/view_renderer_variables.rs','crates/aa-runtime/src/view_grid_state.rs','crates/aa-runtime/src/view_grid_occupancy.rs']
out=folder/'bundle-source-snapshot';out.mkdir(exist_ok=True)
for name in owned:shutil.copyfile(root/name,out/Path(name).name)
logs=['bundle-production-check-01.log','bundle-tests-check-01.log','bundle-probe-build-01.log','bundle-probe-build-02.log','bundle-probe-build-03.log']
report={
 'scope':'Application-owned composition of existing sealed seeds and distinct current AVar cells; not VWorld, scene activation or battle completion',
 'production_files':{name:digest(root/name)for name in owned},
 'component_source_pins':{name:digest(root/name)for name in dependencies},
 'logs':{name:digest(folder/name)for name in logs},
 'production_check':{'result':'pass','seconds':49.67,'command':'CARGO_TARGET_DIR=target/s-borrowed-leader-check cargo check -p aa-runtime --lib --release --locked --offline -j2','qualification':'SDK orientation provenance hardening was saved after this production check began; that final component delta awaits its author/current cohort checks'},
 'test_metadata_check':{'result':'blocked by unrelated external test root','errors':['tests/world_revision.rs path-includes cfg(test) source requiring crate::load_sim_world and crate::live_sim, neither declared in that external root'],'owned_bundle_errors_reported':False},
 'actual_source_probe':{'result':'not executed','reasons':['native-runtime runtime rlib dependency hashes no longer match rebuilt aa-mapworld artifacts','older coherent owned rlib retains private Catalog.require_source_owner and lacks current APIs'],'no_private_guard_stubbed':True,'mechanical_helper':'build_bundle_probe.py copies require_reference_numeric exactly; all component sources path-included'},
 'pending_filter':'view_startup_inputs::tests::actual_source_factory_retains_one_bundle_and_application --ignored --exact',
 'retention':['Factory Failure retains reached Seeds, Grid and Orientation owners; incomplete Grid and Orientation refuse validated use','Application install returns incoming Inputs on duplicate or validation failure','Existing Application Parts remains unchanged; dedicated ViewStartupParts lends same Loading plus Inputs','Moved authoritative Bank is explicitly borrowed for validation; no reconstructed Bank or mirrored cells'],
 'prior_native_proofs':['hardware-grid-seed-reference.json','native-grid-variables.json','native-view-grid.json','native-grid-occupancy.json','../development10/native-renderer-orientation.json'],
 'limitations':['No new native behavior is introduced by composition; component-specific oracle scopes remain separate','No successful actual factory or Application-install execution yet','No added rendering, native input, Picking, VWorld startup completion or playable battle claim','Client preparation and VWorld coordinator integration remain root-owned'],
}
(folder/'bundle-findings-pending-actual.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'report_sha256':digest(folder/'bundle-findings-pending-actual.json'),'production_files':report['production_files']},indent=2))
