"""Execute original finite range helpers and actor-wrapper capture ordering."""
from pathlib import Path
import json,random,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified,sha
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
def bits(v):return struct.unpack('<I',struct.pack('<f',v))[0]
def shape(kind,radius,bounds):return dict(kind=kind,radius_bits=bits(radius),bounds=list(map(bits,bounds)))
SHAPES=[shape(0,0,[0]*4),shape(1,1.25,[-1.25,-1.25,1.25,1.25]),shape(2,2**.5,[-1,-1,1,1]),shape(3,5**.5,[-2,-1,2,1])]
def placement(s,facing,position):return dict(shape=s,facing=list(map(bits,facing)),position=list(map(bits,position)))
class RangeMachine(Machine):
 def put(self,a,*values):self.u.mem_write(a,struct.pack('<'+'I'*len(values),*[v&0xffffffff for v in values]))
 def shape(self,address,shape):self.put(address,shape['kind'],shape['radius_bits'],*shape['bounds'])
 def descriptor(self,address,shape_address,value):
  self.shape(shape_address,value['shape']);self.put(address,shape_address,*value['facing'],*value['position'],0)
 def run(self,c):
  self.reset(0);a=self.STATE+0x100;bs=self.STATE+0x200
  self.descriptor(a,self.STATE+0x1000,c['a']);self.descriptor(bs,self.STATE+0x1100,c['b'])
  op=c['operation'];arguments=[c['range_bits'],bs,int(c['inverted'])]
  entry=0x46f100
  if op=='shape_point':entry=0x46f230;arguments=[c['range_bits'],*c['b']['position'],int(c['inverted'])]
  if op=='point_delta':entry=0x46e290;arguments=[c['range_bits'],*c['b']['position'],int(c['inverted'])]
  self.put(self.STACK,self.STOP,*arguments);self.u.reg_write(UC_X86_REG_ECX,a)
  self.u.emu_start(entry,self.STOP,count=10000)
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4+len(arguments)*4
  return bool(self.u.reg_read(UC_X86_REG_EAX)&255)
 def wrapper(self,c):
  self.reset(0);actor_a=self.STATE+0x2000;actor_b=self.STATE+0x2100;vtable=self.STATE+0x2200;getter=self.STATE+0x2300
  self.put(vtable+0x14,getter)
  for id,actor,p in [(1,actor_a,c['a']),(2,actor_b,c['b'])]:
   self.put(actor,vtable);self.put(actor+0x20,*p['position'],0,*p['facing']);self.shape(self.STATE+0x3000+id*0x100,p['shape'])
  events=[];positions={1:list(c['a']['position']),2:list(c['b']['position'])};shapes={1:c['a']['shape'],2:c['b']['shape']}
  def hook(u,address,size,_):
   actor=u.reg_read(UC_X86_REG_ECX);id=1 if actor==actor_a else 2
   if address==0x4ee400:
    assert actor in [actor_a,actor_b]
    values=list(struct.unpack('<5I',u.mem_read(actor+0x20,20)))
    events.append(dict(kind='snapshot',actor=id,position=values[:2],facing=values[3:]))
   if address==getter:
    events.append(dict(kind='getter',actor=id))
    for change in c['mutations']:
     if change['on_actor']==id:
      target=actor_a if change['actor']==1 else actor_b
      if 'position' in change:self.put(target+0x20,*change['position'])
      if 'radius_bits' in change:self.put(self.STATE+0x3000+change['actor']*0x100+4,change['radius_bits'])
    sp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EAX,self.STATE+0x3000+id*0x100);u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+4)
  handle=self.u.hook_add(UC_HOOK_CODE,hook)
  args=[c['range_bits'],actor_b,int(c['inverted'])];entry=0x4ee800
  if c['operation']=='actor_point':args=[c['range_bits'],*c['b']['position'],int(c['inverted'])];entry=0x4ee7c0
  self.put(self.STACK,self.STOP,*args);self.u.reg_write(UC_X86_REG_ECX,actor_a)
  self.u.emu_start(entry,self.STOP,count=10000);self.u.hook_del(handle)
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+4+len(args)*4
  return dict(result=bool(self.u.reg_read(UC_X86_REG_EAX)&255),events=events)
