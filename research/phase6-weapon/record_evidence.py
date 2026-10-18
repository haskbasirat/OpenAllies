from pathlib import Path
import hashlib,json

HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
groups=[('weapon',486,'native_weapon.py','native-weapon-fixtures.json'),
        ('periodic',90,'native_periodic.py','native-periodic-fixtures.json'),
        ('metadata',8,'native_damage_types.py','native-damage-type-fixtures.json')]
fixtures={}
paths=[HERE/'README.txt',HERE/'record_evidence.py',HERE/'rust-tests.log',HERE/'clippy.log',
       HERE/'native-weapon.log',HERE/'native-periodic.log',HERE/'native-damage-types.log']
for name,count,producer,fixture in groups:
    value=json.loads((HERE/fixture).read_text(encoding='utf-8'))
    assert value['passed'] and len(value['cases'])==count
    assert value['producer_sha256']==sha(HERE/producer)
    for path,digest in value['oracle_dependencies'].items():
        assert digest==sha(ROOT/path),path
        paths.append(ROOT/path)
    assert value['visited_windows']
    fixtures[name]=value
    paths.extend([HERE/producer,HERE/fixture])
for path,digest in fixtures['metadata']['authored_sources'].items():
    p=ROOT.parent/'reconstruction/assets/original/Data'/path
    assert sha(p)==digest
    paths.append(p)
tests=(HERE/'rust-tests.log').read_text(encoding='utf-8-sig')
clippy=(HERE/'clippy.log').read_text(encoding='utf-8-sig')
assert '2 passed; 0 failed' in tests and '4 passed; 0 failed' in tests
assert 'Finished' in clippy and 'error:' not in clippy
paths.extend((ROOT/'crates/aa-combat').rglob('*.rs'))
paths.append(ROOT/'crates/aa-combat/Cargo.toml')
paths.extend((ROOT/'crates/aa-body/src').rglob('*.rs'))
paths.extend((ROOT/'crates/aa-numeric/src').rglob('*.rs'))
paths.append(ROOT/'crates/aa-body/tests/native_damage.rs')
def relative(path):
    try:return path.relative_to(ROOT).as_posix()
    except ValueError:return '../'+path.relative_to(ROOT.parent).as_posix()
report={
    'schema':1,'passed':True,'component':'native weapon and periodic property callers through real Body',
    'original_sha256':fixtures['weapon']['original_sha256'],
    'accepted_sha256':fixtures['weapon']['accepted_sha256'],
    'native_cases':584,'native_weapon_cases':486,'native_periodic_cases':90,'native_metadata_cases':8,
    'rust_tests':6,'strict_clippy_passed':True,
    'precision_modes':['Single24','Double53','Extended64'],
    'integration':'Weapon and periodic tests dispatch real aa-body healing/damage; required world callbacks remain explicit. No playable world or scheduler wiring is claimed.',
    'dependency_refresh':'Body lib.rs added credit/experience module exports and header wording after initial validation. The six combat tests and strict Clippy were rerun against the final dependency. Health/damage callees are unchanged; this ledger does not independently approve credit/experience behavior.',
    'implemented':['native weapon damage arithmetic, clamps, offense/defense flags and exact DamageRequest',
                   'periodic effect expiry/removal/reenabling, derived flags and ordered heal/damage dispatch',
                   'damage-type presentation writes and exception boundary ordering'],
    'limits':['finite checked numeric domain and valid typed handle storage',
              'actor graph, targeting, projectile/range/cadence, registry loading and world scheduling remain separate',
              'fixture world callbacks are not live game integration',
              'same-Properties reentrant tick and arbitrary raw item mutation are outside the supported callback contract',
              'opaque nonzero stack healing-event token maps to 1; native exception unwinding is not emulated',
              'errors do not roll back prior effects'],
    'sha256':{relative(p):sha(p) for p in sorted(set(paths))},
}
(HERE/'findings.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf-8')
print(json.dumps({'passed':True,'findings_sha256':sha(HERE/'findings.json')}))
