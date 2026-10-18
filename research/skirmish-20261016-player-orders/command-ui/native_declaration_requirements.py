"""Native5BD080 declaration-only field requirements: a template declaration without a value does not authorize a zero default in a non-template instance. A5-filled operator-new seam. Typed getters run only after successful parse; all source lines/native copy/type behavior execute."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-layouts'))
import layout_machine as lm
import trace_registry as base
from unicorn.x86_const import *
P=lm.P;V=lm.V;CW={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
def main():
 rows=[];visited={}
 for image,label in [(P,'original'),(V,'v14')]:
  base.P=image
  for pc,cw in CW.items():
   for kind,value,getter in [('bool','true',0x5b5380),('flag','true',0x5b5380),('int','-2147483648',0x5b4ed0)]:
    for assigned in [False,True]:
     data=('[template TestTemplate]\n{\n '+kind+' field;\n}\n[Test template=TestTemplate]\n{\n'+(' field = '+value+'\n' if assigned else '')+'}\n').encode()
     m=lm.Machine();m.u.reg_write(UC_X86_REG_FPCW,cw)
     r=m.reparse(data,'declaration-requirement.tgi');out={'parsed':r['error']is None,'error':r['error']}
     if assigned:
      assert r['error']is None,r
      n=m.children(r['root'])[-1];got=m.thiscall(getter,n,[m.cstring('field')]);assert 'eax'in got,got
      out['value']=got['eax']&(0xffffffff if kind=='int'else 255);assert out['value']==(0x80000000 if kind=='int'else 1),out
     else:assert r['error']is not None,'missing non-template value unexpectedly accepted'
     rows.append({'image':label,'precision':pc,'type':kind,'assigned':assigned,'source_utf8':data.decode(),'output':out});visited.update(m.visited)
 base.P=P
 pins=[]
 for va,n in sorted(visited.items()):
  if P.base<=va<P.base+P.image_size:
   raw=P.range(va,n);assert raw==V.range(va,n),hex(va);pins.append({'va':hex(va),'bytes':raw.hex()})
 H=lambda b:hashlib.sha256(b).hexdigest();p=HERE/'native-declaration-requirements.json';p.open('x').write(json.dumps({'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'instructions':pins},indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(pins),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()

