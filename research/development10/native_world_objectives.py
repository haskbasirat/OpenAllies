"""Actual MapString502EB0 constructor,502FC0 field loader and4F1AD0 lookup.
The World constructor250..25C interval also executes. CString allocator and
version/String/localized-pair input transports are named inherited seams;
this does not claim full49AD70 fallback, archive IO or full World constructor.
Case-sensitive wcscmp external CRT transport has equality semantics only.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parent
sys.path.insert(0,str(ROOT/'skirmish-20261017-mission-start'))
from native_creator_constructor import Machine,h
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_EIP,UC_X86_REG_ECX
class Stop(Exception):pass

def run(c,image,pc):
 m=Machine(image,pc,dict(width=192,height=192,independent=0,resource_count=3));events=[]
 def guard(name):
  events.append(name)
  if c.get('stop')==name:raise Stop(name)
 p=m.alloc(0x14);m.u.mem_write(p,b'\xcd'*0x14)
 m.call(0x502eb0,[],this=p,callee_pop=0)
 def row(at):return dict(key=m.rs(at+4),display_key=m.rs(at+8),display_text=m.rs(at+12),flag=m.rb(at+16))
 if c['kind']=='load':
  resource=m.alloc(0x60);table=m.alloc(4)
  def read():
   guard('flag');assert m.arg(1)==1;m.b(m.arg(0),c['flag']);m.ret(8,1)
  read_stub=m.alloc(16);m.u.mem_write(read_stub,b'\xcc'*16);m.seams[read_stub]=read
  m.w(table,read_stub);m.w(resource+0x3c,table)
  def version():guard('version');m.w(m.u.reg_read(UC_X86_REG_ECX),0);m.ret(8,0)
  def key():guard('key');m.cs(m.arg(1),c['key']);m.ret(0,0)
  def pair():
   guard('pair');m.cs(m.arg(1),c['display_key']);m.cs(m.arg(1)+4,c['display_text']);m.ret(0,0)
  m.seams[0x8589b0]=version;m.seams[0x41d290]=key;m.seams[0x49ad70]=pair
  stopped=None
  try:m.call(0x502fc0,[resource],this=p,callee_pop=4)
  except Stop as e:stopped=str(e)
  out=dict(row=row(p),events=events,stopped=stopped)
 elif c['kind']=='find':
  world=m.alloc(0x300);array=m.alloc(max(1,len(c['keys']))*4);nodes=[]
  for i,key in enumerate(c['keys']):
   n=m.alloc(0x14);m.cs(n+4,key);nodes.append(n);m.w(array+4*i,n)
  m.w(world+0x250,array);m.w(world+0x254,len(nodes));query=m.alloc(4);m.cs(query,c['query'])
  def compare():
   a=m.text(m.arg(0)).split('\0')[0];b=m.text(m.arg(1)).split('\0')[0]
   events.append([a,b]);m.ret(0,0 if a==b else 1)
  m.seams[0x96cb83]=compare
  value=m.call(0x4f1ad0,[query],this=world,callee_pop=4)
  out=dict(index=None if value==0 else nodes.index(value),events=events)
 else:
  world=m.alloc(0x300);m.u.mem_write(world,b'\xcd'*0x300)
  m.u.reg_write(UC_X86_REG_ESI,world);m.u.reg_write(UC_X86_REG_EBX,0)
  m.u.emu_start(0x4f7fe2,0x4f800c,count=1000)
  assert m.u.reg_read(UC_X86_REG_EIP)==0x4f800c
  out=dict(row=row(p),world_count=m.r(world+0x254),world_capacity=m.r(world+0x258),word_25c=m.r(world+0x25c))
 return out,m.visited,m.intercepted

def main():
 cases=[dict(kind='constructor')]
 for key,dk,dt,flag in [('objective_1','caption','Text',0),('Objective_1','','',255),('','caption','Text',1)]:
  for stop in ['none','version','key','pair','flag']:
   cases.append(dict(kind='load',key=key,display_key=dk,display_text=dt,flag=flag,stop=stop))
 for keys in [[],['one'],['one','one','One'],['a','b','c']]:
  for query in ['one','One','b','absent','']:
   cases.append(dict(kind='find',keys=keys,query=query))
 rows=[];visited={};seams=set()
 for pc in h.PRECISION:
  for c in cases:
   a,pa,sa=run(c,'original',pc);b,pb,sb=run(c,'v14',pc);assert a==b
   visited.update(pa);visited.update(pb);seams|=sa|sb;rows.append(dict(precision=pc,input=c,output=a))
 pins=[]
 for va,n in sorted(visited.items()):
  if va in seams or not 0x401000<=va<0x9ef000:continue
  raw=h.P.range(va,n);assert raw==h.V.range(va,n);pins.append(dict(va=hex(va),bytes=raw.hex()))
 doc=dict(schema=1,scope=__doc__,original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,
  producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,executions=len(rows)*2,
  instructions=pins,seams=[hex(a)for a in sorted(seams)])
 path=HERE/'native-world-objectives.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
