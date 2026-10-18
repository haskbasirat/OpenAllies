"""Actual Base4E5DF0 vs Counted4E6C20 constructor fields and ordered Load.
Fresh Base success prefix executes4E6061..608C including native table add.
Counted ctor executes4E6CA9..6D06; Load4E79F0/4E7920 runs unchanged with
declared stream guard/definition resolution/byte-read transports. Actual hash
query4E3FF0/4E2E00 and all three types' vf4/vf8 execute unchanged. No live
saved-file namespace or whole4E6C20 stream import is claimed.
"""
from pathlib import Path
import sys,json,hashlib
sys.path.insert(0,str(Path(__file__).resolve().parent));import native_eligibility as N
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EAX,UC_X86_REG_ECX,UC_X86_REG_ESI,UC_X86_REG_EDI
HERE=Path(__file__).resolve().parent
class Boundary(Exception):pass
def run(c,image,pc):
 N.L.IMAGE=N.ORIGINAL if image=='original'else N.EXPANDED;m=N.L.Loader(N.PC[pc]);k=m.alloc(0x400);table=k+0x1fc;definition=m.alloc(0x600);buckets=m.alloc(20)
 m.w(table+8,buckets);m.w(table+0xc,5);m.u.mem_write(table+0x14,b'\1');events=[];item=None
 start,end=(0x4e6061,0x4e608c)if c['kind']=='base'else(0x4e6ca9,0x4e6d06)
 def ctor(u,at,size,_):
  nonlocal item
  if at==start:u.reg_write(UC_X86_REG_ESI,definition);u.reg_write(UC_X86_REG_EDI,k if c['kind']=='base'else 0)
  if at==end:item=u.reg_read(UC_X86_REG_EAX);raise Boundary()
 hook=m.u.hook_add(UC_HOOK_CODE,ctor)
 try:m.call(start)
 except Boundary:pass
 m.u.hook_del(hook);assert item
 #Base prefix ends after add, so EAX may be unspecified; recover same node.
 if c['kind']=='base':item=m.r(m.r(buckets+4*(definition%5))+4)
 ctor_fields=dict(definition=1 if m.r(item+4)==definition else m.r(item+4),enabled=m.u.mem_read(item+8,1)[0],count=m.r(item+0xc)if c['kind']=='counted'else None)
 owner=m.alloc(0x80);vt=m.alloc(8);m.w(owner+0x3c,vt);m.w(vt,0xe000600)
 def event(kind):
  if c.get('stop')==len(events):raise Boundary()
  events.append(kind)
 def resolve():event('definition');m.w(m.arg(1),definition);m.ret(0)
 def read():
  n=m.arg(1);event('enabled'if n==1 else'count');m.u.mem_write(m.arg(0),bytes([c['enabled']])if n==1 else(c['count']&0xffffffff).to_bytes(4,'little'));m.ret(8)
 m.seam(0x8589b0,lambda:m.ret(8));m.seam(0x4c2e60,resolve);m.seam(0xe000600,read)
 stopped=False
 try:m.call(0x4e7920 if c['kind']=='base'else 0x4e79f0,item,[owner])
 except Boundary:stopped=True
 result=None
 if not stopped:
  if c['kind']=='counted':
   node=m.alloc(12);m.w(node,definition);m.w(node+4,item);m.w(buckets+4*(definition%5),node)
  result=m.call(0x4e3ff0 if c['restore']else 0x4e2e00,table,[definition,c['wanted']]if c['restore']else[definition,c['wanted'],0])&255
 return dict(constructor=ctor_fields,definition=1 if m.r(item+4)==definition else m.r(item+4),enabled=m.u.mem_read(item+8,1)[0],count=m.r(item+0xc)if c['kind']=='counted'else None,events=events,stopped=stopped,result=result),m.windows()
def cases():
 for kind in('base','counted'):
  for enabled in(0,1,128,255):
   for wanted in(-1,0,1,2):
    for restore in(False,True):
     for count in([0]if kind=='base'else[0,1,2,0x80000000,0x7fffffff]):yield dict(kind=kind,enabled=enabled,wanted=wanted,restore=restore,count=count)
  for stop in range(2 if kind=='base'else 3):yield dict(kind=kind,enabled=128,wanted=1,restore=True,count=0x80000000,stop=stop)
def main():
 rows=[];pins={}
 for c in cases():
  for pc in N.PC:
   a,p1=run(c,'original',pc);b,p2=run(c,'v14',pc);assert a==b,(c,pc,a,b)
   for p in p1+p2:pins[p['address']]=p['sha256']
   rows.append(dict(input=c,pc=pc,output=a))
 result=dict(scope=__doc__,rows=rows,basic_block_windows=[dict(address=a,sha256=h)for a,h in sorted(pins.items())]);path=HERE/'native-item-fields.json'
 if '--write'in sys.argv:path.write_text(json.dumps(result,indent=2)+'\n')
 else:assert json.loads(path.read_text())==result
 print('PASS',len(rows),'rows',len(rows)*2,'separate original-v14/allPC executions',len(pins),'basic-block byte windows',hashlib.sha256(path.read_bytes()).hexdigest())
if __name__=='__main__':main()

