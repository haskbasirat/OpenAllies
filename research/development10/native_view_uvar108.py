"""Native UVar108 Bool setter, same flags0 registration identity as41F9D0.
Executes4B29B0/4B3190/40FCA0/40EAA0; no successful game-code seam.
The existing full213-registration capture supplies separate ctor provenance.
This is not whole VThing Init or profile completion evidence.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-view-init'))
import native_view_bool_assignment as base
def main():
 rows=[];base.h.VISITED.clear()
 for pc,before,requested,dev,session in itertools.product(base.h.PRECISION,[0,128],[0,1,128,255],[0,128],[(0,0),(3,1),(1,1)]):
  c={'before':before,'requested':requested,'flags':0,'dev':dev,'e4':session[0],'f4':session[1]}
  a=base.run('original',pc,c);b=base.run('v14',pc,c);assert a==b
  rows.append({'precision':pc,'input':c,'output':a})
 pins=[]
 for a,b in sorted(base.h.VISITED.items()):
  raw=bytes.fromhex(b);assert raw==base.ORIGINAL.range(a,len(raw))==base.h.V.range(a,len(raw));pins.append({'va':hex(a),'bytes':b})
 doc={'schema':1,'scope':__doc__,'original_sha256':base.h.ORIGINAL_SHA,'v14_sha256':base.h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'executed_instructions':pins}
 p=HERE/'native-view-uvar108.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'executions':len(rows)*2,'pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
