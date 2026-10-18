"""Bind independent material review to tested sources, native cases and assets."""
from pathlib import Path
import hashlib
import json

OUT = Path(__file__).parent
ROOT = OUT.resolve().parents[1]
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

native = json.loads((OUT / 'independent-native-rerun.json').read_text())
assert native['passed']
assert native['crete_mesh_properties_matched'] == 223
tests = []
for name, command, expected in [
    ('independent-gpu-tests.log', 'cargo test -p aa-client --release gpu::tests -- --include-ignored --nocapture', '3 passed; 0 failed; 0 ignored'),
    ('independent-apply-tests.log', 'cargo test -p aa-client --release source_bound_render -- --nocapture', '1 passed; 0 failed; 0 ignored'),
    ('independent-metadata-tests.log', 'python research/texture-phase2/test_model_materials.py', 'Ran 9 tests'),
]:
    text = (OUT / name).read_text()
    assert expected in text, name
    if name == 'independent-metadata-tests.log':
        assert '\nOK' in text, name
    tests.append({'command': command, 'result': expected, 'log': name, 'sha256': sha(OUT / name)})
files = [
    'crates/aa-client/src/gpu.rs', 'crates/aa-client/src/scene.wgsl',
    'crates/aa-client/src/loading.rs', 'crates/aa-client/src/model_fixes.rs',
    'crates/aa-assets/src/types.rs', 'assets-cache/model-materials.json',
    'research/texture-phase2/export_model_materials.py',
    'research/texture-phase2/test_model_materials.py',
    'research/phase5-material-render/original_render_state.py',
    'research/phase5-material-render/native-render-state-vectors.json',
    'research/phase5-material-render/native-render-state-assembly.txt',
    'research/phase5-material-render/findings.txt',
    'research/phase5-material-render/independent_native.py',
    'research/phase5-material-render/independent-native-rerun.json',
    'research/phase5-material-render/record_independent_review.py',
]
report = {
    'schema': 1, 'passed': True, 'reviewer': 'orders_spatial',
    'scope': 'Initial authored RGB blend, alpha test and ordinary Z-buffer state in the opaque Rust preview target.',
    'production_edits_by_reviewer': False, 'findings': [],
    'resolved_review_issues': [
        'Destination-alpha-dependent factors could not match the opaque viewport. Runtime now rejects native factors8..10 for blended materials, and all1847models/11035meshrows were audited: zero retained nonteam blended rows need these factors.',
        'Native enum table execution started midconstructor with supplied registers. Original preceding instructions now pin EBX0, EBP2 and EDI3, and the fixture declares its allocation/registration boundary.',
        'Native depth fixture now explicitly declares ordinary Z mode; fallback3/3 is pinned through constructor/type evidence, and the byte range includes the complete ret8 instruction.',
    ],
    'tests': tests,
    'native_cases_reexecuted': {'alpha': native['alpha_cases'], 'depth': native['depth_cases']},
    'source_models_reparsed': native['crete_source_models_reparsed'],
    'source_mesh_properties_matched': native['crete_mesh_properties_matched'],
    'source_sha256': {name: sha(ROOT / name) for name in files},
    'checks': [
        'The sidecar propagates inherited NiAlphaProperty/NiZBufferProperty fields with original/converted source hashes and per-mesh identity. Source property values independently match all223 exported meshes across127 Crete NIF inputs.',
        'Native descriptor lookup tables and532alpha/32depth state emissions were independently re-executed from the pinned original instructions; device emission leaves only are intercepted.',
        'Pipeline RGB factor mapping preserves the native enum mapping for supported factors0..7; authored blending and effective depth writing remain independent.',
        'All eight alpha comparison codes reach WGSL with an explicit disabled mode. Threshold0 still tests when enabled; material opacity alone no longer invents blend enable.',
        'RepColor handling retains the previous source-backed alpha suppression and team-color-mask behavior.',
        'Hardware pixels verify city ONE/SRC_ALPHA solid RGB, transparent zero alpha, strict zero-threshold cutout, solid cutout and independently enabled/disabled blended depth writes.',
        'The source-bound metadata application regression retains geometry and rejects invalid enum values.',
        'Opaque viewport presentation is retained deliberately; errors do not replace the prior uploaded scene.',
    ],
    'limits': [
        'No full original-match visual equivalence is claimed.',
        'Transparent mesh/per-triangle sorting, alpha-controller animation and dynamic material overrides remain missing.',
        'Optional W-buffer mode and native framebuffer-alpha semantics are unsupported.',
        'Lighting, color space, filtering, alpha quantization and float comparison are not claimed bit-identical to Direct3D9.',
        'Legacy sidecars without render_state retain legacy fallback behavior; the updated sidecar is required for these fixes.',
    ],
}
(OUT / 'independent-review.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'passed': True, 'sha256': sha(OUT / 'independent-review.json')}))
