"""Actual44FDB0 scalar/list tail, after separately proved453F10 base.
Only the base and6F7810 allocator are seams. Poisoned heaps distinguish the
allocated but unwritten fourteen slots and byte16C from its unwritten padding.
The allocator-entry cutoff is an observed prefix, not a native exception/OOM.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn import UC_HOOK_MEM_WRITE
ORIGINAL,V14=h.P,h.V
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def run(pc,c):
 m=h.Machine(b'',pc);view=m.alloc(0x174,c['fill']);data=m.alloc(0x10);thing=m.alloc(0x10)
 known=set();events=[];allocation=None
 def writes(u,access,address,size,value,context):known.update(range(address,address+size))
 m.u.hook_add(UC_HOOK_MEM_WRITE,writes)
 def base(m):
  assert m.arg(0)==data and m.arg(1)==thing
  events.append(['base453F10']);m.ret(8)
 def alloc(m):
  nonlocal allocation
  assert m.arg(0)==0x38;events.append(['allocate',0x38])
  if c['cutoff']:
   m.outcome={'kind':'allocator-entry-cutoff'};raise h.Throw()
  allocation=m.alloc(0x38,c['heap_fill']);m.ret(0,allocation)
 m.seam(0x453f10,base);m.seam(0x6f7810,alloc)
 m.call(0x44fdb0,view,[data,thing]);assert not m.null_accesses,m.null_accesses
 assert not m.calls,m.calls
 words={};masks={}
 for o in range(0x158,0x174,4):
  mask=sum(0xff<<(8*i)for i in range(4)if view+o+i in known)
  masks[str(o)]=mask;words[str(o)]=(m.r(view+o)&mask)if mask else None
 if allocation:
  assert m.r(view+0x158)==allocation;words[str(0x158)]=None
  assert bytes(m.u.mem_read(allocation,0x38))==bytes([c['heap_fill']])*0x38
  assert not any(allocation+i in known for i in range(0x38))
 assert m.r(view)==0xa01ef0
 return dict(events=events,words=words,known_masks=masks,allocated=allocation is not None,
             byte_16c=m.b(view+0x16c)if view+0x16c in known else None,
             outcome='allocator-entry-cutoff'if c['cutoff']else'ok')

def main():
 cases=[dict(fill=fill,heap_fill=heap,cutoff=cutoff)for fill,heap in[(0xa5,0xa5),(0,0xff),(0xff,0),(0x37,0x91)]for cutoff in[False,True]]
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.CW,cases):
  outputs=[]
  for image in(ORIGINAL,V14):h.P=image;outputs.append(run(pc,c))
  assert outputs[0]==outputs[1];rows.append(dict(precision=pc,input=c,output=outputs[0]))
 h.P=ORIGINAL;pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in(0x453f10,0x6f7810)}
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,pins=pins,
          constructor_vtable='0xa01ef0',seams=['453F10 separately retained Actor base','6F7810 allocation transport'],
          domain='successful typed allocation; allocator-entry prefix only for portable refusal. No native OOM/unwind/destructor/fullInit/ABI or publication claim')
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-organization-fields.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
