"""Native parser declaration-only bool/flag/int initial values with A5-filled operator-new blocks. Source/heap/CString seams inherited from layout_machine; native5BD080 parser and getters execute. No missing-field fallback."""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-layouts'))
import layout_machine as lm
import trace_registry as base
from unicorn.x86_const import *
P=lm.P;V=lm.V;CW={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
DATA=b'[template TestTemplate]\n{\n bool b\n flag f;\n int i;\n bool bt = true\n flag ft = true\n int iv = -2147483648\n}\n[Test template=TestTemplate]\n{\n}\n'
def main():
 rows=[];visited={}
 for image,label in [(P,'original'),(V,'v14')]:
  base.P=image
  for pc,cw in CW.items():
   m=lm.Machine();m.u.reg_write(UC_X86_REG_FPCW,cw)
   root=m.parse_file('explicit-declarations.tgi',DATA)['root'];nodes=m.children(root);assert len(nodes)==2
   values={}
   for name,getter in [('b',0x5b5380),('f',0x5b5380),('i',0x5b4ed0),('bt',0x5b5380),('ft',0x5b5380),('iv',0x5b4ed0)]:
    r=m.thiscall(getter,nodes[1],[m.cstring(name)]);assert 'eax'in r,r
    values[name]=r['eax']& (0xffffffff if getter==0x5b4ed0 else 255)
   assert values=={'b':0,'f':0,'i':0,'bt':1,'ft':1,'iv':0x80000000},values
   rows.append({'image':label,'precision':pc,'values':values});visited.update(m.visited)
 base.P=P
 pins=[]
 for va,n in sorted(visited.items()):
  if P.base<=va<P.base+P.image_size:
   raw=P.range(va,n);assert raw==V.range(va,n),hex(va);pins.append({'va':hex(va),'bytes':raw.hex()})
 H=lambda b:hashlib.sha256(b).hexdigest()
 p=HERE/'native-declaration-defaults.json';p.open('x').write(json.dumps({'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'source_utf8':DATA.decode(),'rows':rows,'instructions':pins},indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(pins),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()

