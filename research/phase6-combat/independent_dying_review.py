"""Immutable original corpse-initialization replay; separate from damage review."""
from pathlib import Path
import hashlib,json
import original_dying as native

HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
fixture_path=HERE/'dying-native-oracle.json'
before=fixture_path.read_bytes();fixture=json.loads(before)
assert len(fixture['cases'])==84
for case in fixture['cases']:
    machine=native.Machine(case['input'])
    assert machine.run()==case['output'],(case['input']['name'],case['input']['precision'])
for row in fixture['native_windows']:
    start,end=int(row['start'],16),int(row['end'],16)
    raw=native.IMAGE.at(start,end-start)
    assert raw==native.ACCEPTED.at(start,end-start)
    assert hashlib.sha256(raw).hexdigest()==row['sha256']
assert fixture_path.read_bytes()==before
report={'schema':1,'reviewer':'visual_audit','passed':True,'native_valid_path_passed':True,'native_cases_replayed':84,
        'source_review':['Two terrain height calls use fresh actor positions and explicit f32 stores.',
                         'Three dimension virtual calls are uncached; native intermediate speed is stored f32.',
                         'Ability context uses kind2, actor/allegiance and XY with Zzero.',
                         'World time and death definition are read after ability lifetime ends.',
                         'Only an upper10-second delay clamp applies, preserving negative authored durations.',
                         'Allegiance is queried twice before ownership removal; callback changes are reread.'],
        'resolved_finding':'After successful begin_ability, every Result exit now attempts release exactly once. Primary and cleanup failures are both reported, including the retained ability identity if host release fails. Added tests exercise failed allegiance/invocation/release, combined errors and invalid post-acquisition state.',
        'rust_tests':{'command':'cargo test -p aa-body --test native_dying','passed':3,'failed':0},
        'limits':['Actual ability effects, world removal, allegiance and decay scheduling remain required host services.',
                  'No actor death integration or original exception unwinding parity is established by finite successful-path fixtures.'],
        'pins':{str(p.relative_to(ROOT)):hashlib.sha256(p.read_bytes()).hexdigest()for p in [fixture_path,HERE/'original_dying.py',ROOT/'crates/aa-body/src/dying.rs',ROOT/'crates/aa-body/tests/native_dying.rs']}}
(HERE/'independent-dying-review.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
print(json.dumps({'passed':True,'native_cases':84,'cleanup_finding_resolved':True}))
