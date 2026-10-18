"""Actual5A6D10/5A5070 and5A9330/5A6340 fields; read-only default replay.
Allocator/free, mutex and diagnostic transport; loaded source child is a cutoff.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
class Required(Exception):pass

def run(c,image,pc):
 m=n.Machine(pc)
 if image=='v14':
  old=(n.original.image_size+0xfff)&~0xfff;new=(n.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(n.original.base+old,new-old)
  for vs,rva,raw,off in n.modified.sections:m.u.mem_write(n.modified.base+rva,n.modified.b[off:off+raw])
 events=[];objects=[m.alloc(0x100),m.alloc(0x100)];which={v:i for i,v in enumerate(objects)}
 def alloc(mm):
  size=mm.arg(1);p=mm.alloc(size);mm.u.mem_write(p,b'\xcd'*size);events.append(['alloc',size]);mm.ret(p,pop=0)
 def free(mm):events.append(['free']);mm.ret(0,pop=0)
 m.seam(0x6f7810,alloc);m.seam(0x6f7430,free)
 m.call(0x9e75f0)
 fmt=[m.r(0xb43d2c+i*4)for i in range(9)]
 for index,p in enumerate(objects):
  m.u.mem_write(p,b'\0'*0x100);m.w(p+8,c['flags'][index]);m.w(p+4,c['requests'][index]&0xffffffff)
 def writable(mm):events.append(['writable',which.get(m.r(mm.ecx()+4),-1)]);mm.ret(c['writable'])
 for index,p in enumerate(objects):
  if c['locations'][index]:
   v=m.alloc(16);vt=m.alloc(32);m.w(v,vt);m.w(v+4,p);m.w(vt+12,m.stub(writable));a=m.alloc(4);m.w(a,v);m.w(p+0x24,a);m.w(p+0x28,1)
 m.w(objects[0]+0x40,objects[1] if c['child'] else 0)
 count=c['count'];cap=c['capacity'];slots=m.alloc(max(cap,1)*36)
 for i in range(cap):
  for j in range(9):m.w(slots+i*36+j*4,(0x11000000+i*0x100+j)if c['poison_slots']else[2,0,0,0,0,0,8,0x22,0][j])
 m.w(objects[0]+0x5c,slots if cap else 0);m.w(objects[0]+0x60,count);m.w(objects[0]+0x64,cap)
 def lock(mm):
  events.append(['lock']);target=objects[1]if c['child']and c['locations'][0]and c['writable']else objects[0]
  m.w(target+8,m.r(target+8)^c['lock_toggle']);mm.ret(0)
 def unlock(mm):events.append(['unlock']);mm.ret(0)
 def warning(mm):
  events.append(['warning',mm.arg(3)])
  if c['warning_error']:raise Required('warning')
  mm.ret(0,pop=0)
 def loaded(mm):events.append(['loaded',c['raw']&255!=0]);raise Required('loaded')
 m.seam(0x857e10,lock);m.seam(0x857de0,unlock);m.seam(0x404ed0,warning);m.seam(0x5a8b40,loaded)
 #Loaded disable-reading invokes actual Source+48. Its body is explicitly cut.
 for p in objects:
  source=m.alloc(0x60);vt=m.alloc(0x60);m.w(source,vt);m.w(vt+0x48,m.stub(loaded));m.w(p+0x3c,source)
 stopped=None
 try:
  if c['push']:m.call(0x5a6d10,objects[0],[0xb43d2c])
  if c['operation']=='radar':
   m.call(0x5a9330,objects[0],[1]);m.call(0x5a6340,objects[0],[0])
  else:m.call(0x5a9330 if c['operation']=='reading'else 0x5a6340,objects[0],[c['raw']])
 except Required as e:stopped=str(e)
 cap=m.r(objects[0]+0x64);data=m.r(objects[0]+0x5c)
 output=dict(flags=[m.r(p+8)for p in objects],count=m.r(objects[0]+0x60),capacity=cap,
  slots=[[m.r(data+i*36+j*4)for j in range(9)]for i in range(cap)],events=events,stopped=stopped,format=fmt)
 pins={f'0x{a:08x}':raw.hex()for a,raw in m.visited.items()}
 return output,pins

def cases():
 base=dict(flags=[0x200000,0],requests=[0,0],locations=[0,0],writable=0,child=False,count=0,capacity=1,poison_slots=False,push=False,operation='radar',raw=1,warning_error=False,lock_toggle=0)
 for cap,count in [(0,0),(1,0),(1,1),(2,1),(2,2),(4,3),(4,4)]:
  for flags,requests in [(0x200000,0),(0x300000,1),(0x200001,0)]:
   yield {**base,'push':True,'capacity':cap,'count':count,'poison_slots':True,'flags':[flags,0],'requests':[requests,0]}
 for operation in ['reading','rendering']:
  for flags,raw,requests in itertools.product([0,1,0x100000,0x200000,0x300000],[0,1,256,257,2],[-1,0,1]):
   yield {**base,'operation':operation,'flags':[flags,0],'raw':raw,'requests':[requests,0]}
 for operation,writable,child,second in itertools.product(['reading','rendering'],[0,1],[False,True],[0,0x300000]):
  yield {**base,'operation':operation,'flags':[0x300000,second],'locations':[1,1],'writable':writable,'child':child,'raw':0}
 for operation in ['reading','rendering']:
  yield {**base,'operation':operation,'flags':[0x300000,0],'requests':[1,0],'raw':0,'warning_error':True}
 yield {**base,'operation':'reading','lock_toggle':0x400}

def main():
 rows=[];pins={}
 for pc in n.PRECISIONS:
  for index,c in enumerate(cases()):
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc)
   assert a==b,(pc,index,a,b)
   for k,v in pa.items():assert pb.get(k)==v,(k,v,pb.get(k))
   pins.update(pa);rows.append(dict(pc=pc,input=c,output=a))
  print(pc,len(rows),flush=True)
 result=dict(rows=rows,pins=pins,images=dict(original=hashlib.sha256(n.original.b).hexdigest(),v14=hashlib.sha256(n.modified.b).hexdigest()))
 dest=HERE/'native-texture-settings.json'
 if '--write'in sys.argv:
  with dest.open('x',encoding='utf-8')as f:json.dump(result,f,indent=2)
 else:assert result==json.loads(dest.read_text()),'immutable replay differs'
 print('PASS',len(rows),'rows',len(rows)*2,'executions',len(pins),'pins',flush=True)
if __name__=='__main__':main()

