"""Independent reviewer replay; never overwrites author artifacts."""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(HERE))
import native_values,native_source
sha=lambda b:hashlib.sha256(b).hexdigest()
ledger=json.loads((HERE/'findings.json').read_text())
for file,digest in ledger['pins'].items():
 actual=sha((ROOT/file).read_bytes());assert actual==digest,(file,digest,actual)
original,modified=native_values.original,native_values.modified
for item in ledger['static_source_ranges']:
 a,b=int(item['start'],16),int(item['end'],16);raw=original.range(a,b-a)
 assert raw==modified.range(a,b-a) and sha(raw)==item['sha256']
report={'author_ledger_sha256':sha((HERE/'findings.json').read_bytes()),'verified_file_pins':len(ledger['pins']),'static_source_ranges':len(ledger['static_source_ranges']),'original_sha256':sha(original.b),'accepted_sha256':sha(modified.b),'native':{}}
for name,producer,filename in [('values',native_values,'native-values.json'),('source',native_source,'native-source.json')]:
 doc=json.loads((HERE/filename).read_text());visited={}
 cases=list(producer.cases());assert len(cases)==len(doc['cases'])
 for index in reversed(range(len(cases))):
  c=cases[index];assert c==doc['cases'][index]['input']
  if name=='values':m=producer.EconomyMachine(c);out=m.run()
  else:m=producer.SourceMachine(c['precision']);out=m.load(c)
  assert out==doc['cases'][index]['output'],(name,index,c)
  visited.update(m.visited)
 if name=='source':
  m=producer.SourceMachine('Double53');assert m.enum()==doc['enums'];visited.update(m.visited)
 expected={int(i['va'],16):bytes.fromhex(i['bytes'])for i in doc['instructions']}
 assert visited==expected,name
 for a,raw in visited.items():assert raw==original.range(a,len(raw))==modified.range(a,len(raw))
 report['native'][name]={'cases':len(cases),'visited_instruction_windows':len(visited),'exact_expected_outputs':True,'reverse_order_replay':True}
report['passed']=True
(HERE/'review-native-replay.json').write_text(json.dumps(report,indent=2)+'\n',encoding='utf8');print(json.dumps(report))
