"""Read-only independent replay of the frozen map inventory/load-prefix scope."""
from pathlib import Path
from unittest.mock import patch
import hashlib, json, os, subprocess, sys

HERE = Path(__file__).resolve().parent
ROOT = HERE.parents[1]
sys.path.insert(0, str(HERE))
import native_load_oracle as native

def sha(path):
    return hashlib.sha256(path.read_bytes()).hexdigest()

visited = {}
original_machine = native.Machine
class TrackedMachine(original_machine):
    def __init__(self, payload):
        super().__init__(payload)
        self.u.hook_add(native.UC_HOOK_CODE, self.pin)
    def pin(self, _u, pc, size, _):
        if native.P.base <= pc < native.P.base + native.P.image_size:
            raw = native.P.range(pc, size)
            assert raw == native.V.range(pc, size), hex(pc)
            visited[pc] = raw.hex()

native.Machine = TrackedMachine
captured = []
with patch.object(Path, 'write_text', lambda self, text, *a, **kw: captured.append((self, text))):
    native.main()
assert len(captured) == 1 and captured[0][0].name == 'native-load-fixtures.json'
assert json.loads(captured[0][1]) == json.loads((HERE/'native-load-fixtures.json').read_text())
fixture = json.loads(captured[0][1])
env = dict(os.environ, AA_GAME_ROOT=str(ROOT.parent))
commands = [
    ['cargo', 'test', '-p', 'aa-mapworld', '--test', 'archive', '--test', 'bootstrap', '--test', 'native_load', '--test', 'local_corpus', '--', '--include-ignored', '--nocapture'],
    ['cargo', 'clippy', '-p', 'aa-mapworld', '--all-targets', '--', '-D', 'warnings'],
]
logs = []
for index, command in enumerate(commands):
    result = subprocess.run(command, cwd=ROOT, env=env, stdout=subprocess.PIPE, stderr=subprocess.STDOUT)
    log = HERE/f'independent-{index}.log'
    log.write_bytes(result.stdout)
    assert result.returncode == 0, result.stdout.decode(errors='replace')
    logs.append({'command':command, 'exit_code':result.returncode, 'log':log.name, 'sha256':sha(log)})
paths = [
    'crates/aa-mapworld/Cargo.toml', 'crates/aa-mapworld/src/archive.rs',
    'crates/aa-mapworld/src/inventory.rs', 'crates/aa-mapworld/src/bootstrap.rs',
    'crates/aa-mapworld/src/world.rs', 'crates/aa-mapworld/tests/archive.rs',
    'crates/aa-mapworld/tests/bootstrap.rs', 'crates/aa-mapworld/tests/native_load.rs',
    'crates/aa-mapworld/tests/local_corpus.rs', 'crates/aa-actors/src/loading.rs',
    'crates/aa-actors/src/registry.rs', 'research/phase6-mapworld/native_load_oracle.py',
    'research/phase6-mapworld/native-load-fixtures.json',
    'research/phase6-mapworld/independent_review.py',
]
report = {
    'schema':1, 'passed':True, 'reviewer':'orders_spatial', 'production_edits_by_reviewer':False,
    'scope':'Source-bound TGCK archive, indexed placement inventory, GThing/actor/world/identity prefixes and partial loaded-shell bootstrap. MAP_HEADER and new TEAMS/KINGDOMS readers are independently reviewed elsewhere.',
    'native_cases':{key:len(fixture[key]) for key in ['thing_cases','id_manager_cases','world_cases','actor_cases']},
    'native_visited_addresses_and_intercepted_entries':len(visited),
    'native_windows':[{'va':hex(a),'bytes':raw} for a,raw in sorted(visited.items())],
    'rust_tests':9, 'strict_clippy_passed':True, 'commands':logs,
    'source_sha256':{p:sha(ROOT/p) for p in paths}, 'findings':[],
    'integration_rebind':{
        'previous_bootstrap_sha256':'9496def42007e3b3114d21e977f5c77fac02ebf3e99cb80953ca137e0b9cfb13',
        'previous_world_sha256':'b4894c26dc8a474d220f76a11a3805d7d85db783bfb23dd3ed798c3453fe8df2',
        'review':'The bootstrap change is formatting only. WorldInventory adds typed Teams/Kingdoms, generation diagnostics and ownership projection; existing serialized world/ID/prefix parsing is unchanged, and rejected new chunks retain opaque input with a reason. New kingdom/team/ownership algorithms are outside this base review and assigned to visual_audit. The scoped native replay and all nine base tests were rerun against the integrated source.'
    },
    'checks':[
        'Every committed native prefix case reproduced in memory without overwriting producer fixtures; every observed original address matched v14 bytes, including boundary entry bytes whose body was intercepted.',
        'TGCK directory bounds, duplicate names, overlapping extents, decoded allocation budget, exact zlib output length and complete stream consumption are enforced.',
        'Indexed source record order and spans are retained; a rejected bounded record remains explicit with its raw bytes, and bytes outside the index cannot disappear silently.',
        'All 80 original RMP maps and 225851 placements retain prior independently extracted IDs, positions and spans; saved generations match and unsupported prefix count is zero.',
        'GThing versions 0..2 preserve raw UTF16/float words and native flag merge; Actor prefix gates owner references by definition flag bit23 and retains pre-transient-clearing state.',
        'Bootstrap checks resolved category against serialized class, supports only proved load shells, registers saved identity before field restoration without normal-create RNG/defaults, and explicitly retains pending subtype/publication work.',
        'Definition class and final flags use the shared typed native evaluator with source/template provenance; arbitrary definition strings are not treated as category or owner heuristics.',
    ],
    'limits':[
        'Container decoding and GObject byte prefix use direct source-layout/corpus evidence, not a claim that the entire original archive or GObject loader body was emulated.',
        'The raw fallback for equal packed/decoded lengths follows observed TGCK source encoding; this is not a general damaged-archive repair guarantee.',
        'Native oracle mocks CString storage and virtual transport; actor prefix also intercepts the already-decoded GObject base. GWORLD fixtures begin after cleanup and end before subtype/world initialization.',
        'Restored base entries are incomplete until documented pending steps and required world/player ordering are completed. No full actor, campaign, saved-match or live gameplay restoration is claimed.',
        'Teams/KINGDOMS/ownership algorithms added to WorldInventory require their separate independent review; the whole world.rs source pin does not extend this base gate to those algorithms.',
    ],
}
(HERE/'independent-review.json').write_text(json.dumps(report, indent=2)+'\n')
print(json.dumps({'passed':True, 'tests':9, 'native_cases':sum(report['native_cases'].values()), 'sha256':sha(HERE/'independent-review.json')}))