def main():
 positions=[[0,0],[-3,0],[3,0],[0,-2],[0,2],[3,2],[-3,-2],[.00001,-2],[.75,1.25],[1,0],[1.25,0],[2.5,0]]
 faces=[([0,-1],[0,-1]),([1,0],[0,-1]),([.6,-.8],[-.8,-.6])]
 cases=[];wrappers=[];rng=random.Random(46_100)
 for precision,cw in [('Single24',0x7f),('Double53',0x27f),('Extended64',0x37f)]:
  machine=RangeMachine(cw)
  for first in SHAPES:
   for second in SHAPES:
    for fa,fb in faces:
     for point in positions:
      for radius in [-.25,0,1]:
       for inverted in [False,True]:
        c=dict(operation='shapes',precision=precision,a=placement(first,fa,[0,0]),b=placement(second,fb,point),range_bits=bits(radius),inverted=inverted)
        c['result']=machine.run(c);cases.append(c)
   for fa,_ in faces:
    for point in positions:
     for radius in [-.25,0,1]:
      for inverted in [False,True]:
       c=dict(operation='shape_point',precision=precision,a=placement(first,fa,[0,0]),b=placement(SHAPES[0],[0,-1],point),range_bits=bits(radius),inverted=inverted)
       c['result']=machine.run(c);cases.append(c)
  for point in positions:
   for radius in [-.25,0,1]:
    for inverted in [False,True]:
     c=dict(operation='point_delta',precision=precision,a=placement(SHAPES[0],[0,-1],[0,0]),b=placement(SHAPES[0],[0,-1],point),range_bits=bits(radius),inverted=inverted)
     c['result']=machine.run(c);cases.append(c)
  for i in range(180):
   a=placement(rng.choice(SHAPES),rng.choice(faces)[0],[rng.uniform(-7,7),rng.uniform(-7,7)])
   b=placement(rng.choice(SHAPES),rng.choice(faces)[1],[rng.uniform(-7,7),rng.uniform(-7,7)])
   c=dict(operation='shapes',precision=precision,a=a,b=b,range_bits=bits(rng.uniform(-.5,4)),inverted=bool(i%2))
   c['result']=machine.run(c);cases.append(c)
  #Exact precision witnesses: PC24 loses2^-24 at1;PC53 loses2^-55,
  #whereasPC64 retains it. Equality belongs to both normal/inverted results.
  for small in [2**-24,2**-55]:
   for inverted in [False,True]:
    c=dict(operation='shapes',precision=precision,a=placement(shape(1,1,[-1,-1,1,1]),[0,-1],[0,0]),
           b=placement(shape(1,small,[-small,-small,small,small]),[0,-1],[1,0]),range_bits=0,inverted=inverted)
    c['result']=machine.run(c);cases.append(c)
  #Same-kind unequal radius exercises actual operand swapping, not only type.
  for first,second in [(shape(1,.25,[-.25,-.25,.25,.25]),SHAPES[1]),
                       (SHAPES[3],shape(3,10,[-4,-2,4,2])),
                       (shape(2,20,[-10,-10,10,10]),SHAPES[3])]:
   for swap in [False,True]:
    for point in positions:
     for inverted in [False,True]:
      a=placement(first,[.6,-.8],[-1.234567,-2.345678]);b=placement(second,[0,-1],point)
      if swap:a,b=b,a
      c=dict(operation='shapes',precision=precision,a=a,b=b,range_bits=bits(.75),inverted=inverted)
      c['result']=machine.run(c);cases.append(c)
  mutations=[[],[dict(on_actor=2,actor=1,position=list(map(bits,[20,0])))],
   [dict(on_actor=2,actor=2,position=list(map(bits,[20,0])))],
   [dict(on_actor=1,actor=2,radius_bits=bits(20))],
   [dict(on_actor=1,actor=1,position=list(map(bits,[20,0])))]]
  for operation in ['actors','actor_point']:
   for changes in mutations:
    for inverted in [False,True]:
     c=dict(operation=operation,precision=precision,a=placement(SHAPES[1],[0,-1],[0,0]),b=placement(SHAPES[1],[0,-1],[5,0]),range_bits=bits(1),inverted=inverted,mutations=changes)
     c.update(machine.wrapper(c));wrappers.append(c)
 getters=[];machine=RangeMachine()
 for flag in [0,1,2,255]:
  machine.reset(0);definition=machine.STATE+0x1000;machine.put(machine.STATE+4,definition);machine.put(definition+0x80,flag)
  machine.u.emu_start(0x4ea030,machine.STOP,count=32)
  offset=machine.u.reg_read(UC_X86_REG_EAX)-definition
  assert offset==(0x68 if flag else 0x50)
  getters.append(dict(target_override=flag,definition_offset=offset))
 ranges={}
 for name,start,end in [('shapes',0x46f100,0x46f222),('shape_point',0x46f230,0x46f27d),('local_range',0x46eb70,0x46ec64),('rectangle_range',0x46e2d0,0x46e3fa),('point_delta',0x46e290,0x46e2cd),('actor_capture',0x4ee400,0x4ee453),('actor_point',0x4ee7c0,0x4ee7f6),('actors',0x4ee800,0x4ee839),('ordinary_target_getter',0x4ea030,0x4ea045)]:
  data=original.range(start,end-start);assert data==modified.range(start,end-start)
  ranges[name]=dict(start=hex(start),end=hex(end),sha256=sha(data),hex=data.hex())
 report=dict(schema=1,original_sha256=sha(original.b),ranges=ranges,cases=cases,wrappers=wrappers,ordinary_target_getter=getters,
  seams=['Helpers execute unchanged native instructions; shapes and point global use supplied finite fixture state.','Actor wrappers execute actual4EE400 field snapshot and46F100/46F230. Only virtual+14 getter is a declared mutable fixture leaf.','PE zero-initialized global point atB3A8C4 is kind0/radius0 in these bounded point tests.','Explicit x87 PC24/53/64 round-to-nearest; no live process FPU or complete gameplay parity claim.'])
 path=HERE/'native-range.json';path.write_text(json.dumps(report,separators=(',',':'))+'\n')
 print(json.dumps(dict(cases=len(cases),wrappers=len(wrappers),sha256=sha(path.read_bytes()))))
if __name__=='__main__':main()
