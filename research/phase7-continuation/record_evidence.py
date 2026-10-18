from pathlib import Path
import sys,json,hashlib,subprocess,collections
ROOT=Path(__file__).resolve().parents[2];OUT=Path(__file__).parent
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
sys.path.insert(0,str(ROOT/'research/phase5-navigation'))
import native_oracle as old
gold=json.loads((ROOT/'research/phase5-navigation/native-navigation-fixtures.json').read_text())
pe=old.Pe(ROOT.parent/'AA.exe',gold['original_sha256'])
for row in gold['fixtures']:assert old.Machine(pe,row['input']).run()==row['output'],row['input']['name']
commands=[(['cargo','test','-p','aa-navigation','--test','native_fixtures'],'navigation-tests.log'),(['cargo','test','-p','aa-mind','--test','continuation_native'],'continuation-tests.log'),(['cargo','test','-p','aa-trig','--lib'],'trig-tests.log'),(['cargo','clippy','-p','aa-navigation','--test','native_fixtures','--','-D','warnings'],'navigation-clippy.log'),(['cargo','clippy','-p','aa-mind','--test','continuation_native','--','-D','warnings'],'continuation-clippy.log'),(['cargo','clippy','-p','aa-trig','--lib','--','-D','warnings'],'trig-clippy.log')]
for cmd,log in commands:
 r=subprocess.run(cmd,cwd=ROOT,text=True,capture_output=True);(OUT/log).write_text(r.stdout+r.stderr);assert r.returncode==0,(cmd,r.stdout+r.stderr)
source=['crates/aa-navigation/src/lib.rs','crates/aa-navigation/src/action_world.rs','crates/aa-navigation/tests/native_fixtures.rs','crates/aa-mind/src/element_move_continuation.rs','crates/aa-mind/tests/continuation_native.rs','crates/aa-trig/src/turning.rs']
deps=['crates/aa-mind/src/element_move.rs','crates/aa-mind/src/lib.rs','crates/aa-mind/Cargo.toml','crates/aa-trig/src/lib.rs','crates/aa-trig/src/angle.rs','crates/aa-numeric/src/lib.rs','research/phase4-trig/lookup-tables.bin','research/phase5-navigation/native-navigation-fixtures.json','research/phase5-navigation/native_oracle.py']
offset=json.loads((OUT/'native-offset.json').read_text());outer=json.loads((OUT/'native-continuation.json').read_text())
counts=collections.Counter(r['input']['op']for r in outer['cases'])
evidence=[p for p in OUT.iterdir() if p.is_file() and p.suffix in ('.py','.json','.txt','.log','.md') and p.name not in ('findings.json','independent-review.json')]
report=dict(schema=1,passed=True,source={p:sha(ROOT/p)for p in source},dependencies={p:sha(ROOT/p)for p in deps},evidence={str(p.relative_to(ROOT)):sha(p)for p in evidence},validation=dict(original_waypoint_cases=len(offset['cases']),original_continuation_cases=len(outer['cases']),continuation_by_operation=dict(counts),existing_action_path_replayed=len(gold['fixtures']),rust_tests=13,commands=[x[0]for x in commands]),scope='Exact native kernels with required source-bound world services; see README.md for callback and live-runtime limits. Peer review pending separately.')
(OUT/'findings.json').write_text(json.dumps(report,indent=2)+'\n');print(json.dumps({'passed':True,'sha256':sha(OUT/'findings.json'),'counts':dict(counts)}))
