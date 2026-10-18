"""Isolated original x86 cell-collision/filter/shape executions, not game calls."""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
sha=lambda b:hashlib.sha256(b).hexdigest()
WORLD,GRID,TILES,DESC,REGISTRY,AVARS,ORIENT=0x4000000,0x4001000,0x4010000,0x4020000,0x4400000,0x4030000,0x4031000
VTABLE,CONTEXTVT,FPVALUE=0x4040000,0x4041000,0x4042000
RADIUS,SHAPE,CUSTOM,SOURCE,CATEGORY,ALLEGIANCE,HOSTILE,OUTER=range(0x4700000,0x4700080,0x10)
act=lambda i:0 if i is None else 0x4100000+i*0x1000
definition=lambda i:0 if i is None else 0x4200000+i*0x1000
context=lambda i:0 if i is None else 0x4300000+i*0x1000
node=lambda i:0 if i is None else 0x4050000+i*16
shape=lambda i:0x4060000+i*32
aid=lambda p:None if p==0 else (p-0x4100000)//0x1000

class CollisionMachine(Machine):
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.events=[];self.outer_count=0;self.allegiance_count=0
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']])
  self.u.mem_map(0,0x1000);self.u.mem_map(0x4000000,0x800000)
  self.u.hook_add(UC_HOOK_CODE,self.hook)
  g=c['grid'];self.w(0xb36534,WORLD);self.w(WORLD+0x30,GRID);self.w(0xb364c0,REGISTRY)
  self.w(GRID+4,g['coordinate_shift']);self.w(GRID+8,g['cell_size_bits']);self.w(GRID+0x1c,g['dimensions'][0]);self.w(GRID+0x20,g['dimensions'][1]);self.w(GRID+0x28,TILES)
  self.w(0xb39990,AVARS);self.w(AVARS+0x64,ORIENT);self.w(ORIENT+4,0);self.w(ORIENT+8,bits(-1.))
  self.tile=TILES+48*(c['tile']['coordinates'][1]*g['dimensions'][0]+c['tile']['coordinates'][0])
  self.w(self.tile+8,node(c['tile']['first_node']));self.w(self.tile+0x20,c['tile']['flags20'])
  ref=c['tile']['actor_reference'];self.w(self.tile+0x1c,0x10000+ref if ref else 0)
  for i,a in enumerate(c['actors'],1):
   ap=act(i);self.w(ap,VTABLE);self.w(ap+4,definition(a['definition']));self.w(ap+8,a['flags'])
   for off,value in zip([0x20,0x24,0x2c,0x30],a['position']+a['facing']):self.w(ap+off,value)
   self.u.mem_write(shape(i),struct.pack('<6I',a['shape']['kind'],a['shape']['radius_bits'],*a['shape']['bounds']))
   self.u.mem_write(REGISTRY+4+i*2,struct.pack('<H',1));self.w(REGISTRY+0x20004+i*4,ap)
  for i,d in enumerate(c['definitions'],1):
   self.w(definition(i)+0x1c,d['flags']);self.u.mem_write(definition(i)+0x1a8,bytes([d['flag_1a8']]))
  for i,n in enumerate(c['nodes'],1):self.w(node(i),act(n['actor']));self.w(node(i)+4,node(n['next']))
  for off,ptr in [(4,RADIUS),(12,SHAPE),(0x30,SOURCE),(0xb4,CATEGORY),(0x108,ALLEGIANCE),(0x100,OUTER)]:self.w(VTABLE+off,ptr)
  for i in [1,2]:self.w(context(i),CONTEXTVT)
  self.w(CONTEXTVT+0x140,HOSTILE)
  self.u.mem_write(RADIUS,b'\xd9\x05'+struct.pack('<I',FPVALUE)+b'\xc3')
  self.write_descriptor()
 def write_descriptor(self):
  d=self.c['descriptor']
  if d is None:return
  for off,key,convert in [(0x18,'source_definition',definition),(0x1c,'source_actor',act),(0x20,'ignore_actor',act),(0x24,'ignore_definition',definition),(0x28,'collision_owner',act),(0x2c,'hostility_context',context)]:self.w(DESC+off,convert(d[key]))
  self.u.mem_write(DESC+0x30,bytes([d['ignore_all'],d['ignore_moving']]))
  self.w(DESC+0x34,CUSTOM if d['custom_filter'] else 0);self.w(DESC+0x3c,d['mask'])
 def ret(self,value,pop=0):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value)
  self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4+pop)
 def mutate(self,event,actor):
  change=self.c.get('mutation',{})
  if change.get('event')!=event:return
  if 'descriptor' in change:self.c['descriptor'].update(change['descriptor']);self.write_descriptor()
  if 'position' in change:
   self.c['actors'][actor-1]['position']=change['position']
   for i,v in enumerate(change['position']):self.w(act(actor)+0x20+4*i,v)
  if 'next' in change:self.c['nodes'][0]['next']=change['next'];self.w(node(1)+4,node(change['next']))
 def hook(self,u,pc,size,user):
  sp=u.reg_read(UC_X86_REG_ESP);actor=aid(u.reg_read(UC_X86_REG_ECX))
  if pc==RADIUS:
   self.events.append(['radius',actor]);self.w(FPVALUE,self.c['actors'][actor-1]['blocking_radius_bits']);self.mutate('radius',actor)
  elif pc==SHAPE:
   self.events.append(['shape',actor]);self.mutate('shape',actor);self.ret(shape(actor))
  elif pc==CUSTOM:
   actor=aid(self.r(sp+8));self.events.append(['custom',actor]);self.mutate('custom',actor);self.ret(int(self.c['custom_result']))
  elif pc==SOURCE:
   target=aid(self.r(sp+4));self.events.append(['source',actor,target]);self.mutate('source',target);self.ret(int(self.c['source_result']),4)
  elif pc==CATEGORY:
   self.events.append(['category',actor]);self.mutate('category',actor);self.ret(int(self.c['category_result']))
  elif pc==ALLEGIANCE:
   self.events.append(['allegiance',actor]);values=self.c['allegiance_results'];value=values[min(self.allegiance_count,len(values)-1)];self.allegiance_count+=1;self.mutate('allegiance',actor);self.ret(context(value))
  elif pc==HOSTILE:
   receiver=(u.reg_read(UC_X86_REG_ECX)-0x4300000)//0x1000;other=(self.r(sp+4)-0x4300000)//0x1000 if self.r(sp+4) else None
   self.events.append(['hostile',receiver,other]);self.mutate('hostile',1);self.ret(int(self.c['hostile_result']),4)
  elif pc==OUTER:
   self.events.append(['outer',actor]);values=self.c['outer_results'];value=values[min(self.outer_count,len(values)-1)];self.outer_count+=1;self.mutate('outer',actor);self.ret(act(value))
  elif pc==0x4cede0:
   self.events.append(['moving',actor]);self.mutate('moving',actor);self.ret(int(self.c['moving_result']))
 def run(self):
  self.reset(0);self.u.reg_write(UC_X86_REG_ECX,DESC if self.c['operation']=='excluded' else self.tile)
  self.w(self.STACK+4,act(1) if self.c['operation']=='excluded' else (DESC if self.c['descriptor'] is not None else 0))
  self.u.emu_start({'blocked':0x54f370,'find':0x54f190,'excluded':0x552340}[self.c['operation']],self.STOP,count=30000)
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+8
  value=self.u.reg_read(UC_X86_REG_EAX)
  return {'result':aid(value) if self.c['operation']=='find' else bool(value&255),'events':self.events,'descriptor':self.c['descriptor']}

