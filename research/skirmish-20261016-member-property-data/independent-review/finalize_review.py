from pathlib import Path
import hashlib
import json

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[2]
AUTHOR = HERE.parent
def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()
actual = json.loads((HERE / 'actual-source-replay.json').read_text())
expected = json.loads((AUTHOR / 'actual-source-data.json').read_text())
assert actual == expected
known = {(s['path'].lower(), s['sha256']) for s in actual['sources']}
counts = []
for row in actual['precisions']:
    data = row['data']
    assert row['definitions'] == len(data) == 1084
    assert row['nonempty'] == 0
    assert len({r['id'].lower() for r in data}) == 1084
    assert all(not r['properties'] and r['sources'] for r in data)
    for r in data:
        for source in r['sources']:
            assert (source['path'].lower(), source['sha256']) in known
            assert 0 <= source['span']['start'] <= source['span']['end']
    counts.append({'precision': row['precision'], 'definitions': len(data),
                   'nonempty': 0, 'element_present': sum(r['element_present'] for r in data),
                   'all_provenance_matches_catalog_source_hashes': True})
assert {c['precision'] for c in counts} == {'Single24', 'Double53', 'Extended64'}
native = json.loads((HERE / 'native-replay-summary.json').read_text())
assert native['fresh_rows'] == 72 and native['exact_author_fixture_match']
log = (HERE / 'rust-tests.log').read_text(encoding='utf-8-sig')
assert '4 passed; 0 failed; 0 ignored' in log
frozen = {
 'crates/aa-rules/src/native_member_property_data.rs': 'd776e7a9723515eaf95a2a53b637946885910a23945a7b430ad86fab04ca8607',
 'crates/aa-rules/tests/native_member_property_data.rs': 'b7f66b14f1e16b3f2179f6e36298688194bb701fa7bb7696cf73763cfd5cddfb',
 'research/skirmish-20261016-member-property-data/native_member_property_data.py': 'a580b45baa80a2eb2262a93db07382e1020ea9b9e205a8147b88c055f2477553',
 'research/skirmish-20261016-member-property-data/native-member-property-data.json': '602d2964bfdc1373f5a63a8ebd0c6e5d3c7938c33fa7cc3c4cfcebe039ea41bf',
 'research/skirmish-20261016-member-property-data/actual-source-data.json': 'df62dd757965a535e9430570879a32c79261fc71922ef33c3e7fe2d160d9d6e1'}
for path, pin in frozen.items():
    assert sha(ROOT/path) == pin, path
artifacts = []
for p in sorted(HERE.iterdir()):
    if p.is_file() and p.name != 'independent-review.json':
        artifacts.append({'path': str(p.relative_to(ROOT)), 'bytes': p.stat().st_size,
                          'sha256': sha(p)})
doc = {'schema': 1, 'reviewer': 'sdk_native_geometry_review', 'accepted': True,
       'blocking_findings': [], 'production_edits': [], 'all_reviewer_commands_reaped': True,
       'scope': 'Independent source/harness review and fresh replay; no independently constructed native parser oracle or playable initialization claim.',
       'native_replay': native, 'rust_tests': {'passed': 4, 'failed': 0, 'ignored': 0},
       'actual_source_census': counts, 'actual_capture_equals_author_structurally': True,
       'actual_source_files': len(actual['sources']),
       'same_layout_api_precondition': 'Documented, not authenticated by aa-rules public method. The reviewed runtime path supplies the same Catalog-owned cached arena.',
       'commands': [
         'python research/skirmish-20261016-member-property-data/independent-review/replay_native.py',
         'python research/skirmish-20261016-member-property-data/independent-review/audit_static.py',
         'CARGO_TARGET_DIR=target/s-graphics-reference AA_GAME_ROOT=<original root> AA_MEMBER_PROPERTY_DATA_CAPTURE=<absolute review capture> cargo test -p aa-rules --test native_member_property_data --release --locked --offline -j4 -- --include-ignored --nocapture',
         'python research/skirmish-20261016-member-property-data/independent-review/finalize_review.py'],
       'frozen_author_pins_verified': frozen, 'artifacts': artifacts}
with (HERE/'independent-review.json').open('x', encoding='utf-8', newline='\n') as f:
    f.write(json.dumps(doc, indent=2)+'\n')
print(json.dumps({'accepted': True, 'native_rows': 72, 'rust_passed': 4,
                  'census': counts, 'review_sha256': sha(HERE/'independent-review.json')}))
