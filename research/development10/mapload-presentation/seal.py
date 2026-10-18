"""Seal/verify the bounded first-battle-frame source checkpoint.

No native readiness is inferred from metadata checks or the aspect test.
Root owns the actual linked application/GPU probe after this source freeze.
"""
from pathlib import Path
import hashlib, json, sys

ROOT=Path(__file__).resolve().parents[3]
HERE=Path(__file__).resolve().parent
DEST=HERE/'findings.json'
def digest(path):return hashlib.sha256((ROOT/path).read_bytes()).hexdigest()
def pin(path):return {'path':path,'sha256':digest(path)}
owned=['crates/aa-client/src/frontend.rs','crates/aa-client/src/tgi_frontend.rs','crates/aa-client/src/battle_presentation.rs']
before='backups/source-checkpoint-20261018-mapload/settlement/'
old=(ROOT/(before+'tgi_frontend.rs')).read_text()
new=(ROOT/owned[1]).read_text()
def body(text):
    start=text.index('    pub fn frame(')
    return text[text.index('        if !self.fonts_installed {',start):text.index('    fn command(',start)]
assert body(old)==body(new).replace('if clear_background && matches!(self.page, Page::Credits | Page::Loading)',
                                   'if matches!(self.page, Page::Credits | Page::Loading)')
helper=(ROOT/owned[2]).read_text()
isolated=(HERE/'aspect_fit_exact.rs').read_text()
assert isolated=='extern crate egui;\n'+helper[helper.index('fn aspect_fit('):]
doc={
 'scope':'Frontend/native battle renderer presentation wiring, not complete battle startup or combat acceptance',
 'owned_source':list(map(pin,owned)),
 'exact_before_backups':[pin(before+'frontend.rs'),pin(before+'tgi_frontend.rs')],
 'source_changes':[
  'Allocate the actual fixed initial window target before pumping native camera/frontend RPC.',
  'Refresh the actual same-size Viewport receipt after scene upload changes renderer epoch.',
  'Submit a nonblocking read on the same retained SimulationThread; use actual VWorld scene renderer receipt, current normalized camera, World/root relationship and real Viewport.',
  'Return only sealed immutable camera packet/weak witnesses; native World and Directory remain on SimulationThread.',
  'Draw with SceneRenderer.render_native and paint its texture behind the unchanged source TGI forest.',
  'Share the exact prior TGI input/update/event/layout/paint algorithm; only legacy Loading black clear is omitted when an actual battle frame was drawn.',
  'Keep actual Loading TGI until native AppState selects Game; no Playing status, AD/task/Loading/Sim-ready writes.',
  'Keep first target dimensions fixed; letterbox subsequent window sizes rather than silently change native projection aspect.',
  'Emit a one-time actual battle native frame presented log only after render_native succeeds, with selected map label, World revision, native init word B8 and camera pixel dimensions.'
 ],
 'source_equivalence':{
  'tgi_common_frame_body_identical_except_clear_predicate':True,
  'prior_frame_body_sha256':hashlib.sha256(body(old).encode()).hexdigest(),
  'qualification':'exact source comparison; no new native TGI callback execution claimed'
 },
 'validation':{
  'client_metadata':{'result':'PASS','elapsed_seconds':14.02,'log':pin('research/development10/mapload-ui-check02.log')},
  'client_tests_metadata':{'result':'PASS','elapsed_seconds':9.00,'log':pin('research/development10/mapload-ui-tests-check03.log')},
  'isolated_exact_aspect_function':{'result':'1/1 PASS','source':pin('research/development10/mapload-presentation/aspect_fit_exact.rs'),
   'binary':pin('research/development10/mapload-presentation/aspect-fit-tests.exe'),
   'log':pin('research/development10/mapload-presentation/aspect-fit-tests.log'),
   'qualification':'exact production aspect-fit function and its test only; no fabricated Camera Snapshot or VWorld owner'},
  'first_metadata_predecessor':{'result':'blocked only by Builder controller E0505, repaired before final checks',
   'log':pin('research/development10/mapload-ui-check01.log')}
 },
 'shared_context_snapshot_only':list(map(pin,['crates/aa-client/src/battle_controller.rs','crates/aa-client/src/battle_scene_startup.rs',
  'crates/aa-client/src/battle_view_startup.rs','crates/aa-client/src/main.rs'])),
 'not_claimed':['Actual native map first-frame marker observed','linked current application probe','full VWorld startup success',
  'Loading/UserReady completion','Admin304/315 unfreeze success','combat/AI readiness','native camera resize operation'],
 'initial_audit_barriers':['Product skirmish_session never called render_native.',
  'No product caller prepared native camera viewport or minted same-VWorld camera Frame.',
  'Legacy TgiFrontend Page::Loading black clear persisted after actual Game forest was selected.'],
 'remaining_validation':'Root linked application probe must reach actual scene/camera receipts and show the first-frame marker. Reached native startup failures remain authoritative; no viewer fallback.',
}
encoded=(json.dumps(doc,indent=1)+'\n').encode()
if '--capture' in sys.argv:
    assert not DEST.exists(),'immutable findings already exists'
    DEST.write_bytes(encoded)
else:assert DEST.read_bytes()==encoded,'source/checkpoint successor differs'
print(json.dumps({'findings_sha256':hashlib.sha256(encoded).hexdigest(),'owned_source_files':len(owned),'actual_first_frame_observed':False}))
