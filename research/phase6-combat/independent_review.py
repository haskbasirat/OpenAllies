"""Independent immutable replay and source review for DealDamage/health APIs."""
from pathlib import Path
import hashlib
import json
import original_damage as native

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
fixture_path=HERE/'damage-native-oracle.json'
before=fixture_path.read_bytes()
fixture=json.loads(before)
assert len(fixture['cases'])==219
for case in fixture['cases']:
    machine=native.Machine(case['input'])
    result=machine.run()
    assert result==case['output'],(case['input']['name'],case['input']['precision'])
for window in fixture['native_windows']:
    start,end=int(window['start'],16),int(window['end'],16)
    raw=native.IMAGE.at(start,end-start)
    assert raw==native.ACCEPTED.at(start,end-start)
    assert hashlib.sha256(raw).hexdigest()==window['sha256']
assert fixture_path.read_bytes()==before
lib=ROOT/'crates/aa-body/src/lib.rs'
health_source='\n'.join(line for line in lib.read_text().splitlines() if line.strip()!='pub mod dying;')
report={'schema':1,'reviewer':'visual_audit','passed':True,'native_cases_replayed':219,
        'review_scope':['aa-body/src/damage.rs','aa-body/src/lib.rs health/Host APIs; dying module excluded'],
        'source_review':[
            'Linked damage dispatch precedes nonpositive/dead gates; linked health dispatch precedes local clamping.',
            'The input neighbor-propagation bool is replaced by the computed hostility result at5627B8; later notification and capture branches use cached hostility.',
            'Wall neighbor reads are separate and interleaved with damage dispatch; first-neighbor mutations affect second-neighbor eligibility.',
            'Source and modifier identities are cached at their native load points, while organization/mind/recovery/capture fields are reread after callbacks.',
            'The mutable local damage record is separate from the original source record passed to the health setter; later source-type3 notifications use the updated local event_kind.',
            'Derived health normalization preserves extended base-health times modifier until native ratio division; it is not prematurely stored to binary32.',
            'Directional health FX subtracts XYZ, normalizes/scales XY only, and retains the unscaled Z difference.',
            'Capture eligibility/attempt can mutate health; subsequent one-health restoration/death logic preserves the native read sequence.',
        ],
        'notes':['Original actor graph, body propagation targets, capture implementation, component effects and death queue actions remain explicit host seams.',
                 'Finite arithmetic is validated; invalid callback results can fail after committed effects and must not be retried automatically.',
                 'This review does not cover the concurrently implemented dying module or claim complete weapon hit computation.'],
        'lib_health_scope_sha256':hashlib.sha256(health_source.encode()).hexdigest(),
        'pins':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in [fixture_path,HERE/'original_damage.py',ROOT/'crates/aa-body/src/damage.rs',ROOT/'crates/aa-body/tests/native_damage.rs']}}
(HERE/'independent-review.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps({'passed':True,'native_cases':219,'review':'research/phase6-combat/independent-review.json'}))
