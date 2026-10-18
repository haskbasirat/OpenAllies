"""Native4F57F0 full no-merge path and exact first positive4F48E0 cutoff.
Actual major-slot iterator4A5BC0/4F0680, CString pointer/length tests and full
normal return run unchanged. Allocation/free and SyncA marker transport are
explicit seams;77DC50's arithmetic has a separate native proof. Positive alias
lookup is a REQUIRED boundary and never returns a guessed Kingdom.
"""
from pathlib import Path
import hashlib,itertools,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ
ORIGINAL=h.P
class Required(Exception):pass

def run(image,pc,values):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);seams=set();events=[];actors={}
 def seam(a,f):m.seams[a]=f;seams.add(a)
 seam(0x6f7810,lambda _:m.ret(value=m.alloc(m.arg())))
 seam(0x6f7430,lambda _:m.ret())
 def log(_):
  assert m.arg(1)==0
  p=m.arg();b=bytearray()
  while bytes(m.u.mem_read(p,2))!=b'\0\0':b+=bytes(m.u.mem_read(p,2));p+=2
  assert b.decode('utf-16le')=='MergeKingdoms';events.append('marker');m.ret(8)
 seam(0x77dc50,log)
 def required(_):events.append('required_4F48E0');raise Required()
 seam(0x4f48e0,required)
 world=m.alloc(0x300);data=m.alloc(max(1,len(values))*4);m.w32(world+0x154,data,len(values),len(values))
 for i,row in enumerate(values):
  if row=='hole':continue
  actor=m.alloc(0x400);actors[actor+0x248]=i;m.w32(data+i*4,actor)
  if row is not None:
   string=m.alloc(16+max(1,len(row))*2);m.w32(string+4,len(row));ptr=string+16;m.w32(actor+0x248,ptr)
   if row:m.u.mem_write(ptr,b''.join(int(x).to_bytes(2,'little')for x in row))
 def read(u,access,a,n,v,_):
  if a in actors:events.append(['name',actors[a]])
 m.u.hook_add(UC_HOOK_MEM_READ,read);outcome='ok'
 try:m.call(0x4f57f0,world)
 except Required:outcome='required_4F48E0'
 return {'events':events,'outcome':outcome},seams

def main():
 cases=[];seams=set()
 inputs=[[],['hole'],[None], [[],None,'hole',[]],['hole',None,'hole',[],'hole'],[[],[ord('x')],[]],[[0],[]]]
 for image,pc,c in itertools.product(('original','v14'),h.PRECISION,inputs):
  result,used=run(image,pc,c);seams|=used;cases.append({'image':image,'precision':pc,'input':c,'output':result})
 pins=[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams]
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'executed_instructions':pins,
  'seams_not_executed':[hex(a)for a in sorted(seams)],'cases':cases}
 p=HERE/'native-merge-kingdoms.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'cases':len(cases),'executed_pins':len(pins),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()

