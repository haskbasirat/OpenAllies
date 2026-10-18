"""Bind the independently executed GPU regressions and reviewed static draw cache."""
from pathlib import Path
import hashlib,json,re
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
log=HERE/'independent-gpu-tests.log';text=log.read_text(encoding='utf-8-sig')
assert '5 passed; 0 failed' in text,text[-2000:]
assert 'authored_material_layers_keep_solid_parts_and_cutout_holes ... ok' in text
assert 'gpu_validation_is_scoped_and_failed_upload_preserves_scene ... ok' in text
captures=[]
for name,frames in [('crete',5),('stalingrad',3)]:
    p=ROOT/f'qa-output/phase6/water/{name}-final-native-source.json';row=json.loads(p.read_text())
    assert row['render_frames']==frames and row['draw_order_rebuilds']==1
    assert 'not isolated GPU time or in-game FPS' in row['timing_scope']
    captures.append({'path':p.relative_to(ROOT).as_posix(),'sha256':sha(p),'png_sha256':sha(p.with_suffix('.png')),
        'adapter':row['adapter'],'render_frames':frames,'draw_order_rebuilds':row['draw_order_rebuilds'],
        'first_encode_submit_ms':row['first_encode_submit_ms'],'repeated_encode_submit_mean_ms':row['repeated_encode_submit_mean_ms'],
        'producer':'material_render final frozen executable capture; JSON and image independently inspected by reviewer'})
files=['crates/aa-client/src/gpu.rs','crates/aa-client/src/main.rs','crates/aa-client/src/transparency.rs',
    'crates/aa-client/src/camera.rs','crates/aa-client/src/scene.wgsl',
    'research/phase6-render-cache/independent-gpu-tests.log','research/phase6-render-cache/record_review.py']
report={'schema':1,'passed':True,'reviewer':'orders_spatial','production_edits_by_reviewer':False,
    'scope':'Draw-command/render-bundle cache preserves the current recovered submission sequence, and headless repeated-frame instrumentation reports its actual limited timing scope.',
    'findings':[],'independent_gpu_tests':5,'test_command':['cargo','test','-p','aa-client','--release','gpu::','--','--include-ignored','--nocapture'],
    'source_sha256':{name:sha(ROOT/name)for name in files},'headless_witnesses':captures,
    'checks':[
        'Cache key uses all three direction binary32 bit patterns, exactly the values entering current native depth-key computation. Translation, viewport size and team color do not enter that key.',
        'On a miss, terrain commands and original source-ordered immediate commands precede the same recovered native transparent submission_order. Only adjacent commands with the same draw and contiguous instance indices are merged; their selected order is unchanged.',
        'Render bundle records identical pipeline/material/vertex/index bindings and instance ranges. Its camera uniform buffer is updated on every render, including warm-cache frames.',
        'Successful scene upload and pose updates invalidate direction and bundle together. New command/bundle state is committed only after GPU scope validation succeeds. Preflight invalid uploads/poses preserve the earlier view.',
        'Actual GPU tests verify unchanged-view cache reuse, orientation rebuilds, restoration of the previous order, pose invalidation, unchanged pixels after invalid updates, and preserved authored blend/cutout/depth behavior.',
        'Added existing-bundle RepColor test changes team color from blue to red without upload or pose change, requires ordering_rebuilds unchanged, and verifies actual output pixels change to red.',
        'Viewport resize uses the same color/depth formats and sample count as the bundle; the bundle holds resource handles rather than a stale target view.',
        'The headless CLI rejects zero frame count and render-frames without render-check. It renders the requested count at the same static camera, reports first and repeated encode/submit timings, and waits for final image readback before printing success.',
        'Final Crete and Stalingrad headless outputs report one draw-order build over five and three frames. Both final PNGs were independently visually inspected.',
    ],
    'limits':[
        'This preserves the renderer current command sequence. Original whole-engine traversal, generated-water sphere bounds and live rendering FPU state still have their separately documented limits.',
        'Timings include CPU encode/submit and error-scope handling; they exclude isolated GPU timing and complete gameplay. No FPS figure or controlled performance speedup is inferred.',
        'Capture measurements were collected during concurrent development and are functional reuse witnesses, not a controlled benchmark.',
        'Only the available local graphics adapter was exercised. No other GPU, laptop scaling session, live gameplay or network performance is claimed.',
    ]}
(HERE/'independent-review.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'passed':True,'gpu_tests':5,'sha256':sha(HERE/'independent-review.json')}))
