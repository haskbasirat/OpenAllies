"""Actual453F10 tail with separately owned455050 base seam, allocator/free
transport only. Native source virtual38,57DBE0/45DC90,766DE0/4849F0 and
both list loops execute. Reached Sources refusals are explicit pre-read/entry
cutoffs, not claims of native exceptions. Source-count changes are external
storage interventions, not invented application callbacks.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
ORIGINAL,V14=h.P,h.V
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
GLOBALS=[0xb322a4,0xb322a8,0xb322ac]*2+[0xb43cf8,0xb43cfc,0xb43d00]
def run(pc,c):
 m=h.Machine(b'',pc);view=m.alloc(0x158,0xa5);data=m.alloc(0x300,0xa5);thing=m.alloc(0x40);known=set();events=[];outcome='ok';global_at=0;counts={0x1fc:0,0x244:0}
 sourcevt=0xa05540 if c['actor']else 0xa13318;m.w(data,sourcevt)
 pitches=m.alloc(4*len(c['pitches']))
 for i,v in enumerate(c['pitches']):p=m.alloc(0x30,0xa5);m.w(p+0x14,v);m.w(pitches+4*i,p)
 m.w(data+0x1f8,pitches);m.w(data+0x1fc,c['turret_counts'][0]);m.w(data+0x244,c['secondary_counts'][0]);m.w(data+0x23c,c['source_word'])
 def event(name,*args):
  nonlocal outcome
  events.append([name,*args])
  if len(events)==c['fail']:
   outcome=name;m.outcome={'kind':outcome};raise h.Throw()
 def reads(u,access,a,size,value,context):
  nonlocal global_at
  if a in GLOBALS and size==4:
   assert a==GLOBALS[global_at];v=c['globals'][global_at];global_at+=1;m.w(a,v);event('global',a,v)
  if a in(data+0x1fc,data+0x244)and size==4:
   off=a-data;seq=c['turret_counts']if off==0x1fc else c['secondary_counts'];n=counts[off];counts[off]+=1;v=seq[min(n,len(seq)-1)];m.w(a,v);event('turret-count'if off==0x1fc else'secondary-count',v)
  if a==data+0x23c and size==4:event('source-word',c['source_word'])
 def writes(u,access,a,size,value,context):known.update(range(a,a+size))
 m.u.hook_add(UC_HOOK_MEM_READ,reads);m.u.hook_add(UC_HOOK_MEM_WRITE,writes)
 def base(m):assert m.arg(0)==data and m.arg(1)==thing;m.ret(8)
 m.seam(0x455050,base)
 getter=int.from_bytes(h.P.range(sourcevt+0x38,4),'little');m.probes[getter]=lambda m:event('actor',c['actor'])
 def pitch(m):assert m.arg(0)==data;event('pitch',m.arg(1),c['pitches'][m.arg(1)])
 m.probes[0x57dbe0]=pitch
 m.call(0x453f10,view,[data,thing]);assert not m.null_accesses,m.null_accesses
 assert not m.calls,m.calls
 words={str(o):m.r(view+o)if all(view+o+i in known for i in range(4))else None for o in range(0xbc,0x158,4)}
 arrays=[]
 for offset,size in[(0x128,16),(0x134,12)]:
  allocated=None;records=[]
  if all(view+offset+i in known for i in range(12)):
   pointer=m.r(view+offset);allocated=pointer!=0;count=m.r(view+offset+4)
   if pointer:
    words[str(offset)]=None
    for i in range(count):
     p=m.r(pointer+4*i);assert p
     masks=[sum(0xff<<(8*j)for j in range(4)if p+o+j in known)for o in range(0,size,4)]
     records.append(dict(words=[m.r(p+o)&masks[o//4]for o in range(0,size,4)],known=masks))
  arrays.append(dict(allocated=allocated,records=records))
 return dict(outcome=outcome,events=events,words=words,arrays=arrays)
def main():
 base=dict(actor=True,globals=[0x3f800000,0,0x80000000,0x3f000000,0x7fc12345,0x00800000,0,0x80000000,0x7fa12345],turret_counts=[0],secondary_counts=[0],pitches=[h.f32(v)for v in range(20)],source_word=0x87654321,fail=None)
 cases=[dict(base,actor=False),dict(base,turret_counts=[-1],secondary_counts=[-5])]
 for count,secondary in [(0,0),(1,1),(2,3),(3,4),(5,7),(12,11)]:cases.append(dict(base,turret_counts=[count],secondary_counts=[secondary],pitches=[0x80000000,1,0x007fffff,h.f32(.5),h.f32(-4)]+[h.f32(v)for v in range(15)]))
 cases += [dict(base,turret_counts=[3,1],secondary_counts=[2,0]),dict(base,turret_counts=[1,3,3,3],secondary_counts=[1,4,4,4,4])]
 sample=cases[5];n=len(run('Double53',sample)['events']);cases.extend(dict(sample,fail=i)for i in range(1,n+1))
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.CW,cases):
  outputs=[]
  for image in(ORIGINAL,V14):h.P=image;outputs.append(run(pc,c))
  assert outputs[0]==outputs[1];rows.append(dict(precision=pc,input=c,output=outputs[0]))
 h.P=ORIGINAL;seams={0x455050,0x6f7810,0x6f7430};pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in seams}
 getters={}
 for vt,target in[(0xa05540,0x4f0500),(0xa13318,0x4c9fa0)]:
  assert ORIGINAL.range(vt+0x38,4)==V14.range(vt+0x38,4)==target.to_bytes(4,'little');getters[hex(vt+0x38)]=hex(target)
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,pins=pins,source_virtual38=getters,seams=['455050 separately owned base','6F7810 allocator transport','6F7430 free transport'],domain='valid typed source arrays, finite pitch binary32, successful growable allocation; no native OOM/ABI/destructor claim')
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-actor-fields.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),source_cutoffs=n,sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
