"""Native89FBF0 sphere transform,89FAD0 ordered merge and5A96B2 selection
geometry arithmetic. Inputs are explicit finite source sphere/vertex/transform
words. Selection stops BEFORE6FF850 parent unlink; source named lookup and
Ni allocation/Scene traversal are not certified by this scalar fixture.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBP,UC_X86_REG_ESP
f=lambda v:struct.unpack('<I',struct.pack('<f',v))[0]
I=[f(1),0,0,0,f(1),0,0,0,f(1),0,0,0,f(1)]

def run(m,pc,mode,a,b):
 m.fpu(pc);m.actual={};m.record_actual=True
 if mode=='transform':
  out=m.alloc(16);src=m.alloc(16);transform=m.alloc(52)
  for j,w in enumerate(a):m.w(src+4*j,w)
  for j,w in enumerate(b):m.w(transform+4*j,w)
  m.call(0x89fbf0,[src,transform],this=out,callee_pop=8)
  result=[m.r(out+4*i)for i in range(4)]
 elif mode=='merge':
  out=m.alloc(16);other=m.alloc(16)
  for j,w in enumerate(a):m.w(out+4*j,w)
  for j,w in enumerate(b):m.w(other+4*j,w)
  m.call(0x89fad0,[other],this=out,callee_pop=4)
  result=[m.r(out+4*i)for i in range(4)]
 elif mode=='selection':
  g=m.alloc(0x110);data=m.alloc(0x40);vt=m.alloc(0x40);resource=m.alloc(0xa0)
  stub=m.alloc(8);m.u.mem_write(stub,bytes.fromhex('668b410cc3'));m.w(vt+0x38,stub)
  m.w(data,vt);m.w(data+0xc,len(a));array=m.alloc(max(1,len(a))*12)
  for j,v in enumerate(a):
   for k,w in enumerate(v):m.w(array+j*12+k*4,w)
  m.w(data+0x20,array);m.w(g+0xcc,data)
  for j,w in enumerate(b):m.w(g+0x6c+4*j,w)
  m.u.reg_write(UC_X86_REG_ESI,g);m.u.reg_write(UC_X86_REG_EBP,resource);m.u.reg_write(UC_X86_REG_ESP,m.stack)
  m.u.emu_start(0x5a96b2,0x5a97f9,count=100000)
  result=[m.r(resource+0x4c+4*i)for i in range(6)]
 return result,m.actual

def main():
 rows=[];pins={};cases=[]
 transforms=[I,I[:9]+[f(1.25),f(-2.5),f(.125),f(.75)], [f(.6),f(-.8),0,f(.8),f(.6),0,0,0,f(1),f(.01),f(.02),f(.03),0x3f800001]]
 spheres=[[0,0,0,f(1)],[f(1.25),f(-2.5),f(.125),f(.75)],[0x3f800001,0xbf7fffff,f(1e-9),f(1e-15)]]
 for t in transforms:
  for s in spheres:cases.append(('transform',s,t))
 for a,b in [(spheres[0],spheres[0]),(spheres[0],[0,0,0,f(3)]),([0,0,0,f(3)],spheres[0]),(spheres[0],[f(4),f(3),0,f(1)]),(spheres[1],spheres[2]),(spheres[2],spheres[1]),([f(1e-8),0,0,f(1)],[0,0,0,f(1)])]:cases.append(('merge',a,b))
 for t in transforms:
  for v in [[],[[f(3),f(4),f(-.1)],[f(-5),f(2),f(.2)],[f(1),f(-2),f(3)]],[[0x3f800001,0x3f7fffff,0x80000000],[f(127.1),f(-129.3),f(-.001)]]]:cases.append(('selection',v,t))
 for image in ['original','v14']:
  m=Machine(image)
  for pc in PC:
   for mode,a,b in cases:
    result,actual=run(m,pc,mode,a,b);pins.update(actual)
    rows.append(dict(image=image,precision=pc,mode=mode,a=a,b=b,output=result))
 assert [{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original']==[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14']
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(scope=__doc__,merge_epsilon=int.from_bytes(PE0.range(0xb32d94,4),'little'),producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions)
 out=HERE/'native-model-spheres.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
