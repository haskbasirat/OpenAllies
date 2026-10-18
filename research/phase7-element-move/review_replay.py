"""Independent replay: reads author evidence, writes reviewer files only."""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE));import native_element_move as native
sha=lambda b:hashlib.sha256(b).hexdigest()
ledger=json.loads((HERE/'findings.json').read_text());changed=[]
for p,d in ledger['source_sha256'].items():assert sha((ROOT/p).read_bytes())==d,p
for p,d in ledger['dependency_sha256'].items():
 actual=sha((ROOT/p).read_bytes())
 if actual!=d:
  assert p=='crates/aa-mind/src/lib.rs',p
  changed.append({'path':p,'author_hash':d,'current_hash':actual,'note':'Additive continuation module export by navigation after author freeze; inspected separately by reviewer.'})
doc=json.loads((HERE/'native-element-move.json').read_text());cases=native.cases();assert len(cases)==384
visited={};outputs={}
for i in reversed(range(len(cases))):
 c=cases[i];assert c==doc['cases'][i]['input'];m=native.Machine(c);out=m.run();assert out==doc['cases'][i]['output'],c['name'];visited.update(m.visited);outputs[i]=out
leaves,lv=native.leaf_cases();assert leaves==doc['leaf_cases'];visited.update(lv)
assert visited=={int(v['va'],16):v['bytes']for v in doc['executed_windows']}
for a,raw in visited.items():assert bytes.fromhex(raw)==native.original.range(a,len(raw)//2)==native.modified.range(a,len(raw)//2)
for c in doc['constants']:
 raw=bytes.fromhex(c['bytes']);a=int(c['va'],16);assert raw==native.original.range(a,len(raw))==native.modified.range(a,len(raw))
width=len(cases)//3;witnesses=sum(outputs[i]!=outputs[i+width] or outputs[i]!=outputs[i+2*width]for i in range(width));assert witnesses==6
report={'passed':True,'author_ledger_sha256':sha((HERE/'findings.json').read_bytes()),'state_cases':len(cases),'numeric_cases':len(leaves),'precision_witnesses':witnesses,'execution_order':'reverse state cases','exact_fixture_equality':True,'instruction_windows':len(visited),'verified_source_pins':len(ledger['source_sha256']),'dependencies_rebound':changed}
(HERE/'review-native-replay.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report))
