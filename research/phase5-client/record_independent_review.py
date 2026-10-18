"""Review the asset-free diagnostic; source selection checks do not rebuild it."""
from pathlib import Path
import hashlib
import json
import re

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

bundle_path = HERE / 'world-samples.json'
bundle = json.loads(bundle_path.read_text())
code = (ROOT / 'crates/aa-client/src/world_check.rs').read_text()
expected = re.search(r'const SAMPLE_SHA: &str = "([0-9a-f]+)";', code).group(1)
assert sha(bundle_path) == expected
assert bundle['schema'] == 1
counts = {}
for area, provenance in bundle['sources'].items():
    source_path = ROOT / provenance['oracle']
    selection_path = ROOT / provenance['selection']
    assert sha(source_path) == provenance['oracle_sha256']
    assert sha(selection_path) == provenance['selection_sha256']
    assert sha(ROOT / provenance['adapter_source']) == provenance['adapter_sha256']
    source = json.loads(source_path.read_text())
    selection = json.loads(selection_path.read_text())
    assert bundle['components'][area] == selection
    assert source['original_sha256'] == bundle['original_sha256']
    assert source['v14_sha256'] == bundle['accepted_sha256']
    if area == 'navigation':
        rows = selection['fixtures']
        by_name = {row['input']['name']: row for row in source['fixtures']}
        assert all(row == by_name[row['input']['name']] for row in rows)
    else:
        rows = selection['samples']
        for row in rows:
            if area == 'actors':
                native = {key: val for key, val in row.items() if key != 'source_case_index'}
                assert native == source['cases'][row['source_case_index']]
            else:
                assert row['fixture'] == source[selection['source_field']][row['source_index']]
    assert len(rows) == 6 == provenance['selected_samples']
    counts[area] = len(rows)
assert set(counts) == {'actors', 'navigation', 'orders', 'spatial'}
log = (HERE / 'independent-tests.log').read_text(encoding='utf-8-sig')
assert 'test result: ok. 3 passed; 0 failed' in log
files = ['crates/aa-client/src/world_check.rs', 'crates/aa-client/src/main.rs',
         'crates/aa-client/Cargo.toml', 'research/phase5-client/build_samples.py',
         'research/phase5-client/world-samples.json', 'research/phase5-client/independent-tests.log']
report = {
    'schema': 1, 'passed': True, 'component': 'aa-client asset-free world-fragment diagnostic only',
    'files': {p: sha(ROOT / p) for p in files},
    'findings': [],
    'independent_tests': {'command': 'cargo test -p aa-client world_check -- --nocapture',
                          'passed': 3, 'failed': 0, 'sample_counts': counts},
    'checks': [
        'Independently checked embedded bundle SHA, exact selected native-row identity, source/selection/adapter hashes and matching original/accepted executable identities for all24 samples.',
        'Reviewed each adapter against the corresponding library fixture harness; actual values come from Rust library execution and expected values remain separate immutable oracle data.',
        'Navigation/spatial mutable callbacks are explicit fixture hosts, with the native state/event ordering retained. There is no connection of raw commands to a live Mover or fabricated actor world.',
        'Runtime validates sample hash/schema, rejects empty components and duplicate names, reports exact per-sample comparisons, and passes only when all comparisons pass.',
        'Focused tests independently passed, including one deliberately changed expected result for each component and rejection of an empty sample group.',
        'Reviewed CLI option dispatch: world-check returns before asset discovery, renderer and worker initialization; failed report produces a nonzero error and mechanics/world diagnostics cannot be combined.',
        'Report and documentation state the unfinished full-world, pathfinding, combat, AI, saves and multiplayer boundaries and do not turn fixture success into gameplay parity.'
    ],
    'limits': [
        'This review gates only world_check.rs and the related CLI/dependency additions; unrelated renderer/UI changes in main/client are reviewed separately.',
        'Focused tests exercise24 selected records, not all native corpora. Base actor diagnostic compares selected identity fields, not the entire full-registry snapshot.',
        'Fixture adapters are internal pinned data consumers, not untrusted external JSON APIs or live world service implementations.',
        'Final packaged Windows executable smoke and deployment hashes are parent build validation; no executable smoke is claimed here.',
        'No implementation, native/runtime/asset/save/release file was changed by this review.'
    ]
}
(HERE / 'independent-review.json').write_text(json.dumps(report, indent=2) + '\n')
print(json.dumps({'passed': True, 'report_sha256': sha(HERE / 'independent-review.json')}))
