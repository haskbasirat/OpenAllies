from pathlib import Path
import hashlib, json, shutil

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]


def digest(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()


if (HERE / 'findings.json').exists():
    raise SystemExit('Checkpoint is already sealed; do not overwrite historical findings.')

fixture = json.loads((HERE / 'native-hero-postpass.json').read_text())
assert len(fixture['rows']) == 411 and len(fixture['instructions']) == 78
assert fixture['producer_sha256'] == digest(HERE / 'native_hero_postpass.py')
assert '2 passed; 0 failed' in (HERE / 'exact-tests03.log').read_text()
assert '4 passed; 0 failed' in (HERE / 'existing-mapworld-regression03.log').read_text()
assert 'Finished `release`' in (HERE / 'check-lib-final.log').read_text()

source_paths = [
    'crates/aa-runtime/src/fresh_organization_completion.rs',
    'crates/aa-runtime/src/fresh_organization_completion_tests.rs',
    'crates/aa-mapworld/src/fresh_creation.rs',
    'crates/aa-mapworld/tests/fresh_creation.rs',
]
snapshots = HERE / 'source-checkpoint'
snapshots.mkdir()
source_records = []
for name in source_paths:
    original = ROOT / name
    saved = snapshots / name
    saved.parent.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(original, saved)
    source_records.append({'path': name, 'snapshot': str(saved.relative_to(HERE)).replace('\\', '/'), 'sha256': digest(original)})

exports = [line for line in (ROOT / 'crates/aa-runtime/src/lib.rs').read_text().splitlines() if 'pub mod fresh_organization_completion;' in line]
assert len(exports) == 1
(snapshots / 'lib-export.txt').write_text(exports[0] + '\n')

artifacts = [
    'native_hero_postpass.py', 'native-hero-postpass.json',
    'native-write.log', 'native-replay.log', 'native-4fa150-instructions.txt',
    'exact-hero-tests.rs', 'exact-hero-tests.exe', 'exact-tests-build03.log', 'exact-tests03.log',
    'existing-mapworld-regression03.log', 'check-lib-final.log',
    'integration-spec.md', 'README.md', 'seal_checkpoint.py',
]
records = [{'path': name, 'sha256': digest(HERE / name)} for name in artifacts]
external = [
    'research/phase8-conversion/native-fresh-creation.json',
    'research/phase8-ai/native_machine.py',
]
external_records = [{'path': name, 'sha256': digest(ROOT / name)} for name in external]
findings = {
    'schema': 1,
    'status': 'portable constituents accepted by author; concrete same-world integration pending',
    'ordinary': 'Unchanged existing create_organization tail extracted once; old entry delegates; actual Params+51 getter invoked only immediately before each reached nonleader spawn.',
    'hero': 'Native4FA3D0 postpass after explicit4FA150 child result; same captured/current owner order and partial error/mutation prefixes.',
    'native': {'rows': 411, 'executions': 822, 'executed_address_byte_pins': 78, 'images': ['original', 'v14'], 'precisions': ['Single24', 'Double53', 'Extended64'],
               'explicit_children': ['4FA150 creation result', 'member virtual108 current owners', '4E2A60 recruit query', '57B830 hero attachment', '4FA4A0..4FA54F localized notification subtree']},
    'execution': {'exact_production_module_tests': '2/2 pass', 'existing_mapworld_fresh_creation_regressions': '4/4 pass', 'runtime_production_check': 'pass', 'new_runtime_unit_link': False},
    'required_positive_children': ['actual recruit-table ownership/query', '57B830 hero insertion/refresh', 'actual localized formatted notification', 'recursive same-world member creation and teardown'],
    'not_claimed': ['fully initialized Staged World', 'natural mission organization completion', 'native notification or positive hero child implementation', 'raw pointer production identity domain'],
    'source': source_records,
    'artifacts': records,
    'external_evidence': external_records,
}
(HERE / 'findings.json').write_text(json.dumps(findings, indent=2) + '\n')
print(json.dumps({'findings_sha256': digest(HERE / 'findings.json'), 'source_files': len(source_records), 'artifacts': len(records)}))
