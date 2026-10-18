"""Real4EC4A0->4EAAA0->49B480 default-index memory path, class9.
Explicit scalar/default and saved variant=-1 guard inputs, successful CString
transport. This separately verifies consumption, not definition/asset loading.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-store-memory-refresh'))
import native_memory_refresh as b
h=b.h;H=lambda v:hashlib.sha256(v).hexdigest()
def main():
 rows=[]
 for default in [0,7,0xffffffff]:
  for pc in h.CW:
   c={'class':9,'time':0,'flags':0x40a12400,'position':[1128563193,1107914057],'variant':0xffffffff,'owner':0,'display':'\u0085Hedges','key':'','text':'','kingdom':'','area':'','camera':'','stop':'none','data_variations':False}
   out=[]
   for image in(h.P,h.V):
    m=b.Machine(pc,image,c);m.w(m.definition+0x118,default);out.append(m.run())
   assert out[0]==out[1];assert out[0]['aspect']==default
   rows.append({'input':dict(c,default=default),'pc':pc,'native':out[0]})
 assert 0x49b480 in h.VISITED
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':v,'kind':'executed'}for a,v in sorted(h.VISITED.items())]}
 p=HERE/'native-default-memory.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
