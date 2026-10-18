"""Actual86B940 fresh constructor and86E8D0/86EED0/866BE0/86BC30/
8A2C70/86BA10/870720/4B2830 update. Constructor-null controller/property,
parent and bound links only; no native child, allocator or callback seam.
Numeric C8 identity/B32450 and Ni ABI accounting are not portable values.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
ORIGINAL,V14=h.P,h.V
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
SUPPORTED=sorted(set(range(0x38,0xa0,4))|set(range(0xf4,0x154,4))|set(range(0x16c,0x18c,4))|{0xd0,0x158,0x15c,0x160,0x164,0x10,0x24,0xa4,0xb0})
def run(pc,c):
 m=h.Machine(b'',pc);effect=m.alloc(0x190,0xa5);known=set();global_reads=[];reads=[];writes=[];stage='ctor';outcome='ok';c4_known=False
 for i,v in enumerate(c['seeds']):m.w(0xb43cf8+4*i,v)
 def read(u,access,a,size,value,data):
  nonlocal outcome
  if stage=='ctor'and a in(0xb43cf8,0xb43cfc,0xb43d00)and size==4:
   global_reads.append([a,m.r(a)])
   if len(global_reads)==c['seed_fail']:
    outcome='required global';m.outcome={'kind':outcome};raise h.Throw()
  if stage=='update'and effect<=a<effect+0x190:
   assert size==4;reads.append(a-effect)
   if len(reads)==c['read_fail']:
    outcome='required field';m.outcome={'kind':outcome};raise h.Throw()
 def write(u,access,a,size,value,data):
  nonlocal c4_known
  if effect<=a<effect+0x190:
   known.update(range(a-effect,a-effect+size))
   if a<=effect+0xc4<a+size:c4_known=True
   if stage=='update':assert size==4;writes.append([a-effect,value&0xffffffff])
 m.u.hook_add(UC_HOOK_MEM_READ,read);m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 m.call(0x86b940,effect,[])
 if outcome=='ok'and c['flow']=='update':
  for i,v in enumerate(c['local']):m.w(effect+0x38+4*i,v)
  for offset,value in [(0x160,2),(0x158,1),(0x15c,3),(0x164,2)]:m.w(effect+offset,value)
  m.call(0x86ba10,effect,[]);m.wb(effect+0xc4,1)
  stage='update';m.call(0x86e8d0,effect,[c['time'],c['controllers']])
 assert not m.null_accesses,m.null_accesses
 assert not m.calls,m.calls
 return dict(outcome=outcome,global_reads=global_reads,reads=reads,writes=writes,
  words={str(o):m.r(effect+o)if all(o+i in known for i in range(4))else None for o in SUPPORTED},enabled_c4=m.b(effect+0xc4)if c4_known else None)
def main():
 cases=[]
 for seeds in ([0,0,0],list(map(h.f32,[3,-4,.125]))):
  cases.append(dict(flow='ctor',seeds=seeds,seed_fail=None,read_fail=None))
  cases.extend(dict(flow='ctor',seeds=seeds,seed_fail=i,read_fail=None)for i in range(1,10))
 local=[[h.f32(v)for v in [1,0,0,0,1,0,0,0,1,0,0,0,1]],
  [h.f32(v)for v in [.5,-.125,2,.25,.75,-1,-2,.125,.5,1,-2,3,.5]]]
 for values,controllers in itertools.product(local,[0,1,128]):cases.append(dict(flow='update',seeds=[0,0,0],seed_fail=None,read_fail=None,local=values,time=h.f32(1.25),controllers=controllers))
 sample=cases[-2];n=len(run('Double53',sample)['reads']);cases.extend(dict(sample,read_fail=i)for i in range(1,n+1))
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.CW,cases):
  outputs=[]
  for image in(ORIGINAL,V14):h.P=image;outputs.append(run(pc,c))
  assert outputs[0]==outputs[1];rows.append(dict(precision=pc,input=c,output=outputs[0]))
 h.P=ORIGINAL
 virtual={hex(o):int.from_bytes(ORIGINAL.range(0xab70a0+o,4),'little')for o in(0x48,0x64,0x68)}
 assert virtual=={'0x48':0x86eed0,'0x64':0x4b2830,'0x68':0x86bc30}
 for o in virtual:assert ORIGINAL.range(0xab70a0+int(o,16),4)==V14.range(0xab70a0+int(o,16),4)
 assert ORIGINAL.range(0xb3206c,12)==V14.range(0xb3206c,12)==bytes.fromhex('0000803f0000000000000000')
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,pins={hex(a):v for a,v in sorted(h.VISITED.items())},seams=[],virtual_slots=virtual,unit_x_constant=dict(address=hex(0xb3206c),bytes=ORIGINAL.range(0xb3206c,12).hex()),supported_offsets=SUPPORTED,domain='fresh typed constructor-null links, finite successful arithmetic, round-nearest masked explicit x87 PC; no native pointer/identity/refcount ABI')
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-effect.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(doc['pins']),required_update_reads=n,sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
