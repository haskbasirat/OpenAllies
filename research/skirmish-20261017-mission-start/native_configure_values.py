"""Native Configure first-pass scalar/string/vector prefix and Team/Color leaves.

The tail executes50EB06..50EBB5. CString/storage transport are explicit seams;
positive4E6640 is a required entry before the later merge/resource stores.
Team4E3A50 executes real4F1430 removal, including the old/new counter increments.
Color4F0D00 executes without seams. ColorPool50E370 uses actual4F02B0/85D840,
provided nonempty initialized color registry/storage and retained raw RNG state.
These are field/leaf proofs, not source materialization or full ConfigureGWorld.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P
class Required(Exception):pass
def machine(image,pc):h.P=ORIGINAL if image=='original'else h.V;return h.Machine(pc)
def vector(m,p,values,capacity=None):
 a=m.alloc(max(1,len(values),capacity or 0)*4)
 for i,v in enumerate(values):m.w32(a+4*i,v)
 m.w32(p,a,len(values),max(len(values),capacity or 0));return a
def allocation(m,p,n):
 old=m.r32(p);count=m.r32(p+4);new=m.alloc(max(1,n)*4)
 if old and count:m.u.mem_write(new,bytes(m.u.mem_read(old,count*4)))
 m.w32(p,new);m.w32(p+8,n)
def tail(image,pc,flags,positive,values):
 m=machine(image,pc);seams=set();events=[]
 def seam(a,f):m.seams[a]=f;seams.add(a)
 def cstring(_):m.w32(m.ecx(),m.r32(m.arg()));m.ret(4)
 def grow(_):allocation(m,m.ecx(),m.arg());m.ret(4)
 def required(_):events.append('required_4E6640');raise Required()
 seam(0x405b80,cstring);seam(0x4849f0,grow);seam(0x4e6640,required)
 kingdom=m.alloc(0x400);parameter=m.alloc(0x6c)
 m.w32(parameter+0x24,0x76543210);m.w32(parameter+0x2c,0x12340000);m.w32(parameter+0x58,0x56780000)
 m.w8(parameter+0x28,flags);m.w8(parameter+0x32,255-flags)
 m.w32(kingdom+0x238,0xaabbccdd,0x11223344);m.w32(kingdom+0x240,0x22220000,0x33330000,0x44440000)
 vector(m,kingdom+0x258,[0x3f800000,0x40000000],2)
 vector(m,parameter+0x34,values);vector(m,parameter+0x4c,[0x2000400]if positive else[])
 m.u.reg_write(UC_X86_REG_ESI,kingdom);m.u.reg_write(UC_X86_REG_EDI,parameter);m.u.reg_write(UC_X86_REG_ESP,h.STACK)
 outcome='ok'
 try:m.u.emu_start(0x50eb06,0x50ebb5,count=10000);assert m.u.reg_read(UC_X86_REG_EIP)==0x50ebb5
 except Required:outcome='required_4E6640'
 n=m.r32(kingdom+0x25c);a=m.r32(kingdom+0x258)
 return {'outcome':outcome,'events':events,'word238':m.r32(kingdom+0x238),'word23c':m.r32(kingdom+0x23c),
  'string240':m.r32(kingdom+0x240),'reference244':m.r32(kingdom+0x244),'string248':m.r32(kingdom+0x248),
  'resources':[m.r32(a+4*i)for i in range(n)]},seams
def teams(image,pc,old,new,members):
 m=machine(image,pc);seams={0x6f7810,0x6f7430};m.seams[0x6f7810]=lambda _:m.ret(value=m.alloc(m.arg()));m.seams[0x6f7430]=lambda _:m.ret()
 k=m.alloc(0x400);p=[0,m.alloc(0x80),m.alloc(0x80)];others={1:k,2:m.alloc(0x400),3:m.alloc(0x400)}
 for i in [1,2]:
  last=0
  for v in reversed(members[i-1]):node=m.alloc(8);m.w32(node,others[v],last);last=node
  m.w32(p[i]+0x44,last);tail=last
  while tail and m.r32(tail+4):tail=m.r32(tail+4)
  m.w32(p[i]+0x48,tail);m.w32(p[i]+0x68,0xffffffff if i==1 else 7)
 m.w32(k+0x1ec,p[old]);m.call(0x4e3a50,k,(p[new],));output=[]
 for i in [1,2]:
  a=m.r32(p[i]+0x44);items=[]
  while a:items.append(next(i for i,p in others.items()if p==m.r32(a)));a=m.r32(a+4)
  output.append({'members':items,'counter':m.r32(p[i]+0x68)})
 return {'team':new,'teams':output},seams
def color(image,pc,old,new):
 m=machine(image,pc);world=m.alloc(0x300);a=m.alloc(12);m.w32(world+0x160,a);m.w32(a,0,0xffffffff,10)
 p=[0]+[m.alloc(16)for _ in range(3)]
 for i in range(1,4):m.w32(p[i]+12,i-1)
 m.call(0x4f0d00,world,(p[old],p[new]));return [m.r32(a+4*i)for i in range(3)]
def pool(image,pc,seed,remaining):
 m=machine(image,pc);data=m.alloc(0x500);world=m.alloc(0x300);m.w32(0xb36530,data);m.w32(0xb36534,world)
 events=[];m.w32(0xb3655c,m.alloc(0x80));m.w32(0xb36560,m.alloc(0x80))
 def log(_):assert m.arg(1)==seed;events.append(seed);m.ret(12)
 m.seams[0x77da70]=log
 colors=m.alloc(16);m.w32(data+0x3f0,colors,4)
 for i in range(4):m.w32(colors+4*i,0x11220000+i)
 v=m.alloc(12);vector(m,v,remaining,4)
 rng=world+0xd4;m.w32(rng,seed)
 m.call(0x50e370,v);return {'chosen':m.u.reg_read(UC_X86_REG_EAX)-0x11220000,'seed_after':m.r32(rng),'remaining':[m.r32(m.r32(v)+4*i)for i in range(m.r32(v+4))],'seed_logs':events}
def properties(image,pc,initial,value):
 m=machine(image,pc);k=m.alloc(0x400);vector(m,k+0x24c,initial,8)
 m.call(0x4e6640,k,(value,));return [m.r32(m.r32(k+0x24c)+4*i)for i in range(m.r32(k+0x250))]
def main():
 tails=[];teamrows=[];colorrows=[];pools=[];propertyrows=[];seams={0x77da70}
 for image,pc in itertools.product(('original','v14'),h.PRECISION):
  for flags,positive,values in itertools.product((0,1,128,255),(False,True),([], [0x7fc12345,0x80000000,0x3f800001])):
   out,s=tail(image,pc,flags,positive,values);seams|=s;tails.append({'image':image,'precision':pc,'input':{'flags':flags,'positive':positive,'resources':values},'output':out})
  for old,new,members in itertools.product(range(3),range(3),([[],[]],[[2,1,1,3],[1,2]],[[2,3],[]])):
   out,s=teams(image,pc,old,new,members);seams|=s;teamrows.append({'image':image,'precision':pc,'input':{'old':old,'new':new,'members':members},'output':out})
  for old,new in itertools.product(range(4),range(4)):colorrows.append({'image':image,'precision':pc,'input':[old,new],'output':color(image,pc,old,new)})
  for seed,remaining in itertools.product((0,1,0xffffffff),([0,1,2,3],[2],[3,1])):pools.append({'image':image,'precision':pc,'input':{'seed':seed,'remaining':remaining},'output':pool(image,pc,seed,remaining)})
  for initial,value in itertools.product(([],[0,2],[1,2,1]),(0,1,2,3)):propertyrows.append({'image':image,'precision':pc,'input':{'initial':initial,'value':value},'output':properties(image,pc,initial,value)})
 doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'seams_not_executed':[hex(a)for a in sorted(seams)],'executed_pins':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams],
  'tails':tails,'teams':teamrows,'colors':colorrows,'pools':pools,'properties':propertyrows}
 p=HERE/'native-configure-values.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'tails':len(tails),'teams':len(teamrows),'colors':len(colorrows),'pools':len(pools),'properties':len(propertyrows),'pins':len(doc['executed_pins']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
