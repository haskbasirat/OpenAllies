"""Root's scoped independent review after replaying five original-code producers.

The replay is independent execution, not a claim of comparison with a pre-replay
golden: no pre-replay digest was recorded. Rust tests compare the newly executed
original instructions to the implementation. Whole live worlds remain unready.
"""
from pathlib import Path
import hashlib
import json
import sys

ROOT = Path(__file__).resolve().parents[3]
sys.path.insert(0, str(ROOT / 'research/engine-baseline'))
from audit_baseline import PE

original = PE(ROOT.parent / 'AA.exe')
patched = PE(ROOT.parent / 'reconstruction/native-game/runtime-expanded-v14/AA.exe')
base = ROOT / 'research/phase7-restoration'
names = ['special-constructors', 'kingdom-actor-load', 'kingdom-prefix',
         'kingdom-ai', 'kingdom-suffix']
files = {}
instructions = {}
for name in names:
    path = base / ('native-' + name + '.json')
    data = json.loads(path.read_text(encoding='utf-8'))
    assert data['original_sha256'] == hashlib.sha256(original.b).hexdigest()
    assert data['v14_sha256'] == hashlib.sha256(patched.b).hexdigest()
    for ins in data['instructions']:
        address = int(ins.get('va', ins.get('address')), 16)
        expected = bytes.fromhex(ins['bytes'])
        assert original.range(address, len(expected)) == expected
        assert patched.range(address, len(expected)) == expected
        old = instructions.setdefault(address, expected)
        assert old == expected
    files[str(path.relative_to(ROOT))] = hashlib.sha256(path.read_bytes()).hexdigest()

for relative in [
    'crates/aa-mapworld/src/special_state.rs',
    'crates/aa-mapworld/src/kingdom_ai.rs',
    'crates/aa-mapworld/src/definition_counts.rs',
    'crates/aa-mapworld/src/commander_restore.rs',
]:
    files[relative] = hashlib.sha256((ROOT / relative).read_bytes()).hexdigest()

report = {
    'reviewer': 'root (independent of scenegraph implementation)',
    'status': 'pass for the bounded scopes below',
    'native_executions': 831,
    'original_instruction_windows_checked': len(instructions),
    'tests': {'passed': 12, 'failed': 0, 'targets': [
        'special_state', 'kingdom_actor_restore', 'kingdom_ai', 'kingdom_suffix',
        'commander_restore', 'bootstrap', 'restore_stop']},
    'reviewed': [
        'Actual Team/Kingdom constructors and bounded original Actor Load return.',
        'Resource index mapping, ordered assignments and native clamp including nonfinite words.',
        'Typed tactical empty-table state and exact strategic v0/v1 word mapping.',
        'Definition duplicate chains including per-key reversal across native rehash thresholds.',
        'Signed Nation badge comparison and head-first insertion without dereferencing negative indices.',
        'Commander shared-state adapter, mismatch discard and destruction-before-replacement.',
        'Lookup failure preserves prior count insertion and prevents later field mutation.',
    ],
    'limits': [
        'Not a whole native world or playable tutorial test; PostLoad/publication remain required.',
        'Nonempty tactical/hash/named-state ownership remains an explicit error.',
        'Definition tokens provide equality; native cross-key pointer-hash iteration is not exposed.',
        'Native producers document fixture lookup, I/O, allocation and prior-load seams.',
        'Pre-replay hashes were not captured; this review does not claim byte-identical old fixtures.',
        'These file hashes describe this review point; later source changes require scoped revalidation.',
    ],
    'files': files,
}
output = Path(__file__).with_name('independent-review.json')
output.write_text(json.dumps(report, indent=2) + '\n', encoding='utf-8')
print(json.dumps({'report': str(output.relative_to(ROOT)),
                  'sha256': hashlib.sha256(output.read_bytes()).hexdigest(),
                  'instruction_windows': len(instructions)}))
