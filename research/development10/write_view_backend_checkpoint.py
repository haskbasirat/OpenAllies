"""Pin this bounded development checkpoint; never certify view/battle readiness."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
files=[
 'crates/aa-client/src/view_directory.rs','crates/aa-client/src/view_object_fields.rs',
 'crates/aa-client/src/view_request_policy.rs','crates/aa-client/src/view_scene_backend.rs',
 'crates/aa-client/src/view_world_owner.rs','crates/aa-client/src/battle_map_upload.rs',
 'crates/aa-client/src/live_scene.rs','crates/aa-client/src/loading.rs',
 'crates/aa-client/src/gpu.rs','crates/aa-client/src/scene.wgsl',
 'crates/aa-client/src/dynamic.wgsl','crates/aa-client/src/particles.wgsl',
 'crates/aa-runtime/src/view_renderer_variables.rs','crates/aa-runtime/src/fx_manager.rs',
 'crates/aa-runtime/src/live_world_frame.rs','crates/aa-runtime/src/resource_depot.rs',
 'research/development10/view-backend-checkpoint.md',
 'research/development10/fx-manager/README-attachment.md',
 'research/skirmish-20261017-renderer-mode/README.md',
]
evidence=[
 'research/development10/native-view-publication.json','research/development10/native-view-fields.json',
 'research/development10/native-view-flag14.json','research/development10/native-view-request-queue.json',
 'research/development10/native-renderer-orientation.json',
 'research/development10/fx-manager/native-view-attachment.json',
 'research/skirmish-20261017-renderer-mode/native-request-policy.json',
 'research/skirmish-20261017-renderer-mode/hardware-request-mode-reference.json',
 'research/development10/view-current-client-tests01.log','research/development10/view-scene-backend-tests01.log',
 'research/development10/battle-map-transport-tests01.log','research/development10/view-renderer-witness-tests-current.log',
 'research/development10/view-current-headless-gpu-tests01.log','research/development10/view-shader-tests01.log',
 'research/development10/view-backend-scalar-tests02.log','research/development10/fx-manager/attachment-tests02.log',
 'research/development10/view-renderer-variables-tests01.log','research/development10/view-directory-check-tests12.log',
 'research/development10/fx-manager/native-view-attachment-replay02.log',
 'research/skirmish-20261017-renderer-mode/native-request-policy-owner-corrected-replay.log',
]
doc=dict(schema=1,scope=__doc__,status='bounded checkpoint; source integration continues',
 original_sha256='5d2548a24f0c6597008bd6c83e88bfa64f7d1e00e8e1e7126c4e10d20d160d26',
 v14_sha256='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1',
 source_pins=[dict(path=p,sha256=sha(ROOT/p))for p in files],
 evidence_pins=[dict(path=p,sha256=sha(ROOT/p))for p in evidence],
 tests=dict(full_client_selected=9,unrelated_ignored=2,scene_matrix=1,static_transport_send=1,
  renderer_witness=1,actual_headless_gpu=1,wgsl_validation_layout=1,exact_scalar_modules=6,exact_fx_manager_module=3,
  orientation_author_actual_source=3),
 native=dict(publication_rows=228,base_common_rows=204,flag14_rows=72,fifo_rows=30,orientation_rows=30,
  attachment_rows=120,attachment_executed_pins=61,physical_policy_rows=108,physical_policy_executed_pins=48,
  actual_windows_captures=2,armed_threads_per_capture=[73,73],native_writes_per_capture=[5,5],post_cutoff_probes_per_capture=[4,4]),
 qualifications=[
  'Shared source pins are this historical checkpoint; authorized peer edits may follow.',
  'Full client binary predates newest VWorld wrapper/terrain method tests; current source/test check12 covers those APIs.',
  'Orientation author binary may precede final declaration guard; peer exact-source bundle probe separately executed final guard.',
  'Native heap/child seams are scoped in each producer; no concrete child success inferred.',
  'Modern physical RAM projection deliberately corrects legacy prior-word shift; observed byte1 is no general default.',
  'Actual GPU regression exercises viewer-compatible shader/picking/order contracts, not a completed native scene binding or battle.',
  'Current shader materials lack authored specular/glossiness; no Gamebryo pixel parity.',
  'No positive Thing+C, VWorldAD,74FF50 or activation capability minted by this checkpoint.',
 ])
out=HERE/'view-backend-findings.json';out.write_text(json.dumps(doc,indent=2)+'\n')
print(json.dumps(dict(path=str(out),sha256=sha(out),source_files=len(files),evidence_files=len(evidence))))
