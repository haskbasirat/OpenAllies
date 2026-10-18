"""Reproduce bounded spatial checks and hash their exact sources/outputs."""
from pathlib import Path
import collections,hashlib,json,subprocess,sys
ROOT=Path(__file__).resolve().parents[2]
OUT=Path(__file__).parent
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def main():
 commands=[]
 for name,cmd in [
  ('native-oracle.log',[sys.executable,str(OUT/'original_spatial.py')]),
  ('rust-tests.log',['cargo','test','-p','aa-spatial']),
  ('clippy.log',['cargo','clippy','-p','aa-spatial','--all-targets','--','-D','warnings'])]:
  run=subprocess.run(cmd,cwd=ROOT,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
  (OUT/name).write_bytes(run.stdout)
  commands.append({'command':cmd,'exit_code':run.returncode,'log':name,'sha256':sha(OUT/name)})
  assert run.returncode==0,run.stdout.decode(errors='replace')
 fixture=json.loads((OUT/'native-spatial-fixtures.json').read_text())
 ranges=json.loads((OUT/'native-ranges.json').read_text())
 files=[ROOT/'crates/aa-spatial'/s for s in ['Cargo.toml','src/lib.rs','tests/native_parity.rs','README.md']]
 files += [OUT/s for s in ['README.md','inspect_native.py','original_spatial.py','record_evidence.py','native-ranges.json','native-executed-ranges.json','native-spatial-fixtures.json','client_samples.rs','client-samples.json']]
 deps=[ROOT/s for s in ['crates/aa-numeric/src/lib.rs','research/phase3-terrain/terrain_uv_oracle.py']]
 report={
  'schema':1,'passed':True,'component':'aa-spatial bounded ground-position and terrain query',
  'original_sha256':ranges['original_sha256'],'v14_sha256':ranges['v14_sha256'],
  'native_cases':fixture['case_count'],'by_operation':dict(collections.Counter(f['input']['operation'] for f in fixture['fixtures'])),
  'precision_modes':['Single24','Double53','Extended64'],'rust_tests':4,
  'native_executed_instructions':json.loads((OUT/'native-executed-ranges.json').read_text())['instruction_count'],
  'source_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in files},
  'dependency_sha256':{p.relative_to(ROOT).as_posix():sha(p) for p in deps},'commands':commands,
  'native_operations':['4EEB40 ground move','4F0E50 height query coordination','4EE5C0 position commit','4CF210/4EA0F0 shape snapshots','46EE00 half-cell snapping','50C250 presentation predicate'],
  'seams':fixture['seams'],
  'bounds':[
   'No collision/path solver, heightfield sampler implementation, tile ownership, or rendering implementation.',
   'Actor callback mutation is re-read at native points; world grid/settings remain stable operation inputs.',
   'Commit candidate is a distinct copied buffer as in4EEB40; arbitrary aliasing is excluded.',
   'Bit12 withoutbit21 uninitialized native height rejects; invalid/nonfinite arithmetic and grid domains reject.',
   'Snapping requires finite positive half-cell/reciprocal and i32 floor results, narrower than general native CRTfloor.',
   'Operations are nontransactional and cannot be automatically retried after failure.',
   'No adapter to aa-movement GroundWorld exists; its infallible move callback cannot silently discard spatial Result errors.',
   'Explicit round-to-nearest PC24/53/64; no process-wide native precision default inferred.',
   'No live process, GUI, original asset, native runtime, profile or security change.'
  ],
  'peer_review':'Separate independent-review.json; not included recursively in source ledger.'
 }
 (OUT/'findings.json').write_text(json.dumps(report,indent=2)+'\n')
 print(json.dumps({'passed':True,'native_cases':fixture['case_count'],'findings_sha256':sha(OUT/'findings.json')}))
if __name__=='__main__':main()
