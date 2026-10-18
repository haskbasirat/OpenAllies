"""Unchanged5638B0/4CFD90 plus real45E5E0/45E600/45E690/4D0720.
Explicit source arrays and canonical animation-type bytes are controlled input.
Property apply/remove/admission, animation play and diagnostics are required
child services; injected child errors retain actual parent stores. No initialized
actor/source-loader/full Animation transition is claimed by this parent probe.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
class Required(Exception):pass
def row(**kwargs):
 c=dict(mode='change',sets=[1,2,3],aspects=[0,1,2],properties=[1,None,2],normal=0,variant=0,
 current=-1,next=0,has_properties=True,applicable=True,kind=0,pending=-2,fail=None)
 c.update(kwargs);return c
def cases():
 out=[]
 for mode in('change','apply'):
  for current,next,props,app,kind in itertools.product((-1,0,1),(-1,0,1,2),(False,True),(False,True),(0,1)):
   if mode=='apply'and kind==1:continue
   out.append(row(mode=mode,current=current,next=next,has_properties=props,applicable=app,kind=kind))
 out.extend([row(sets=[1,None,3],current=0,next=1),row(sets=[1,1,3],current=0,next=1),
  row(sets=[None,None,None],current=-1,next=2),row(variant=-1,next=2),
  row(mode='apply',next=3),row(mode='apply',current=5,next=0),
  row(mode='apply',next=3,fail='diagnostic'),row(mode='apply',current=5,next=0,fail='diagnostic'),
  row(mode='apply',current=0,next=2,fail='remove'),row(mode='apply',current=-1,next=2,fail='applicable'),
  row(mode='apply',current=0,next=2,fail='add'),row(current=0,next=1,fail='play'),row(current=0,next=1,kind=2,fail='play')])
 return out
def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 actor=m.alloc(0x180);data=m.alloc(0x200);character=m.alloc(0x40);properties=m.alloc(0x20);animation=m.alloc(0x20)
 m.w(actor+4,data);m.w(actor+0x40,c['variant']);m.w(actor+0x15c,c['current']);m.w(actor+0x78,properties if c['has_properties']else 0)
 m.w(character+4,actor);m.w(character+0x20,animation);m.w(character+0x2c,c['pending']);m.w(animation+8,c['kind'])
 kinds=m.alloc(7*8);m.w(0xb3ae88,kinds)
 for i in range(7):m.wb(kinds+8*i+4,int(i in(0,2)))
 symbols={v:m.alloc(4)for v in set(c['sets'])if v is not None}
 m.w(data+0x1bc,m.array([symbols.get(v,0)for v in c['sets']]));m.w(data+0x1c0,len(c['sets']));m.w(data+0x118,c['normal'])
 property_symbols={v:m.alloc(4)for v in c['properties']if v is not None}
 stances=[]
 for aspect,property in zip(c['aspects'],c['properties']):
  p=m.alloc(0x18);a=m.alloc(0x10);m.w(a+8,aspect);m.w(p+8,a);m.w(p+12,property_symbols.get(property,0));stances.append(p)
 m.w(data+0x188,m.array(stances));m.w(data+0x18c,len(stances))
 events=[]
 def reached(label,event):
  events.append(event)
  if c['fail']==label:raise Required(label)
 def remove(mm):
  assert mm.ecx()==properties and mm.arg(2)==actor
  property=next(k for k,v in property_symbols.items()if v==mm.arg(1));reached('remove',['remove',property]);mm.ret(pop=8)
 def applicable(mm):
  assert mm.arg(1)==actor
  property=next(k for k,v in property_symbols.items()if v==mm.arg(2));reached('applicable',['applicable',property]);mm.ret(int(c['applicable']))
 def add(mm):
  assert mm.ecx()==properties and mm.arg(2)==9 and mm.arg(3)==actor
  property=next(k for k,v in property_symbols.items()if v==mm.arg(1));reached('add',['add',property,9]);mm.ret(pop=12)
 def play(mm):
  assert mm.ecx()==character and mm.arg(1)==animation and mm.arg(2)==0
  reached('play',['play',0]);mm.ret(pop=8)
 def diagnostic(mm):
  caller=mm.r(mm.u.reg_read(nm.UC_X86_REG_ESP));assert caller in(0x4cfdda,0x4cfe2c)
  reached('diagnostic',['diagnostic',caller==0x4cfe2c]);mm.ret()
 for addr,fn in((0x581810,remove),(0x5035e0,applicable),(0x5817a0,add),(0x563770,play),(0x404ed0,diagnostic)):m.seam(addr,fn)
 outcome='ok';result=None
 try:result=bool(m.call(0x5638b0,character,[c['next']])&255)if c['mode']=='change'else bool(m.call(0x4cfd90,actor,[c['next'],0])&255)
 except Required as e:outcome=str(e)
 return dict(outcome=outcome,result=result,current=m.r(actor+0x15c),pending=m.r(character+0x2c),events=events),m.visited
def main():
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases():
   pair=[]
   for image in('original','v14'):
    output,visited=run(c,image,pc);pair.append(output);pins.update(visited)
   assert pair[0]==pair[1];rows.append(dict(precision=pc,input=c,output=pair[0]))
 for a,b in pins.items():assert b==nm.original.range(a,len(b))==nm.modified.range(a,len(b))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'harness_sha256':hashlib.sha256((ROOT/'research/phase8-ai/native_machine.py').read_bytes()).hexdigest()}
 path=HERE/'native-character-view.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1))
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
