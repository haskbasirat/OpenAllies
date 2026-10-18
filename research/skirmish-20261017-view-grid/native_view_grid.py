"""Actual430290/869C30 constructor and42F540 first grid allocation.
Constructor globals are explicit transported current words, not a claim that
the native BSS seed remains unchanged. Unwritten storage is poisoned and only
executed MOV stores become initialized outputs. Heap is the declared growable
allocation transport; CRT ceil/truncate and native zero-fill execute unchanged.
"""
from pathlib import Path
import json,hashlib,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ECX,UC_X86_REG_EAX
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stop(Exception):pass

def run(c,image,pc,stop=None):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);v=m.alloc(0x110)
 m.u.mem_write(v,b'\xcd'*0x110);events=[];counts={};written=set();active=True
 global_reads={0x4302c2:0xb43dbc,0x4302ca:0xb43dc0,0x869c30:0xb43cf8,0x869c3a:0xb43cfc,0x869c43:0xb43d00}
 fixed={0x4302b2:4,0x4302b5:8,0x4302bc:12,0x4302bf:16,0x4302c7:20,0x4302d5:24,
  0x4302f4:0x100,0x4302fa:0x104,0x430300:0x108,0x430306:0x10c,0x43030d:0,0x43030f:0x1c,0x430312:0x20,0x430315:0x24}
 init_stores={0x42f55b:0x14,0x42f55e:0x18,0x42f561:0x24,0x42f58e:0x1c,0x42f5be:0x20}
 def event(label):
  if not active:return
  key=f'{label}:{counts.get(label,0)}';counts[label]=counts.get(label,0)+1
  if stop==key:raise Stop(key)
  events.append(key)
 def before(u,at,size,_):
  if at in global_reads:
   address=global_reads[at];event(f'load{address:x}')
   count=counts.get(f'load{address:x}',0)-1 if active else 0
   m.w(address,(c['globals'][hex(address)]+c['step']*count)&0xffffffff)
  if at in fixed:
   offset=fixed[at];event(f'store{offset:x}');written.add(offset)
  elif at in(0x869c38,0x869c40,0x869c49,0x869c4c):
   base=u.reg_read(UC_X86_REG_EAX if at==0x869c4c else UC_X86_REG_ECX)
   offset=base-v+{0x869c38:0,0x869c40:4,0x869c49:8,0x869c4c:12}[at]
   event(f'store{offset:x}');written.add(offset)
  if c['phase']=='init':
   if at in init_stores:
    offset=init_stores[at];event(f'store{offset:x}');written.add(offset)
   elif at==0x6f7810 and active:event('allocate')
   elif at==0x42f5e0:
    event('publish');written.add(0)
 m.u.hook_add(UC_HOOK_CODE,before)
 stopped=None
 try:
  if c['phase']=='init':
   active=False;m.call(0x430290,v);active=True;events.clear();counts.clear()
   m.call(0x42f540,v,[*c['extents'],c['shift']])
  else:m.call(0x430290,v)
 except Stop as e:stopped=str(e)
 words=[m.r(v+offset)if offset in written else None for offset in range(0,0x110,4)]
 grid=False;zero=None;count=None
 if words[0]not in(None,0):
  words[0]='grid';grid=True
  count=m.r(v+0x1c)*m.r(v+0x20);zero=not any(m.u.mem_read(m.r(v),count*8))
 return dict(events=events,words=words,grid=grid,cells=count,zero=zero,stopped=stopped),m.windows()

def cases():
 seeds={hex(a):0 for a in(0xb43dbc,0xb43dc0,0xb43cf8,0xb43cfc,0xb43d00)}
 yield dict(phase='ctor',globals=seeds,step=0)
 yield dict(phase='ctor',globals={hex(a):v for a,v in zip((0xb43dbc,0xb43dc0,0xb43cf8,0xb43cfc,0xb43d00),(0x80000000,0x7fc01234,0x7f800001,0xbf800000,0x12345678))},step=0)
 yield dict(phase='ctor',globals=seeds,step=0x10203)
 for extents,shift in [([0,0],0),([0x80000000,0],4),([0x3f800000,0x3f800000],0),([0x3f000000,0x3fc00000],0),
  ([0x43400000,0x43400000],4),([0x43408000,0x43800000],4),([0x42ffffff,0x43000001],3),([0x43000001,0x42ffffff],35),
  ([0x44000000,0x43c00000],5),([0x3f800001,0x3fffffff],1),([0x43400000,0x43400000],9)]:
  yield dict(phase='init',globals=seeds,step=0,extents=extents,shift=shift)

def main():
 rows=[];pins={}
 for c in cases():
  baseline,_=run(c,'original','Double53')
  for stop in [None]+baseline['events']:
   for pc in PC:
    a,pa=run(c,'original',pc,stop);b,pb=run(c,'v14',pc,stop);assert a==b,(c,stop,pc,a,b)
    rows.append(dict(input=c,pc=pc,stop=stop,output=a));pins.update({v['address']:v for v in pa+pb})
 out=dict(schema=1,scope=__doc__,cases=rows,blocks=[pins[k]for k in sorted(pins)])
 raw=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-view-grid.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
