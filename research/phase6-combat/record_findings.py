"""Bind implemented combat/body evidence without claiming a complete world."""
from pathlib import Path
import hashlib,json,re,subprocess
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
commands=[];test_count=0
for name,command in [('body-tests.log',['cargo','test','-p','aa-body']),
                     ('clippy.log',['cargo','clippy','-p','aa-body','--all-targets','--','-D','warnings'])]:
    run=subprocess.run(command,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
    (HERE/name).write_bytes(run.stdout)
    commands.append({'command':command,'exit_code':run.returncode,'log':name,'sha256':sha(HERE/name)})
    assert run.returncode==0,run.stdout.decode(errors='replace')
    if name=='body-tests.log':test_count=sum(map(int,re.findall(r'test result: ok\. (\d+) passed',run.stdout.decode())))
damage=json.loads((HERE/'damage-native-oracle.json').read_text())
dying=json.loads((HERE/'dying-native-oracle.json').read_text())
for report,producer in [(damage,'original_damage.py'),(dying,'original_dying.py')]:
    assert report['producer_sha256']==sha(HERE/producer)
assert len(damage['cases'])==219 and len(dying['cases'])==84
files=[ROOT/'crates/aa-body'/p for p in ['Cargo.toml','src/lib.rs','src/damage.rs','src/dying.rs',
       'tests/native_parity.rs','tests/native_damage.rs','tests/native_dying.rs','README.md']]
files += [HERE/p for p in ['README.txt','inspect_native.py','native-ranges.json',
         'original_damage.py','damage-native-oracle.json','original_dying.py','dying-native-oracle.json']]
report={'schema':1,'passed':True,'component':'Body damage and corpse initialization',
    'original_sha256':damage['original_sha256'],'accepted_sha256':damage['accepted_sha256'],
    'rust_tests':test_count,'focused_historical_health_damage_dying_tests':8,'native_damage_cases':219,'native_dying_cases':84,'historical_body_cases':270,
    'commands':commands,'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in files},
    'dependency_sha256':{p:sha(ROOT/p) for p in ['crates/aa-numeric/src/lib.rs','research/phase4-body/original_body.py']},
    'scope':['5625C0 outer damage including required linked/wall dispatch, arithmetic/history, component order, capture-health and death queue selection.',
             '561E50 source-aware setter with required graph/source resolution, XY-only directionalFX and extended modifier maximum.',
             '562380 corpse timing, optional ability lifetime, repeated terrain/dimension calls, sink arithmetic and timer/allegiance sequence.'],
    'limits':['No fully integrated combat world, implicit graph/component defaults, actor factories, ownership, abilities, timers or death queues.',
              'Upstream aa-combat and sibling credit/experience modules have separate implementation/evidence scopes; this report does not expand its review to them.',
              'Explicit finite PC24/53/64; errors are nontransactional and must not trigger automatic retries.',
              'Native diagnostic logging/string lifetime and invalid native pointer behavior are excluded.',
              'Visited original byte pins include intercepted seam entry addresses, not execution behind those leaves.'],
    'peer_review':'independent-review.json and any separate dying review; this producer is not its own review.'}
(HERE/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps({'passed':True,'sha256':sha(HERE/'findings.json')}))
