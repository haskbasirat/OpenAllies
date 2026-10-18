"""Source53 ger_corps targets in controlled eligibility native laboratories.
Source definitions/layouts come from actual-source-inputs.json, freshly compared
by source_probe.rs. Current Kingdom Single availability and requirements are
explicit lab controls; they are not a captured actor or whole-loader input.
"""
import json,hashlib,sys
from pathlib import Path
import native_eligibility as N
HERE=Path(__file__).resolve().parent
def main():
 source=json.loads((HERE/'actual-source-inputs.json').read_text());rows=[];pins={}
 for target in source['targets']:
  for mode in('no_owner','missing','single_unavailable','single_available'):
   c=dict(definitions=source['definitions'],layouts=source['layouts'],target=target,explicit_layout=source['definitions'][target]['layout'],check=1,owner=mode!='no_owner',own_list=source['targets'],items=[]if mode in('no_owner','missing')else[dict(definition=i,available=mode=='single_available')for i in range(len(source['definitions']))],wrapper=True)
   for pc in N.PC:
    a,p1=N.run(c,'original',pc);b,p2=N.run(c,'v14',pc);assert a==b
    for pin in p1+p2:pins[pin['address']]=pin['sha256']
    rows.append(dict(target=source['definitions'][target]['ids'],mode=mode,pc=pc,input=c,output=a))
 payload=dict(scope=__doc__,source_inputs_sha256=hashlib.sha256((HERE/'actual-source-inputs.json').read_bytes()).hexdigest(),rows=rows,basic_block_windows=[dict(address=a,sha256=h)for a,h in sorted(pins.items())])
 path=HERE/'native-actual-source.json'
 if '--write'in sys.argv:path.write_text(json.dumps(payload,indent=2)+'\n')
 else:assert json.loads(path.read_text())==payload
 print('PASS',len(rows),'rows',2*len(rows),'separate original-v14/allPC executions',len(pins),'basic-block byte windows',hashlib.sha256(path.read_bytes()).hexdigest())
if __name__=='__main__':main()
