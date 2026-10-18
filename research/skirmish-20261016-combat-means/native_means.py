"""Seam-bounded native 4BBF90 producer; original/v14 and three explicit PCs.

Owner list/source reads, ordered getter calls, all shared mean stores/reads,
assertion state and diagnostic logs are compared. Native allocation/free and
assert/log callbacks are declared leaves. Getter mode is either original native
code over supplied objects or explicit dynamic callback responses. No completed
startup, active user profile, or actual source-load FP context is supplied.
"""
from pathlib import Path
import copy, hashlib, json, random, struct, sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
import native_machine as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE,UC_MEM_READ,UC_MEM_WRITE
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
f32=h.f32
FIELDS=list(range(0x74,0x94,4))
GETTERS={'extent':(0x45e6d0,0),'power':(0x4be3f0,4),'animation':(0x4be680,0),'recharge':(0x4be4b0,4),'defense':(0x45d760,4),'movement':(0x45d6f0,4),'health':(0x45d730,4)}
LEAVES={0x6f7810,0x6f7430,0x855d30,0x404f60}

class Machine(h.Machine):
 def __init__(self,pc,image,case):
  super().__init__(pc);self.case=case;self.trace=[];self.stopped=None;self.calls={};self.defs={};self.locals={};self.abilities={};self.nodes={};self.arrays={};self.watch={}
  old=(h.original.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
  if new>old:self.u.mem_map(image.base+old,new-old)
  for _,rva,size,off in image.sections:self.u.mem_write(image.base+rva,image.b[off:off+size])
  # Required actual Settings pointer table, with an explicit diagnostic input.
  table=self.alloc(0x200);cell=self.alloc(4);self.w(cell,case['discard']);self.w(table+0x15c,cell)
  self.w(0xb39ab0,table)
  self.manager=self.alloc(0x100);self.holder=self.alloc(0x10);self.w(self.manager+0x68,self.holder)
  for off,bits in zip(FIELDS,case['initial_means']):self.w(self.manager+off,bits)
  self.wb(0xb3b4f8,case['guard'])
  for i,t in enumerate(case['things']):
   d=self.defs[i]=self.alloc(0x500);node=self.nodes[i]=self.alloc(0x10);self.w(node,d)
   for off,key in [(0x174,'flags'),(0x288,'defense'),(0x228,'movement'),(0x284,'health'),(0x478,'extent')]:self.w(d+off,t[key])
   self.w(d+0x468,0);arr=self.arrays[i]=self.alloc(4*max(1,len(t['abilities'])));self.w(d+0x1ec,arr);self.w(d+0x1f0,t.get('ability_count',len(t['abilities'])))
   self.watch[node]=('definition',i);self.watch[node+8]=('next',i);self.watch[d+0x174]=('flags',i);self.watch[d+0x1f0]=('count',i)
   for j,a in enumerate(t['abilities']):
    local_id=1000+i*16+j;ability_id=2000+i*16+j
    local=self.locals[local_id]=self.alloc(0x60);ad=self.abilities[ability_id]=self.alloc(0x80)
    self.w(local,ad);self.w(ad+0x64,a['power']);self.w(local+4,a['reload']);self.w(local+0x18,a['recharge']);self.w(arr+4*j,local)
    if a['animation'] is not None:
     p,q,data,elem=self.alloc(4),self.alloc(8),self.alloc(4),self.alloc(0x20)
     self.w(local+0x3c,p);self.w(local+0x40,1);self.w(p,q);self.w(q,data);self.w(q+4,1);self.w(data,elem);self.w(elem+0x10,a['animation'])
    self.watch[arr+4*j]=('ability_at',i,j);self.watch[local]=('ability_definition',local_id);self.watch[ad+0x64]=('base_power',ability_id);self.watch[local+4]=('reload',local_id)
  for i in self.nodes:self.w(self.nodes[i]+8,self.nodes.get(i+1,0))
  self.w(self.holder+8,self.nodes.get(0,0));self.watch[self.holder+8]=('first',)
  self.reverse={v:k for m in [self.defs,self.locals,self.abilities,self.nodes] for k,v in m.items()}
  self.seam(0x6f7810,lambda m:m.ret(m.alloc(max(m.arg(1),4))))
  self.seam(0x6f7430,lambda m:m.ret())
  self.seam(0x855d30,self.assertion);self.seam(0x404f60,self.log)
  for name,(addr,pop) in GETTERS.items():
   callback=lambda m,n=name,p=pop:m.getter(n,p)
   if case['mode']=='dynamic':self.seam(addr,callback)
   else:self.observe(addr,callback)
  self.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,self.memory)
  if case.get('stop')=='empty':self.observe(0x4bc332,lambda m:m.stop('empty'))
 def stop(self,why):self.stopped=why;self.u.emu_stop()
 def event(self,event,key=None):
  self.trace.append(event)
  if key is None:return None
  nth=self.calls[key]=self.calls.get(key,0)+1
  action=next((a for a in self.case['actions'] if a['key']==key and a['nth']==nth),{})
  for field,value in action.get('set',{}).items():
   kind,raw=field.split(':');ident=int(raw)
   if kind=='mean':self.w(self.manager+ident,value)
   elif kind=='flags':self.w(self.defs[ident]+0x174,value)
   elif kind=='reload':self.w(self.locals[ident]+4,value)
   elif kind=='count':self.w(self.defs[ident]+0x1f0,value)
   elif kind=='next':self.w(self.nodes[ident]+8,self.nodes.get(value,0))
   elif kind=='ability_ref':self.w(self.locals[ident],self.abilities[value])
   elif kind=='slot':self.w(self.arrays[(ident-1000)//16]+4*((ident-1000)%16),self.locals[value])
   else:raise ValueError(kind)
  if action.get('error'):self.stop(action['error'])
  return action.get('return_f64')
 def getter(self,name,pop):
  ident=self.reverse[self.ecx()];response=self.event(['get',name,ident],f'get:{name}:{ident}')
  if self.stopped:return
  if self.case['mode']=='native':assert response is None;return
  if response is not None:self.ret_st0_f64(response,pop);return
  if name in ['extent','defense','movement','health']:value=self.r(self.ecx()+{'extent':0x478,'defense':0x288,'movement':0x228,'health':0x284}[name])
  elif name=='power':value=self.r(self.ecx()+0x64)
  elif name=='recharge':value=self.r(self.ecx()+0x18)
  elif name=='animation':
   i=(ident-1000)//16;j=(ident-1000)%16;value=self.case['things'][i]['abilities'][j]['animation'] or 0
  else:raise ValueError(name)
  self.ret_st0_f32(value,pop)
 def assertion(self,m):
  assert m.arg(3)==0x24b;m.event(['assert'],'assert')
  if not m.stopped:m.ret(m.case['assert_result'])
 def log(self,m):
  line=m.arg(3);off=[0x78,0x7c,0x80,0x84,0x88,0x90,0x8c][line-0x253]
  value=struct.unpack('<Q',m.u.mem_read(m.u.reg_read(UC_X86_REG_ESP)+24,8))[0]
  m.event(['log',off,line,f'{value:016x}'],f'log:{off}')
  if not m.stopped:m.ret()
 def memory(self,u,access,address,size,value,_):
  pc=u.reg_read(UC_X86_REG_EIP)
  if not 0x4bbf90<=pc<0x4bc5d4:return
  if self.manager+0x74<=address<self.manager+0x94:
   assert size==4;self.event(['read' if access==UC_MEM_READ else 'write',address-self.manager,self.r(address) if access==UC_MEM_READ else value&0xffffffff]);return
  if address==0xb3b4f8:
   self.event(['guard',self.rb(address)] if access==UC_MEM_READ else ['guard_write',value]);return
  if access!=UC_MEM_READ:return
  if pc==0x4bc2e8:self.event(['discard',self.r(address)]);return
  data=self.watch.get(address)
  if data is None:return
  op,*args=data
  if op in ['first','next','definition','ability_at','ability_definition']:v=self.reverse.get(self.r(address),None)
  else:v=self.r(address)
  if op=='count' and v>=0x80000000:v-=0x100000000
  self.event([op,*args,v],f'base_power:{args[0]}' if op=='base_power' else None)

def ability(power=10,reload=1,recharge=3,animation=1):return {'power':f32(power),'reload':f32(reload),'recharge':f32(recharge),'animation':None if animation is None else f32(animation)}
def thing(flags=0x29,power=10):return {'flags':flags,'extent':f32(2),'defense':f32(1),'movement':f32(4),'health':f32(120),'abilities':[ability(power)]}
def case(name,things=None,mode='native',actions=None,discard=.2,guard=0):return {'name':name,'things':things if things is not None else [thing(),thing(power=20),thing(power=30)],'mode':mode,'actions':actions or [],'discard':f32(discard),'guard':int(guard),'assert_result':0xa5,'initial_means':[f32(v) for v in range(101,109)]}
def specs(pc):
 out=[];rng=random.Random(0x4bbf90)
 for i in range(14):
  ts=[]
  for j in range(3+i%9):
   t=thing(rng.choice([0x29,0x4029,0x20129,0x20029,0x4000,9,0x21]));t.update(extent=f32(rng.uniform(0,20)),defense=f32(rng.uniform(0,4)),movement=f32(rng.uniform(0,12)),health=f32(rng.uniform(1,600)))
   t['abilities']=[ability(rng.choice([0,rng.uniform(.5,60)]),rng.uniform(0,3),rng.uniform(.1,10),rng.uniform(0,4)) for _ in range(rng.randrange(4))];ts.append(t)
  # Ensure positive attack and a nonzero finite terminal divisor.
  ts.append(thing(power=100));out.append(case(f'finite-{i}',ts,discard=[0,.05,.1,.2,.25][i%5],guard=i%3==0))
 f64=lambda f:struct.unpack('<Q',struct.pack('<d',f))[0]
 out += [case('extent-second-and-flags',[thing(0x4029),thing(power=20),thing(power=30)],'dynamic',[{'key':'get:extent:0','nth':1,'return_f64':f64(4.0000002),'set':{'flags:0':0x29,'mean:116':f32(1)}},{'key':'get:extent:0','nth':2,'return_f64':f64(7.0000003)}]),
  case('extent-disables-combat',[thing(0x4029),thing(power=20),thing(power=30)],'dynamic',[{'key':'get:extent:0','nth':1,'set':{'flags:0':0}}]),
  case('reload-captured-before-animation',mode='dynamic',actions=[{'key':'get:animation:1000','nth':1,'set':{'reload:1000':f32(90)},'return_f64':f64(2.0000001)}]),
  case('recharge-two-independent-calls',mode='dynamic',actions=[{'key':'get:recharge:1000','nth':1,'return_f64':f64(2)},{'key':'get:recharge:1000','nth':2,'return_f64':f64(7.0000002)}]),
  case('ability-count-captured',things=[dict(thing(),abilities=[ability(),ability(17)]),thing(power=20),thing(power=30)],mode='dynamic',actions=[{'key':'get:power:2000','nth':1,'set':{'count:0':0,'slot:1001':1016}}]),
  case('power-definition-captured',mode='dynamic',actions=[{'key':'base_power:2000','nth':1,'set':{'ability_ref:1000':2016}}]),
  case('next-reread-after-getter',mode='dynamic',actions=[{'key':'get:health:0','nth':1,'set':{'next:0':2}}]),
  case('assert-callback-mutates-shared',things=[dict(thing(power=10),defense=f32(40)),dict(thing(power=20),defense=f32(40)),dict(thing(power=30),defense=f32(40))],mode='dynamic',actions=[{'key':'assert','nth':1,'set':{'mean:120':f32(50),'mean:128':f32(3),'mean:136':f32(777)}}]),
  case('logs-reread-shared',mode='dynamic',actions=[{'key':'log:120','nth':1,'set':{'mean:124':f32(77),'mean:128':f32(88)}}]),
  case('negative-ability-count',things=[dict(thing(),ability_count=-1),thing(power=20),thing(power=30)])]
 for name,key in [('failed-extent','get:extent:0'),('failed-power','get:power:2000'),('failed-animation','get:animation:1000'),('failed-second-recharge','get:recharge:1000'),('failed-defense','get:defense:1'),('failed-movement','get:movement:1'),('failed-health','get:health:1'),('failed-assert','assert'),('failed-fourth-log','log:132')]:
  ts=[thing(0x4029),thing(power=20),thing(power=30)]
  if key=='assert':
   for t in ts:t['defense']=f32(80)
  c=case(name,ts,'dynamic',[{'key':key,'nth':2 if name=='failed-second-recharge' else 1,'error':name}]);c['stop']=name;out.append(c)
 empty=case('empty-attack-range',[dict(thing(),abilities=[])]);empty['stop']='empty';out.append(empty)
 # A real registered-input SUBSET in native manager order, not full-manager means.
 actual_path=ROOT/'research/skirmish-20261016-combat-value-source/actual-source-inputs.json';actual=json.loads(actual_path.read_text())
 rows={r['id']:r for r in actual['rows'] if r['precision']==pc};ordered=[rows[i] for i in actual['actor_manager_order'] if i in rows];ts=[]
 for r in ordered:
  t=thing(r['flags_174']);t.update(defense=(r['body'] or [0,0])[0],health=(r['body'] or [0,0])[1],movement=r['speed_228'] or 0,abilities=[{'power':a['power_64'],'reload':a['value_04'],'recharge':a['recharge_18'],'animation':a['first_entry_10']} for a in r['abilities']]);ts.append(t)
 real=case('registered-source-subset',ts);real['source_subset']={'fixture_sha256':sha(actual_path.read_bytes()),'definitions':[{'id':r['id'],'source':r['source']} for r in ordered]};out.append(real)
 return out
def main():
 rows=[];pins={}
 for image_name,image in [('original',h.original),('v14',h.modified)]:
  for pc in h.PRECISIONS:
   for spec in specs(pc):
    m=Machine(pc,image,spec)
    try:m.call(0x4bbf90,ecx=m.manager)
    except AssertionError:
     if m.stopped is None:raise
    assert m.stopped==spec.get('stop'),(spec['name'],m.stopped,spec.get('stop'))
    assert not m.null_accesses;m.verify_v14();pins.update(m.visited)
    rows.append({'image':image_name,'precision':pc,'input':spec,'trace':m.trace,'means':[m.r(m.manager+o) for o in FIELDS],'guard':m.rb(0xb3b4f8),'stopped':m.stopped})
 out=h.header(__file__,__doc__);out.update(passed=True,cases=rows,instructions=h.instructions(pins),seam_entries=[hex(a) for a in sorted(LEAVES|{v[0] for v in GETTERS.values()})],limitations=['Getter entry bytes are pins only in native getter mode; dynamic mode declares them response seams. Native allocator/free/assert/log entries are always excluded.','Empty range stops immediately before native reciprocal division, not a claimed safe native recovery.','Source subset is ten registered definitions previously bound from source, in filtered native-kind-2 list order; it is not all 1028 definitions or completed startup.','No native source-load PC/profile context, preceding completion-pass receipt, ActorData+1E4, 4BB590, full game or battle claim.'])
 (HERE/'native-means.json').write_text(json.dumps(out,indent=2)+'\n',encoding='utf-8');print('passed',len(rows),'cases',len(pins),'pins')
if __name__=='__main__':main()
