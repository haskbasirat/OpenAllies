"""Original4E15D0/4730D0/573650/4E2640 and random50EC66..50ECFA/474B40.
Alloc/free and77DA70 synchronization transport are explicit services.4E0F00 is
an explicit existing tested Commander constructor/destructor child seam; its
call order/arguments and native store prefix are observed, not reimplemented
inside the oracle. Raw data fixture has no source readiness authority.
"""
from pathlib import Path
import json,sys,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn.x86_const import *
def machine(image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 m.seam(0x6f7810,lambda mm:mm.ret(m.alloc(max(1,mm.arg(1)))))
 m.seam(0x6f7430,lambda mm:mm.ret())
 return m

def linked(m,where,values):
 head=0;tail=0
 for v in reversed(values):
  p=m.array([v,head]);head=p
  if not tail:tail=p
 m.w(where,head);m.w(where+4,tail)
def list_values(m,where):
 p=m.r(where);last=0;out=[]
 while p:out.append(m.r(p));last=p;p=m.r(p+4)
 assert m.r(where+4)==last
 return out

def run(c,image,pc):
 m=machine(image,pc);events=[]
 factions=[m.alloc(0x40)for _ in range(3)];reverse={p:i for i,p in enumerate(factions)}
 for i,p in enumerate(factions):m.u.mem_write(p+0x18,bytes([c.get('enabled',[1,1,1])[i]]))
 nations=[m.alloc(0x80)for _ in range(3)]
 allowed=c.get('allowed',[[0,1],[2],[0,2]])
 for i,p in enumerate(nations):
  m.w(p+0x40,c.get('slots',4));m.w(p+0x48,m.array([factions[v]for v in allowed[i]])if allowed[i]else 0);m.w(p+0x4c,len(allowed[i]));m.u.mem_write(p+0x68,bytes([c.get('selectable',[0,1,1])[i]]))
 world=m.alloc(0x400);data=m.alloc(0x600);kingdom=m.alloc(0x400)
 m.w(0xb36534,world);m.w(0xb36530,data);m.w(data+0x80,m.array(nations));m.w(data+0x84,c.get('nation_count',3));m.w(world+0xd4,c['seed'])
 a=m.alloc(0x80);s=m.alloc(0x80);m.w(0xb3655c,a);m.w(0xb36560,s)
 def log(mm):
  assert mm.arg(2)==m.r(world+0xd4);events.append(['log', 'A'if mm.ecx()==a else'S',mm.arg(2)]);mm.ret(pop=12)
 m.seam(0x77da70,log)
 def replace(mm):
  new=mm.arg(1);events.append(['commander',reverse.get(new),nations.index(m.r(kingdom+0x234))if m.r(kingdom+0x234)else None])
  m.w(kingdom+0x230,m.array([0,new])if new else 0);mm.ret(pop=4)
 m.seam(0x4e0f00,replace)
 m.w(kingdom+0x234,nations[c['old_nation']]if c.get('old_nation')is not None else 0)
 m.w(kingdom+0x230,m.array([0,factions[c['commander']]])if c.get('commander')is not None else 0)
 linked(m,kingdom+0x36c,c.get('taken',[]));hqs=[]
 for i,index in enumerate(c.get('hqs',[])):
  actor=m.alloc(0x200);component=m.alloc(0x2c);vt=m.alloc(0x120);m.w(actor,vt);m.w(vt+0x108,0x4d1e80);m.w(actor+0xe8,kingdom);m.w(actor+0xd8,component);m.w(component+4,actor);m.w(component+0x10,index);hqs.append((actor,component))
 m.w(kingdom+0x2dc,m.array([p for p,_ in hqs])if hqs else 0);m.w(kingdom+0x2e0,len(hqs))
 if c['kind']=='set':m.call(0x4e15d0,kingdom,[nations[c['new_nation']]if c.get('new_nation')is not None else 0])
 else:
  m.u.reg_write(UC_X86_REG_ESI,kingdom);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_ESP,nm.STACK)
  m.u.emu_start(0x50ec66,0x50ecfa,count=100000)
  assert m.u.reg_read(UC_X86_REG_EIP)==0x50ecfa and m.u.reg_read(UC_X86_REG_ESP)==nm.STACK
 current=m.r(kingdom+0x234);commander=m.r(kingdom+0x230)
 out=dict(nation=nations.index(current)if current else None,commander=reverse.get(m.r(commander+4))if commander else None,seed_after=m.r(world+0xd4),hqs=[m.r(p+0x10)for _,p in hqs],taken=list_values(m,kingdom+0x36c),events=events)
 assert not m.null_accesses
 seams={0x6f7810,0x6f7430,0x77da70,0x4e0f00}
 return out,{p:b for p,b in m.visited.items()if p not in seams}

def cases():
 out=[]
 for old,new,commander in itertools.product([None,0,1],[None,0,1],[None,0,1,2]):
  out.append(dict(kind='set',seed=1,old_nation=old,new_nation=new,commander=commander))
 for slots,taken,hqs,seed in itertools.product([0,1,4],[[],[0,2,0]],[[-1],[2],[-1,0]], [0,0xffffffff]):
  out.append(dict(kind='set',seed=seed,new_nation=1,slots=slots,taken=taken,hqs=hqs))
 for old,commander,enabled,seed in itertools.product([None,0,1,2],[None,0],[[0,0,0],[0,1,0],[1,1,1]], [0,1,0xffffffff]):
  out.append(dict(kind='random',seed=seed,old_nation=old,commander=commander,enabled=enabled))
 out+=[dict(kind='random',seed=1,nation_count=0),dict(kind='random',seed=1,old_nation=0,allowed=[[],[],[]])]
 return out

def main():
 rows=[];pins={}
 for pc,c in itertools.product(nm.PRECISIONS,cases()):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(c,a,b);pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for p,raw in pins.items():assert nm.original.range(p,len(raw))==nm.modified.range(p,len(raw))==raw,hex(p)
 doc=dict(scope=__doc__,rows=rows,native_executions=2*len(rows),seams_not_executed=['6F7810','6F7430','77DA70','4E0F00'],instructions=[dict(va=hex(p),bytes=b.hex())for p,b in sorted(pins.items())])
 path=HERE/'native-configure-nations.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()

