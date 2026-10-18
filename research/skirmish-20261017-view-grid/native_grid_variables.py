"""Real AVar Int constructors and43FCF1..FD35 current-cell metric reads.
4B3470 registration and named lookup are declared transports; native401650,
512160/4B29B0 flags0 setter and the signed-min/highest-bit loop execute.
Metrics use explicit current-cell laboratory inputs, not source defaults.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Done(Exception):pass
class Stop(Exception):pass
def var(self):self.ret(4,self.grid_cell)
L.Loader._var=var
def constructors(image,pc,values):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);out=[];ptrs={}
 def register():
  index=m.arg(0);v=m.arg(1);p=m.r(v+0x1c);name=[]
  while (c:=int.from_bytes(m.u.mem_read(p+2*len(name),2),'little')):name.append(c)
  name=''.join(map(chr,name));out.append(dict(index=index,name=name,current=m.r(v),kind=m.r(v+0x24),flags=m.r(v+0x28)))
  q=m.alloc(0x30);m.u.mem_write(q,bytes(m.u.mem_read(v,0x2c)));ptrs[name]=q;m.ret(8)
 m.seam(0x4b3470,register)
 def before(u,at,size,_):
  if at in(0x401c20,0x401f2c):raise Done()
 m.u.hook_add(UC_HOOK_CODE,before)
 for address in[0x401beb,0x401ef9]:
  owner=m.alloc(0x100);m.u.reg_write(UC_X86_REG_ESI,owner)
  try:m.call(address,owner)
  except Done:pass
 for (name,v),value in zip(ptrs.items(),values):
  cell=m.alloc(4);m.w(cell,v);m.grid_cell=cell;m.call(0x512160,0xb39988,[m.cstring(name),value])
 return dict(constructors=out,current=[m.r(v)for v in ptrs.values()]),m.windows()
def metrics(image,pc,c,stop=None):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);events=[];stopped=None
 a=m.alloc(0x40);u=m.alloc(4);m.w(0xb39990,a);m.w(0xb39940,u)
 for offset,value in [(0,c['terrain']),(0x18,c['grid'])]:q=m.alloc(4);m.w(q,value);m.w(a+offset,q)
 q=m.alloc(4);m.w(q,c['quad']);m.w(u,q)
 def before(u,at,size,_):
  labels={0x43fcf9:'grid',0x43fcfd:'terrain',0x43fd09:'quad'}
  if at in labels:
   key=labels[at]
   if stop==key:raise Stop(key)
   events.append(key)
  if at==0x43fd35:raise Done()
 m.u.hook_add(UC_HOOK_CODE,before)
 result=None
 try:m.call(0x43fcf1)
 except Done:result=dict(grid_shift=m.u.reg_read(UC_X86_REG_ECX),terrain_quad_size=m.u.reg_read(UC_X86_REG_EBP))
 except Stop as e:stopped=str(e)
 return dict(events=events,stopped=stopped,result=result),m.windows()
def main():
 rows=[];pins={}
 for values in[(8,4),(0,0),(0x80000000,0xffffffff),(0x7fffffff,0x12345678)]:
  for pc in PC:
   a,pa=constructors('original',pc,values);b,pb=constructors('v14',pc,values);assert a==b
   assert a['constructors']==[dict(index=0,name='TerrainBlockSize',current=16,kind=3,flags=0),dict(index=6,name='VGridCellSize',current=4,kind=3,flags=0)]
   assert a['current']==list(values);rows.append(dict(phase='constructors',pc=pc,input=values,output=a));pins.update({v['address']:v for v in pa+pb})
 for terrain,grid,quad in [(8,4,1),(4,8,2),(0,4,16),(16,0,32),(1,1,0),(8,3,8),(7,31,4),(32,63,64),(0xffffffff,8,4),(8,0xffffffff,4),(0x80000000,0x7fffffff,0xffffffff),(0x7fffffff,0x80000000,0x80000000),(0x7fffffff,0x7fffffff,0x7ffffffe)]:
  c=dict(terrain=terrain,grid=grid,quad=quad)
  for stop in[None,'grid','terrain','quad']:
   for pc in PC:
    a,pa=metrics('original',pc,c,stop);b,pb=metrics('v14',pc,c,stop);assert a==b
    rows.append(dict(phase='metrics',pc=pc,input=c,stop=stop,output=a));pins.update({v['address']:v for v in pa+pb})
 out=dict(schema=1,scope=__doc__,cases=rows,blocks=[pins[k]for k in sorted(pins)])
 raw=(json.dumps(out,indent=1)+'\n').encode();p=HERE/'native-grid-variables.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
