"""Adapt native-writer-validated research records into the Rust decoded schema."""
from pathlib import Path
import hashlib,json
HERE=Path(__file__).resolve().parent
def sha(path):return hashlib.sha256(path.read_bytes()).hexdigest()
oracle=json.loads((HERE/'native-saved-writer.json').read_text());checked={(r['map'],r['trigger_id']):r for r in oracle['cases']}
def span(value):return {'offset':value[0],'length':value[1]}
def units(text):return list(int.from_bytes(text.encode('utf-16le')[i:i+2],'little') for i in range(0,len(text.encode('utf-16le')),2))
def param(p):
 v=p['value'];kind=p['storage']
 if kind==0:v='Empty'
 elif kind in [1,2]:v={'Word':v}
 elif kind==5:v={'Position':v}
 elif kind==6:v={'Text':units(v)}
 elif kind==7:v={'Localized':{'key':units(v['key']),'inline':None if v['inline'] is None else units(v['inline'])}}
 return {'source':span(p['span']),**{k:p[k] for k in ['argument','item','item_index','storage','flag']},'value':v}
def element(e):return {'source':span(e['span']),**{k:e[k] for k in ['name','opcode','word_10']},'parameters':[param(p) for p in e['parameters']]}
rows=[]
for name in ['tutorial01','tutorial02']:
 path=HERE/f'{name}-decoded.json';doc=json.loads(path.read_text());m=doc['manager'];triggers=[]
 for t in m['triggers']:
  assert checked[name,t['id']]['span']==t['span']
  triggers.append({'source':span(t['span']),**{k:t[k] for k in ['id','mode','enabled','repeat','activation_bits','word_2c']},'name':units(t['name']),'description':units(t['description']),**{k:[element(e) for e in t[k]] for k in ['events','conditions','actions']}})
 rows.append({'map':name,'source':doc['source'],'research_decode_sha256':sha(path),'expected':{'next_id':m['next_id'],'triggers':triggers,'variables':[{'name':units(v['name']),'bits':v['bits']} for v in m['variables']],'strings':[units(s) for s in m['strings']]}})
doc={'schema':1,'producer_sha256':sha(Path(__file__)),'native_writer_sha256':sha(HERE/'native-saved-writer.json'),'cases':rows}
path=HERE/'saved-rust-fixtures.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'maps':len(rows),'sha256':sha(path)}))
