from pathlib import Path
import sys,json,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n
rows=[];pins={}
for pc,poison in itertools.product(n.PRECISIONS,[0,0xffffffff]):
 outputs=[]
 for image in [n.original,n.modified]:
  m=n.Machine(pc)
  old=(n.original.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
  if new>old:m.u.mem_map(image.base+old,new-old)
  for _,rva,size,off in image.sections:m.u.mem_write(image.base+rva,image.b[off:off+size])
  owner=m.alloc(0x130);m.w(owner+0x128,poison);m.w(owner+0x12c,poison)
  #Exact EBX producer from5B0400, no callers guessed: XOR EBX,EBX.
  m.u.reg_write(n.UC_X86_REG_EBX,0xdeadbeef);m.u.emu_start(0x5b0435,0x5b0437,count=1)
  assert m.u.reg_read(n.UC_X86_REG_EBX)==0
  m.u.reg_write(n.UC_X86_REG_ESI,owner)
  m.u.emu_start(0x5b05b8,0x5b05c4,count=2)
  outputs.append([m.r(owner+0x128),m.r(owner+0x12c)])
  pins.update(m.visited)
 assert outputs[0]==outputs[1]==[0,0];rows.append(dict(precision=pc,poison=poison,output=outputs[0]))
for a,b in pins.items():assert n.original.range(a,len(b))==n.modified.range(a,len(b))==b
out=dict(scope='Bounded actual constructor5B0435 EBX producer and5B05B8..C4 empty waiter-list stores only; no thread/manager constructor success claim. No later waiter insertion producer admitted.',rows=rows,native_executions=len(rows)*2,instructions=n.instructions(pins),original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b))
p=HERE/'native-queue-waiter-constructor.json'
if '--write'in sys.argv:
 with p.open('x')as f:json.dump(out,f,indent=2);f.write('\n')
else:assert json.loads(p.read_text())==out
print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=n.sha(p.read_bytes()))))