def main():
 d={'source_definition':None,'source_actor':2,'ignore_actor':None,'ignore_definition':None,'collision_owner':None,'hostility_context':None,'ignore_all':False,'ignore_moving':False,'custom_filter':False,'mask':1024}
 a={'definition':1,'flags':0x20000400,'position':[bits(7.),bits(7.)],'facing':[0,bits(-1.)],'blocking_radius_bits':bits(1.),'shape':{'kind':1,'radius_bits':bits(1.),'bounds':[0]*4}}
 base={'name':'base','operation':'find','grid':{'dimensions':[8,8],'coordinate_shift':1,'cell_size_bits':bits(2.)},'tile':{'coordinates':[3,3],'flags20':1024,'actor_reference':2,'first_node':1},'actors':[a,{**a,'definition':2}],'definitions':[{'flags':0x20000400,'flag_1a8':False}]*2,'nodes':[{'actor':1,'next':2},{'actor':2,'next':None}],'descriptor':d,'custom_result':False,'source_result':False,'category_result':True,'allegiance_results':[1,2],'hostile_result':True,'moving_result':False,'outer_results':[2,2]}
 specs=[]
 def add(name,**kwargs):specs.append({**copy.deepcopy(base),'name':name,**kwargs})
 add('base');add('without_descriptor',descriptor=None)
 for operation in ['find','blocked','excluded']:
  for key,value in [('ignore_all',True),('ignore_actor',1),('ignore_definition',1),('source_actor',None),('custom_filter',True),('collision_owner',2),('hostility_context',1),('ignore_moving',True)]:add(operation+'_'+key,operation=operation,descriptor={**d,key:value})
 for operation in ['find','blocked']:
  for flags in [0,1,256,512,1024,1025]:add(operation+'_flags_'+str(flags),operation=operation,tile={**base['tile'],'flags20':flags})
 for key in ['custom_result','source_result','category_result','hostile_result','moving_result']:
  add(key,operation='excluded',descriptor={**d,'custom_filter':True,'hostility_context':1,'ignore_moving':True},actors=[{**a,'flags':1024},a],**{key:not base[key]})
 for flags in [0,1024,0x20000000,0x20000400]:add('actor_flags_'+str(flags),actors=[{**a,'flags':flags},a])
 for radius in [0.,.49,.5,.51,1.]:add('eligibility_'+str(radius),actors=[{**a,'blocking_radius_bits':bits(radius)},a])
 for x in [7.,8.,8.4999,8.5,8.5001,20.]:add('circle_distance_'+str(x),actors=[{**a,'position':[bits(x),bits(7.)]},a])
 for radius in [0.,.49,.5,1.,2.]:add('shape_radius_'+str(radius),actors=[{**a,'shape':{**a['shape'],'radius_bits':bits(radius)}},a])
 for kind in [0,2,3]:
  for facing in [[0.,-1.],[1.,0.],[.6,.8]]:
   for xy in [[7.,7.],[8.5,7.],[9.,7.],[8.5,8.5],[8.4999,8.],[7.,9.5]]:
    shape_data={'kind':kind,'radius_bits':bits(2.),'bounds':[bits(-1.),bits(-2.),bits(1.),bits(2.)]}
    add(f'shape_{kind}_{facing}_{xy}',actors=[{**a,'facing':list(map(bits,facing)),'position':list(map(bits,xy)),'shape':shape_data},a])
 for size,shift in [(.75,0),(1.75,0),(3.75,2),(2.00000024,1)]:add('cell_size_'+str(size),grid={**base['grid'],'cell_size_bits':bits(size),'coordinate_shift':shift})
 for values in [[None],[2,None],[1,2],[2,1]]:add('outer_'+str(values),descriptor={**d,'collision_owner':2},outer_results=values)
 for values in [[None],[1,None],[1,2]]:add('allegiance_'+str(values),operation='excluded',descriptor={**d,'hostility_context':1},actors=[{**a,'flags':1024},a],allegiance_results=values)
 add('definition_1a8',operation='excluded',descriptor={**d,'source_definition':2},definitions=[{'flags':0x20000400,'flag_1a8':True},{'flags':0,'flag_1a8':False}])
 add('shape_moves_actor',mutation={'event':'shape','position':[bits(99.),bits(99.)]})
 add('custom_sets_ignore',descriptor={**d,'custom_filter':True},mutation={'event':'custom','descriptor':{'ignore_definition':1}})
 add('radius_removes_next',actors=[{**a,'blocking_radius_bits':bits(.1)},a],mutation={'event':'radius','next':None})
 add('outer_clears_owner',descriptor={**d,'collision_owner':2},outer_results=[2,None],mutation={'event':'outer','descriptor':{'collision_owner':None}})
 rows=[]
 for p in ['Single24','Double53','Extended64']:
  for spec in specs:
   c={**spec,'precision':p};rows.append({'input':c,'output':CollisionMachine(c).run()})
 ranges={'tile':(0x54f190,0x54f3b7),'eligible':(0x4ee700,0x4ee762),'filter':(0x552340,0x55243a),'placement_overlap':(0x46ef30,0x46f105),'shape_overlap':(0x46ed10,0x46ee00),'circle_box':(0x46e400,0x46e500)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a)
 report={'schema':1,'source_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'method':'Original tile gate, scan, filter, geometry execute. Virtual actor operations, custom filter, and 4CEDE0 organization/mind state are explicit callbacks. Only those leaves are hooked. Callback trace, returned actor/boolean and mutable descriptor are recorded.','ranges':{n:{'start':hex(a),'end_exclusive':hex(b),'sha256':sha(original.range(a,b-a))}for n,(a,b)in ranges.items()},'cases':rows}
 (HERE/'native-collision-fixtures.json').write_text(json.dumps(report,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha((HERE/'native-collision-fixtures.json').read_bytes())}))
if __name__=='__main__':main()
