"""Independent replay into review-only outputs; never rewrites author evidence."""
from pathlib import Path
import hashlib,importlib,json,sys
ROOT=Path(__file__).resolve().parents[2];HERE=Path(__file__).parent;OUT=HERE/'review-replay';OUT.mkdir(exist_ok=True)
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
ledger=json.loads((HERE/'sai-evidence.json').read_text())
for group in ['source_sha256','dependency_sha256']:
 for file,want in ledger[group].items():assert sha(ROOT/file)==want,file
sys.path.insert(0,str(HERE))
import state_oracle as state
from original_rng import original,modified
provenance=json.loads((HERE/'provenance.json').read_text())
for r in provenance['routines'].values():
 a=int(r['address'],16);raw=original.range(a,r['length']);assert hashlib.sha256(raw).hexdigest()==r['sha256'];assert raw==modified.range(a,len(raw))
for file in ['native-state.json','native-controller.json','native-actor-leaves.json','native-ai.json']:
 d=json.loads((HERE/file).read_text())
 for w in d['native_basic_block_pins']:
  raw=bytes.fromhex(w['bytes']);a=int(w['va'],16);assert original.range(a,len(raw))==raw and modified.range(a,len(raw))==raw
rows=[]
for module,file in [('state_oracle','native-state.json'),('controller_oracle','native-controller.json'),('actor_oracle','native-actor-leaves.json'),('ai_oracle','native-ai.json')]:
 m=importlib.import_module(module);m.OUT=OUT;m.main();actual=json.loads((OUT/file).read_text());expected=json.loads((HERE/file).read_text());assert actual==expected,file
 rows.append({'producer':module,'fixture':file,'exact_json_match':True,'sha256':sha(OUT/file)});print(module,'exact replay passed',flush=True)
(OUT/'result.json').write_text(json.dumps({'passed':True,'evidence_sha256':sha(HERE/'sai-evidence.json'),'replays':rows,'native_counts':ledger['native_counts'],'source_pins_verified':len(ledger['source_sha256']),'dependency_pins_verified':len(ledger['dependency_sha256'])},indent=2)+'\n')
