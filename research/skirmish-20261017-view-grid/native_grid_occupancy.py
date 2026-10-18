"""Actual42FD10/42F860 and430E20/430410/42FD40 on one constructed VGrid.
Heap and warning output are declared transports. The growable heap transport
reuses freed8-byte nodes LIFO; live pointer/list comparisons execute natively.
Pre-instruction hooks stop before logical view/list stores and services.
Read+write tags on one OR/AND instruction share its pre-instruction boundary.
Only finite, representable coordinate/radius arithmetic is admitted here.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
import unicorn.x86_const as X
from capstone import Cs,CS_ARCH_X86,CS_MODE_32,CS_AC_READ,CS_AC_WRITE
from capstone.x86 import X86_OP_MEM
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stop(Exception):pass
class Machine(L.Loader):
 def __init__(self,*args):self.replacements={};super().__init__(*args)
 def seam(self,address,fn):
  if address in(0x6f7810,0x6f7430,0x404f60):super().seam(address,lambda:self.replacements.get(address,fn)())
  else:super().seam(address,fn)
def run(c,image,pc,stop=None):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=Machine(PC[pc])
 reference=json.loads((HERE/'hardware-grid-seed-reference.json').read_text())
 for address,value in zip(reference['addresses'],reference['words']):m.w(address,value)
 grid=m.alloc(0x110);m.call(0x430290,grid);m.call(0x42f540,grid,[0x42000000,0x41c00000,2]);base=m.r(grid);count=m.r(grid+0x1c)*m.r(grid+0x20)
 views=[]
 for i,v in enumerate(c['views']):
  a=m.alloc(0x100);thing=m.alloc(0x20);m.w(thing+8,m.r(m.cstring('explicit-view')));m.w(a+0x10,thing)
  for offset,key in[(0x38,'x'),(0x3c,'y'),(0x74,'r'),(0x54,'flags'),(0x68,'count')]:m.w(a+offset,v[key])
  views.append(a)
 pointers={};nodes=[];free=[];active=False;events=[];counts={}
 def event(label):
  if not active:return
  key=f'{label}:{counts.get(label,0)}';counts[label]=counts.get(label,0)+1
  if stop==key:raise Stop(key)
  events.append(key)
 def allocate():
  assert m.arg(0)==8;event('allocate')
  if free:q=free.pop();index=pointers[q];nodes[index]=dict(address=q,alive=True,value=False,next=False)
  else:q=m.alloc(8);pointers[q]=len(nodes);nodes.append(dict(address=q,alive=True,value=False,next=False))
  m.ret(0,q)
 def release():
  event('free');q=m.arg(0);n=nodes[pointers[q]];assert n['alive'];n['alive']=False;free.append(q);m.ret(0)
 def warning():event('warning');m.ret(0)
 m.replacements={0x6f7810:allocate,0x6f7430:release,0x404f60:warning}
 cs=Cs(CS_ARCH_X86,CS_MODE_32);cs.detail=True;decoded={}
 def reg(u,name):return u.reg_read(getattr(X,'UC_X86_REG_'+name.upper()))
 def before(u,at,size,_):
  if u.reg_read(X.UC_X86_REG_EIP)!=at:return
  if not any(a<=at<b for a,b in[(0x42fd10,0x42fd3c),(0x42f860,0x42f980),(0x430e20,0x430e4c),(0x430410,0x430600),(0x42fd40,0x42fe70)]):return
  ins=decoded.get(at)
  if ins is None:ins=next(cs.disasm(bytes(u.mem_read(at,size)),at));decoded[at]=ins
  for op in ins.operands:
   if op.type!=X86_OP_MEM:continue
   mem=op.mem;address=mem.disp
   if mem.base:address+=reg(u,ins.reg_name(mem.base))
   if mem.index:address+=reg(u,ins.reg_name(mem.index))*mem.scale
   address&=0xffffffff
   #Only the actual call's VThing is a scalar source/destination.
   if address in [views[current]+o for o in[0x38,0x3c,0x74,0x54,0x68]]:
    offset=address-views[current]
    if op.access&CS_AC_READ:event(f'vread{offset:x}')
    if op.access&CS_AC_WRITE:event(f'vstore{offset:x}')
   if op.access&CS_AC_WRITE:
    if base<=address<base+count*8 and (address-base)%4==0:event('head'if(address-base)%8==0 else'tail')
    for q,index in pointers.items():
     if address==q:event('node_view');nodes[index]['value']=True
     elif address==q+4:event('node_next');nodes[index]['next']=True
 m.u.hook_add(UC_HOOK_CODE,before)
 def call(op):
  nonlocal current
  current=op['view']
  for offset,key in[(0x38,'x'),(0x3c,'y'),(0x74,'r')]:
   if key in op:m.w(views[current]+offset,op[key])
  m.call(0x42fd10 if op['op']=='occupy'else 0x430e20,grid,[views[current]])
 current=0
 for op in c['before']:call(op)
 active=True;stopped=None
 try:call(c['call'])
 except Stop as e:stopped=str(e)
 def node_id(q):return None if not q else pointers[q]+1
 projected=[]
 for n in nodes:
  if not n['alive']:projected.append(None);continue
  q=n['address'];projected.append(dict(view=views.index(m.r(q))+1 if n['value']else None,
   next=node_id(m.r(q+4))if n['next']else'unknown'))
 out=dict(events=events,stopped=stopped,cells=[[node_id(m.r(base+8*i)),node_id(m.r(base+8*i+4))]for i in range(count)],nodes=projected,
  free=[pointers[q]+1 for q in free],views=[dict(flags=m.r(a+0x54),count=m.r(a+0x68))for a in views])
 return out,m.windows()
def cases():
 a=dict(x=0x40f00000,y=0x40a80000,r=0x40200000,flags=0,count=0)
 b=dict(x=0x41000000,y=0x40c00000,r=0x3f800000,flags=0x80000001,count=0)
 def case(before,call,views=None):return dict(views=views or[a,b],before=before,call=call)
 yield case([],dict(op='occupy',view=0))
 yield case([dict(op='occupy',view=0)],dict(op='occupy',view=1))
 yield case([dict(op='occupy',view=0),dict(op='occupy',view=1)],dict(op='unoccupy',view=0))
 yield case([dict(op='occupy',view=0),dict(op='occupy',view=1)],dict(op='unoccupy',view=1))
 yield case([dict(op='occupy',view=0),dict(op='occupy',view=1),dict(op='occupy',view=0)],dict(op='unoccupy',view=1))
 yield case([dict(op='occupy',view=0),dict(op='occupy',view=0)],dict(op='unoccupy',view=0))
 yield case([dict(op='occupy',view=0)],dict(op='unoccupy',view=0,x=0x41e00000,y=0x41a00000))
 yield case([dict(op='occupy',view=0),dict(op='occupy',view=1)],dict(op='unoccupy',view=0,x=0x41e00000,y=0x41a00000))
 yield case([dict(op='occupy',view=0),dict(op='unoccupy',view=0)],dict(op='occupy',view=1))
 yield case([],dict(op='occupy',view=0),[dict(a,x=0x80000000,y=0xc0800000,r=0),b])
 yield case([],dict(op='occupy',view=0),[dict(a,x=0x42c80000,y=0x42c80000,r=0xbf800000),b])
 yield case([],dict(op='occupy',view=0),[dict(a,x=0x40800000,y=0x40800000,r=0x34000000),b])
def main():
 rows=[];pins={}
 for c in cases():
  baseline,_=run(c,'original','Double53')
  for stop in [None]+baseline['events']:
   for pc in PC:
    a,pa=run(c,'original',pc,stop);b,pb=run(c,'v14',pc,stop);assert a==b,(c,pc,stop,a,b)
    rows.append(dict(input=c,pc=pc,stop=stop,output=a));pins.update({p['address']:p for p in pa+pb})
 out=dict(schema=1,scope=__doc__,cases=rows,blocks=[pins[k]for k in sorted(pins)])
 raw=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-grid-occupancy.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
