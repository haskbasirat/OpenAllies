"""Whole5543F0 tile query and actual concrete Mind vf80 getters.
The existing constructor fixture proves each concrete vtable; its+80 words
are immutable data pins here. Both PEs/all requested PCs execute unchanged.
No numeric/position/getter callback is intercepted. Base vf80 purecall remains
an explicit required boundary. Aircraft rows are finite (x87 fld/fst scope).
"""
from pathlib import Path
import hashlib,json,random,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-member-deployment'))
from native_member_deployment import machine,nm,bits
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
TABLES={'element':(0xa35810,0x535ee0,[0x90,0x94]),'regiment':(0xa335a0,0x524890,[0x7c,0x80]),'aircraft':(0xa32710,0x51c230,[0x4c,0x50])}

def inputs():
 rng=random.Random(0x52289080)
 out=[]
 values=[0,0x80000000,bits(.25),bits(-9.5),0x7f7fffff,0x00800000,1]
 for kind in TABLES:
  for i in range(14):
   out.append(dict(op='target',kind=kind,values=[values[i%7],values[(i+3)%7]],z=bits(i-4)))
 for shift in [0,2,5]:
  extent=[8<<shift,4<<shift]
  points=[[-1,0],[0,-1],[0,0],[-0.,-0.],[.5,.5],[1,1],[extent[0]-.001,extent[1]-.001],[extent[0],0],[0,extent[1]],[extent[0]+1,1],[1,extent[1]+1]]
  points += [[rng.random()*extent[0],rng.random()*extent[1]]for _ in range(14)]
  points=list(map(lambda p:list(map(bits,p)),points))+[[0x7fc00000,0],[0,0x7fc00000],[0x7f800000,0],[0xff800000,0]]
  for point in points:out.append(dict(op='tile',shift=shift,width=8,height=4,extent=list(map(bits,extent)),point=point))
 return out

def run(m,c):
 if c['op']=='target':
  table,entry,offsets=TABLES[c['kind']];obj=m.alloc(0x300);out=m.alloc(8)
  m.w(obj,table)
  for off,v in zip(offsets,c['values']):m.w(obj+off,v)
  if c['kind']=='aircraft':m.w(obj+0x54,c['z'])
  actual=m.r(m.r(obj)+0x80);assert actual==entry
  m.call(actual,obj,[out]);return [m.r(out),m.r(out+4)]
 grid=m.alloc(0x30);tiles=m.alloc(c['width']*c['height']*0x30)
 for off,v in [(4,c['shift']),(8,bits(1<<c['shift'])),(0xc,c['extent'][0]),(0x10,c['extent'][1]),(0x1c,c['width']),(0x20,c['height']),(0x28,tiles)]:m.w(grid+off,v)
 result=m.call(0x5543f0,grid,c['point'])
 if result==0:return None
 assert (result-tiles)%0x30==0
 index=(result-tiles)//0x30;assert 0<=index<c['width']*c['height'];return index

def main():
 prior=ROOT/'research/phase7-mind/native-constructors.json';ctor=json.loads(prior.read_text());static=[]
 for kind,(table,entry,_)in TABLES.items():
  row=next(r for r in ctor['cases']if r['input']['kind']==kind)
  assert struct.unpack_from('<I',bytes.fromhex(row['output']['initialized_hex']))[0]==table
  raw=struct.pack('<I',entry);assert nm.original.range(table+0x80,4)==nm.modified.range(table+0x80,4)==raw
  static.append(dict(va=hex(table+0x80),bytes=raw.hex(),classification='immutable concrete vtable data; constructor result proved by prior fixture'))
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  machines=[machine(label,pc)for label in ['original','v14']]
  for c in inputs():
   outputs=[run(m,c)for m in machines];assert outputs[0]==outputs[1]
   rows.append(dict(precision=pc,input=c,output=outputs[0]))
  for m in machines:pins.update(m.visited)
 for va,raw in pins.items():assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 doc={**nm.header(__file__,__doc__),'constructor_fixture_sha256':sha(prior),'static_pins':static,'rows':rows,'instructions':nm.instructions(pins)}
 path=HERE/'native-member-command-fields.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,executed_pins=len(pins),sha256=sha(path))))
if __name__=='__main__':main()
