"""Actual563BA0 through native Character vtable activation4DB0E0/4CFC10,
actual45E690/4D0720,49B4D0/85D840,563770/4DB120 and empty42A590/429080.
Explicit raw source/owner storage; queue cancellation/insertion and integer
logger leaves are transport seams. No complete staged factory/world claim.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *

def run(image,pc,c):
 m=h.Machine(pc)
 if image=='v14':
  old=(h.P.image_size+0xfff)&~0xfff;new=(h.V.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(h.P.base+old,new-old)
  for vs,r,raw,off in h.V.sections:
   if raw:m.u.mem_write(h.V.base+r,h.V.b[off:off+raw])
 actor=m.alloc(0x200);char=m.alloc(0x30);data=m.alloc(0x200);view=m.alloc(0x60);arr=m.alloc(4);idle=m.alloc(0x60);old=m.alloc(0x60);lst=m.alloc(4);types=m.alloc(56);world=m.alloc(0x200)
 m.w32(0xb36534,world);m.w32(world+0xd4,c['seed']);m.w32(world+0xe0,c['now'])
 m.w32(char,0xa3b4d8,actor,0xa3b4b0,c['head']);m.w32(char+0x14,c['elapsed'],0,0x3f800000,idle if c['same_animation']else old,0,c['word28'],-2)
 m.w32(actor+4,data);m.w32(actor+8,c['flags']);m.w32(actor+0x40,-1);m.w32(actor+0x68,char);m.w32(actor+0xe4,char if c['active']else 0);m.w32(actor+0x15c,-1)
 m.w32(data+0x1bc,arr,1);m.w32(arr,view);m.w32(view,types);m.w32(view+0xc,lst,1);m.w32(lst,idle)
 m.w32(data+0x12c,c['min'],c['max']);m.w32(idle+8,0,0,c['length']);m.w32(old+8,0,0,c['length'])
 m.w32(0xb3ae88,types)
 for i in range(7):m.w32(types+8*i+4,int(i in [0,2]))
 events=[];seams=set();visited={}
 def event(name,**kw):events.append(dict(name=name,**kw))
 def cancel(x):
  assert x.ecx()==char+8;event('cancel');m.w32(char+0xc,0);m.ret()
 def schedule(x):
  assert x.ecx()==char+8;event('schedule',delay=x.arg(),data=x.arg(1));m.w32(char+0xc,1);m.ret(8)
 def integer(x):
  #Native4F02B0 logs actual current seed twice, A thenS.
  event('seed',value=x.arg(1));m.ret(12)
 m.seams.update({0x50b250:cancel,0x50acc0:schedule,0x77da70:integer})
 def observe(u,a,n,_):
  if a in m.seams:seams.add(a)
  elif h.P.base<=a<h.P.base+h.P.image_size:visited[a]=n
 m.u.hook_add(UC_HOOK_CODE,observe)
 m.call(0x563ba0,char);assert m.final_esp==h.STACK+4 and m.fpu_top()==0
 state=dict(animation=1 if m.r32(char+0x20)==idle else 2,elapsed=m.r32(char+0x14),rate=m.r32(char+0x1c),word28=m.r32(char+0x28),head=bool(m.r32(char+0xc)),active=m.r32(actor+0xe4)==char,rng=m.r32(world+0xd4))
 return dict(events=events,state=state),visited,seams

def main():
 rows=[];pins={};seams=set()
 for pc,active,head,word,same,flags in itertools.product(h.PRECISION,[False,True],[0,7],[0,2],[False,True],[0,1]):
  c=dict(active=active,head=head,word28=word,same_animation=same,flags=flags,seed=0x12345678,now=0x40880000,elapsed=0x3dfcd35b,length=0x3f333333,min=1,max=4)
  a,pa,sa=run('original',pc,c);b,pb,sb=run('v14',pc,c);assert a==b
  pins.update(pa);pins.update(pb);seams|=sa|sb;rows.append(dict(precision=pc,input=c,output=a))
 instructions=[]
 for a,n in sorted(pins.items()):
  if a in seams:continue
  raw=h.P.range(a,n);assert raw==h.V.range(a,n);instructions.append(dict(va=hex(a),bytes=raw.hex()))
 vtable=[]
 for a in [0xa3b4b0+12,0xa3b4d8+0x38,0xa3b4d8+0x3c,0xa3b4d8+0x40,0xa3b4d8+0x44]:
  raw=h.P.range(a,4);assert raw==h.V.range(a,4);vtable.append(dict(va=hex(a),bytes=raw.hex()))
 out=dict(schema=1,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),original_sha256=h.ORIGINAL_SHA,v14_sha256=h.V14_SHA,rows=rows,executions=len(rows)*2,instructions=instructions,vtable=vtable,seams=[hex(a)for a in sorted(seams)])
 raw=(json.dumps(out,indent=1)+'\n').encode();path=HERE/'native-stance-owner.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(instructions),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
