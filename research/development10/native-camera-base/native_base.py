"""Standalone actual874B40 constructor, original/v14/allPC, nativeknown masks.
All constructor/transform/plane/viewprojection bodies execute. Allocation is
the sole transported leaf, with explicit fills; pointer/vtable ABI words are
reported separately, never portable scalar fields. Source vector reads are
laboratory current inputs and can fail before their native load completes.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_EIP
ORIGINAL,V14=h.P,h.V
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
def run(pc,c):
 m=h.Machine(b'',pc);camera=m.alloc(0x394,c['fill']);known=[0]*(0x394//4);trace=[];read_pcs=set();outcome='ok'
 def write(u,access,a,size,value,data):
  for p in range(max(camera,a),min(camera+0x394,a+size)):known[(p-camera)//4]|=0xff<<(8*((p-camera)&3))
 def read(u,access,a,size,value,data):
  nonlocal outcome
  if a in(0xb43cf8,0xb43cfc,0xb43d00)and size==4:
   read_pcs.add(u.reg_read(UC_X86_REG_EIP));v=c['seeds'][(a-0xb43cf8)//4];trace.append([a,v])
   if len(trace)==c['fail']:
    outcome='required global';m.outcome={'kind':outcome};raise h.Throw()
 m.u.hook_add(UC_HOOK_MEM_WRITE,write);m.u.hook_add(UC_HOOK_MEM_READ,read)
 for i,v in enumerate(c['seeds']):m.w(0xb43cf8+4*i,v)
 result=m.call(0x874b40,camera,[])
 assert not m.null_accesses,m.null_accesses
 if outcome=='ok':assert result==camera
 #Process-global Niinstance bookkeeping is actualexecuted native evidence,
 #not a portable ABI counter or capacity that the Rust camera exposes.
 pointer_offsets=[0,0xa0,0x168,0x16c,0x180,0x184]
 words=[m.r(camera+4*i)if 4*i not in pointer_offsets else None for i in range(0x394//4)]
 arrays=[]
 for start in(0x168,0x180):
  if known[start//4+1]==0xffffffff:
   data=m.r(camera+start+4);capacity=m.r(camera+start+8)
   arrays.append(dict(offset=start,capacity=capacity,used=m.r(camera+start+12),effective=m.r(camera+start+16),grow=m.r(camera+start+20),null_slots=[m.r(data+4*i)==0 for i in range(capacity)]))
 return dict(words=words,known=known,trace=trace,outcome=outcome,arrays=arrays),read_pcs
def main():
 cases=[dict(fill=f,seeds=s,fail=None)for f,s in itertools.product((0,0xa5,0xff),([0,0,0],list(map(h.f32,[3,-4,5]))))]
 cases.extend(dict(fill=0xa5,seeds=list(map(h.f32,[.125,-.25,.5])),fail=i)for i in range(1,28))
 h.VISITED.clear();rows=[];reads=set()
 for pc,c in itertools.product(h.CW,cases):
  outputs=[]
  for image in(ORIGINAL,V14):
   h.P=image;output,r=run(pc,c);outputs.append(output);reads|=r
  assert outputs[0]==outputs[1];rows.append(dict(precision=pc,input=c,output=outputs[0]))
 h.P=ORIGINAL;excluded={0x6f7810}|reads
 pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in excluded}
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,pins=pins,global_read_pcs=[hex(a)for a in sorted(reads)],seams=['6F7810 growable allocation/free transport'],
  excluded_numeric_pointer_offsets=[0,0xa0,0x168,0x16c,0x180,0x184],domain='valid finite seeds, successful array allocation; Rust panic/poison and native allocation failure excluded')
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-base.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
