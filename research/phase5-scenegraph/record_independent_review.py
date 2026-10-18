"""Record the independently checked authored-visibility integration boundary."""
from pathlib import Path
import hashlib
import json

ROOT = Path(__file__).resolve().parents[2]
OUT = Path(__file__).parent
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

report = json.loads((OUT / 'visibility-audit.json').read_text())
maps = report['maps']
for name, row in maps.items():
    path = ROOT.parent / f'reconstruction/game/data/map-actors/{name}.json'
    assert sha(path) == row['placements_sha256'], name
logs = []
for name, command, expected in [
    ('independent-visibility-tests.log', 'cargo test -p aa-client visibility::tests -- --include-ignored --nocapture', '3 passed; 0 failed; 0 ignored'),
    ('independent-inspection-tests.log', 'cargo test -p aa-client inspection::tests', '6 passed; 0 failed; 1 ignored'),
]:
    data = (OUT / name).read_text()
    assert expected in data, name
    logs.append({'command': command, 'result': expected, 'log': name, 'sha256': sha(OUT / name)})
files = [
    'crates/aa-client/src/visibility.rs', 'crates/aa-client/src/loading.rs',
    'crates/aa-client/src/inspection.rs', 'crates/aa-client/src/authored_rules.rs',
    'crates/aa-rules/src/actors.rs', 'crates/aa-rules/src/sources.rs',
    'crates/aa-rules/examples/phase5_visibility.rs',
    'research/phase5-scenegraph/visibility-audit.json',
    'research/phase5-scenegraph/record_independent_review.py',
]
out = {
    'schema': 1, 'passed': True, 'reviewer': 'orders_spatial',
    'scope': 'Source-bound initial authored Flags.invisible and map-only preview integration.',
    'production_edits_by_reviewer': False, 'findings': [], 'tests': logs,
    'final_integration_rebind': {
        'previous_loading_sha256': 'bb4fa5f8db218762e541211509cc7cd613b467914f6adf0fd70d8076c9033b0d',
        'current_loading_sha256': sha(ROOT / 'crates/aa-client/src/loading.rs'),
        'review': 'Re-read final loading.rs after material integration. Visibility still runs immediately after load_scene and before model correction, draw grouping and SceneObjects construction. Both draw grouping and picking skip missing model_key. Model-only loading still bypasses visibility. The final material mapper owns native render-state translation and does not mutate placement visibility or source records; warning wording now accurately says planar placement facing.',
        'validation_scope': 'Manual integration re-review and refreshed source pin only; the independent visibility/inspection test logs above remain the original executions. Final aggregate workspace/render validation is recorded separately by root.',
    },
    'verified_map_inputs': len(maps),
    'source_records': sum(row['source_records'] for row in maps.values()),
    'authored_hidden_records': sum(row['authored_hidden'] for row in maps.values()),
    'maps_with_unresolved_ids': sum(bool(row['unresolved']) for row in maps.values()),
    'source_sha256': {name: sha(ROOT / name) for name in files},
    'checks': [
        'Flags.invisible is projected through the existing bounded original/v14 template resolver, with field source span and SHA256 retained.',
        'Literal true hides; explicit false overrides inherited true. Missing, ambiguous, dynamic or unsupported fields preserve geometry and report unresolved status.',
        'ActorCatalog marks duplicate case-insensitive Thing IDs unsupported before visibility consumes them.',
        'Filter changes only prepared instance model_key; source placements, models, transforms and terrain remain available unchanged. Crete test covers324 hidden and0 unresolved instances.',
        'Map loading applies visibility before both draw grouping and SceneObjects construction; both paths skip absent model_key.',
        'Model-only loading bypasses the visibility filter, so source models remain inspectable.',
        'No object-name, model-name or path exclusions were introduced by visibility.',
        'Rule-loading failure warns and preserves geometry. One source/store-bound OnceLock caches catalog or failure per loader.',
        'All80 map placement inputs still match the independent audit hashes.',
    ],
    'limits': [
        'This is initial authored visibility, not native runtime reveal/fog/controller semantics.',
        '62 maps have unresolved Thing IDs; the implementation preserves their geometry rather than guessing.',
        'The six inspection regression tests passed; the pre-existing ignored native-map inspection test was not executed in this review.',
        'No interactive GUI click or new pixel-equivalence claim is made by this review.',
        'The final integrated loading.rs has been re-reviewed and rebound without production edits; material pipeline details are reviewed separately.',
    ],
}
(OUT / 'independent-review.json').write_text(json.dumps(out, indent=2) + '\n')
print(json.dumps({'passed': True, 'maps': len(maps), 'sha256': sha(OUT / 'independent-review.json')}))
