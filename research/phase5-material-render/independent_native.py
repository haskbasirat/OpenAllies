"""Re-execute supplied native state cases and independently audit corpus bounds."""
from pathlib import Path
import collections
import hashlib
import json
import sys

OUT = Path(__file__).parent
ROOT = OUT.resolve().parents[1]
sys.path.insert(0, str(OUT))
import original_render_state as native

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

fixture = json.loads((OUT / 'native-render-state-vectors.json').read_text())
for name, row in fixture['ranges'].items():
    start, end = int(row['start'], 16), int(row['end'], 16)
    a = native.original.range(start, end - start)
    assert a == native.modified.range(start, end - start), name
    assert hashlib.sha256(a).hexdigest() == row['sha256'], name
machine = native.RenderMachine()
assert machine.tables == fixture['tables']
for row in fixture['alpha']:
    assert machine.alpha(row['flags'], row['threshold']) == row['calls'], row
for row in fixture['depth']:
    assert machine.depth(row['flags'], row['function']) == row['calls'], row

metadata = ROOT / 'assets-cache/model-materials.json'
document = json.loads(metadata.read_text())
factors = collections.Counter()
unsupported = []
meshes = retained = 0
for model, entry in document['models'].items():
    for row in entry['corrections']:
        meshes += 1
        state = row['render_state']
        if row['skip'] or row['team_color_mask']:
            continue
        retained += 1
        if not state['alpha_flags'] & 1:
            continue
        source = (state['alpha_flags'] >> 1) & 15
        destination = (state['alpha_flags'] >> 5) & 15
        factors[f'{source},{destination}'] += 1
        if source >= 8 or destination >= 8:
            unsupported.append({'model': model, 'mesh': row['mesh_index'],
                                'source': source, 'destination': destination})
assert not unsupported, unsupported[:10]

# Independent property traversal of the 127 NIF inputs referenced by Crete.
# This checks the new fields, not only the previous exporter tag regressions.
sys.path.insert(0, str(ROOT.parent / 'reconstruction/tools'))
from convert_models import NifFormat, ORIGINAL, string
from convert_models_animations import read_nif
crete = json.loads((OUT / 'crete-materials.json').read_text())
property_rows = 0
for model in crete['models']:
    source = ORIGINAL / model['source']
    assert sha(source) == model['source_sha256']
    tree = read_nif(source)
    original_states = {}
    def walk(node, hierarchy=(), inherited=None):
        if not isinstance(node, NifFormat.NiAVObject):
            return
        hierarchy += (string(node.name),)
        properties = dict(inherited or {})
        for prop in node.properties:
            if prop is not None:
                properties[type(prop).__name__] = prop
        if isinstance(node, NifFormat.NiTriBasedGeom) and node.data:
            alpha = properties.get('NiAlphaProperty')
            depth = properties.get('NiZBufferProperty')
            original_states.setdefault(hierarchy, []).append({
                'alpha_flags': int(alpha.flags) if alpha else 236,
                'alpha_threshold': int(alpha.threshold) if alpha else 0,
                'zbuffer_flags': int(depth.flags) if depth else 3,
                'zbuffer_function': int(depth.function) if depth else 3,
            })
        for child in getattr(node, 'children', []):
            walk(child, hierarchy, properties)
    for root in tree.roots:
        walk(root)
    for row in document['models'][model['source']]['corrections']:
        matches = original_states[tuple(row['hierarchy'])]
        assert len(matches) == 1, (model['source'], row['hierarchy'])
        assert row['render_state'] == matches[0], (model['source'], row['hierarchy'])
        property_rows += 1

result = {
    'schema': 1, 'passed': True, 'reviewer': 'orders_spatial',
    'method': 'Re-executed original x86 cases in isolated Unicorn through the supplied producer class; no fixture regeneration or source-game mutation.',
    'preconditions': fixture['preconditions'],
    'alpha_cases': len(fixture['alpha']), 'depth_cases': len(fixture['depth']),
    'models': len(document['models']), 'meshes': meshes, 'retained_non_team_meshes': retained,
    'crete_source_models_reparsed': len(crete['models']), 'crete_mesh_properties_matched': property_rows,
    'blend_factor_pair_counts': dict(factors), 'unsupported_destination_alpha_rows': unsupported,
    'source_sha256': {str(p.relative_to(ROOT)): sha(p) for p in [
        metadata, OUT / 'original_render_state.py', OUT / 'native-render-state-vectors.json',
        OUT / 'independent_native.py', OUT / 'crete-materials.json']},
    'limits': ['Original SetRenderState leaves are observed, not actual Direct3D rasterization.',
               'Opaque viewport alpha, no transparent sorting, no dynamic renderer override or W-buffer equivalence is inferred.'],
}
(OUT / 'independent-native-rerun.json').write_text(json.dumps(result, indent=2) + '\n')
print(json.dumps({'passed': True, 'alpha': result['alpha_cases'], 'depth': result['depth_cases'],
                  'unsupported_corpus_rows': len(unsupported),
                  'sha256': sha(OUT / 'independent-native-rerun.json')}))
