"""Actual86BA10 mode2 including870720 and unconditional plane tail.
No emulated child or allocation seams. Laboratory numeric fields are explicit;
required-read interruptions stop BEFORE the named current field is consumed.
Only finite successful arithmetic is a portable success-domain claim.
"""
from pathlib import Path
import sys,json,itertools,hashlib,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
ORIGINAL,V14=h.P,h.V
H=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

def run(pc,c):
 m=h.Machine(b'',pc);effect=m.alloc(0x190,0xa5);reads=[];writes=[];outcome='ok'
 for i,w in enumerate(c['words']):m.w(effect+4*i,w)
 def read(u,access,a,size,value,data):
  nonlocal outcome
  if effect<=a<effect+0x190:
   assert size==4
   reads.append(a-effect)
   if len(reads)==c['fail']:
    outcome='required field';m.outcome={'kind':outcome};raise h.Throw()
 def write(u,access,a,size,value,data):
  if effect<=a<effect+0x190:
   assert size==4;writes.append([a-effect,value&0xffffffff])
 m.u.hook_add(UC_HOOK_MEM_READ,read);m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 m.call(0x86ba10,effect,[])
 assert not m.null_accesses,m.null_accesses
 return dict(outcome=outcome,reads=reads,writes=writes,words=[m.r(effect+4*i)for i in range(0x190//4)])

def case(rotation,translation,scale,plane,counter=0):
 words=[0xa5a5a5a5]*(0x190//4)
 for i,v in enumerate(rotation+translation+[scale]):words[0x6c//4+i]=h.f32(v)if isinstance(v,float)else v
 for i,v in enumerate(plane):words[0x16c//4+i]=h.f32(v)if isinstance(v,float)else v
 words[0x164//4]=2;words[0xd0//4]=counter
 return dict(words=words,fail=None)

def main():
 identity=list(map(h.f32,[1,0,0,0,1,0,0,0,1]))
 cases=[case(identity,list(map(h.f32,[0,0,0])),h.f32(1),list(map(h.f32,[1,0,0,0]))),
  case(identity,[0x80000000,0x80000000,0x80000000],h.f32(-1),[0x80000000,0,0x80000000,0],0xffffffff),
  case(list(map(h.f32,[1,1,1,1,-1,1,1,1,-1])),list(map(h.f32,[2**24,-2**24,1])),h.f32(1),list(map(h.f32,[2**24,1,-2**24,1])),0x7fffffff),
  case([1,0x80000001,0x007fffff,0x00800000,0x80800000,0x80000000,0,0x3f7fffff,0x3f800001],[1,0x80000001,0],h.f32(.5),[1,0x80000001,h.f32(.125),0x00800000],0xfffffffe)]
 rng=random.Random(0x86ba10)
 for _ in range(16):
  values=[h.f32(rng.uniform(-16,16))for _ in range(17)]
  cases.append(case(values[:9],values[9:12],values[12],values[13:17],rng.randrange(2**32)))
 #Every reached read of a nontrivial case is independently interrupted. The
 #portable trait intentionally reports missing fields at those same points.
 n=len(run('Double53',cases[2])['reads'])
 cases.extend(dict(words=cases[2]['words'],fail=i)for i in range(1,n+1))
 h.VISITED.clear();rows=[]
 for pc,c in itertools.product(h.CW,cases):
  out=[]
  for image in(ORIGINAL,V14):h.P=image;out.append(run(pc,c))
  assert out[0]==out[1];rows.append(dict(precision=pc,input=c,output=out[0]))
 h.P=ORIGINAL
 doc=dict(schema=1,scope=__doc__,producer_sha256=H(Path(__file__)),rows=rows,pins={hex(a):v for a,v in sorted(h.VISITED.items())},seams=[],domain='mode2, valid typed nonalias fields, finite successful arithmetic, masked x87 round-nearest precision explicit; native trap/Inf/NaN semantics excluded')
 data=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-projection.json'
 if '--write'in sys.argv:p.write_bytes(data)
 else:assert p.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(doc['pins']),reads=n,sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
