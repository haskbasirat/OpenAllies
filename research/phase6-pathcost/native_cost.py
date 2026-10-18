"""Execute original53CB70; outer-actor virtual and setting binding are explicit seams."""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
WORLD,GRID,STRATEGY,DESC,CURRENT=0x4000000,0x4001000,0x4002000,0x4003000,0x4003100
TILES,REGISTRY=0x4100000,0x4400000
ACTOR_A,ACTOR_B,VTABLE,BODY,OWNER,VAR_A,VAR_B,DEF_A,DEF_B=range(0x4004000,0x400d000,0x1000)
SVARS,WALL_FACTOR,PENALTY,OUTPUT,OUTER_CALL=0x4020000,0x4021000,0x4021004,0x4022000,0x4770000
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
sha=lambda b:hashlib.sha256(b).hexdigest()

class CostMachine(Machine):
 def __init__(self,c):
  self.c=copy.deepcopy(c);super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']])
  self.u.mem_map(0,0x1000);self.u.mem_map(0x4000000,0x800000)
  self.events=[];self.u.hook_add(UC_HOOK_CODE,self.hook)
  self.w(0xb36534,WORLD);self.w(WORLD+0x30,GRID);self.w(0xb364c0,REGISTRY)
  self.w(GRID+8,c['cell_size_bits']);self.w(GRID+0x1c,8);self.w(GRID+0x20,8);self.w(GRID+0x28,TILES)
  self.w(STRATEGY+4,DESC);self.w(STRATEGY+0x30,c['maximum_speed_bits']);self.w(DESC+0x28,OWNER if c['collision_owner'] else 0)
  self.w(ACTOR_A,VTABLE);self.w(VTABLE+0x100,OUTER_CALL)
  self.w(ACTOR_A+0x90,1 if c['has_wall'] else 0);self.w(ACTOR_A+0x60,BODY if c['has_body'] else 0)
  self.w(BODY+0x10,c['health_bits']);self.w(BODY+0x14,c['maximum_health_bits'])
  self.register(0x10001,ACTOR_A);self.register(0x10002,ACTOR_B)
  for cell,var,definition,speed,actor in [(c['from'],VAR_A,DEF_A,c['source_speed_bits'],c['source_actor']),
                                         (c['to'],VAR_B,DEF_B,c['destination_speed_bits'],c['destination_actor'])]:
   tile=self.tile(cell);self.w(tile,var);self.w(var+0x1c,definition);self.w(definition+0x38,speed)
   self.w(tile+0x1c,{0:0,1:0x10001,2:0x10002}[actor])
  self.w(0xb39ab0,SVARS);self.w(SVARS+0x58,WALL_FACTOR);self.w(WALL_FACTOR,c['wall_factor_bits'])
  self.w(0xb3d444,1);self.w(0xb3d440,PENALTY);self.w(PENALTY,c['penalty_bits'])
  self.u.mem_write(CURRENT,struct.pack('<5I',*c['from'],0,0,c['cost_bits']))
 def tile(self,cell):return TILES+(cell[1]*8+cell[0])*0x30
 def cell(self,tile):
  n=(tile-TILES)//0x30;return[n%8,n//8]
 def register(self,id,actor):
  self.u.mem_write(REGISTRY+4+(id&0xffff)*2,struct.pack('<H',id>>16));self.w(REGISTRY+0x20004+(id&0xffff)*4,actor)
 def ret(self,value):
  sp=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value)
  self.u.reg_write(UC_X86_REG_EIP,self.r(sp));self.u.reg_write(UC_X86_REG_ESP,sp+4)
 def hook(self,u,pc,size,user):
  ecx=u.reg_read(UC_X86_REG_ECX)
  if pc==0x45bc70:self.events.append(['tile_actor',*self.cell(ecx)])
  if pc==OUTER_CALL:
   self.events.append(['outer_actor',1])
   changes=self.c.get('mutation',{})
   if 'collision_owner' in changes:self.w(DESC+0x28,OWNER if changes['collision_owner'] else 0)
   if 'maximum_speed_bits' in changes:self.w(STRATEGY+0x30,changes['maximum_speed_bits'])
   if 'cost_bits' in changes:self.w(CURRENT+0x10,changes['cost_bits'])
   if 'health_bits' in changes:self.w(BODY+0x10,changes['health_bits'])
   self.ret(OWNER if self.c['outer_owner'] else 0);return
  if pc==0x53cc48:self.events.append(['wall_factor',self.r(WALL_FACTOR)])
  if pc==0x4d0230:self.events.append(['body_health',1])
  if pc==0x53ccc2:self.events.append(['prepare_terrain'])
  if pc==0x54ece0:self.events.append(['terrain_speed',*self.cell(ecx)])
  if pc==0x53cd68:self.events.append(['terrain_penalty',self.r(PENALTY)])
 def run(self):
  self.reset(0)
  self.u.mem_write(self.STOP,b'\xd9\x1d'+struct.pack('<I',OUTPUT))
  for i,value in enumerate([CURRENT,*self.c['to']]):self.w(self.STACK+4+i*4,value)
  self.u.reg_write(UC_X86_REG_ECX,STRATEGY)
  self.u.emu_start(0x53cb70,self.STOP+6,count=5000)
  assert self.u.reg_read(UC_X86_REG_ESP)==self.STACK+16
  return{'result_bits':self.r(OUTPUT),'events':self.events,
         'state':{'cost_bits':self.r(CURRENT+0x10),'maximum_speed_bits':self.r(STRATEGY+0x30),'collision_owner':bool(self.r(DESC+0x28))}}

def main():
 ranges={'cost':(0x53cb70,0x53cd99),'base_cost':(0x53c5c0,0x53c621),'body_fraction':(0x4d0230,0x4d03d0),
         'tile_actor':(0x45bc70,0x45bca0),'terrain_definition':(0x54ece0,0x54ecf0)}
 for a,b in ranges.values():assert original.range(a,b-a)==modified.range(a,b-a)
 base={'name':'base','from':[3,3],'to':[4,3],'cell_size_bits':bits(2.),'cost_bits':bits(15.25),
       'maximum_speed_bits':bits(6.),'collision_owner':True,'source_actor':0,'destination_actor':1,
       'has_wall':True,'has_body':True,'health_bits':bits(37.),'maximum_health_bits':bits(113.),
       'outer_owner':True,'wall_factor_bits':bits(20.),'source_speed_bits':bits(8.),
       'destination_speed_bits':bits(2.),'penalty_bits':bits(.71)}
 specs=[base]
 for key,values in [('maximum_speed_bits',[0.,-1.,1.,6.,20.]),('health_bits',[-1.,0.,1.,50.,113.,300.]),
                   ('source_speed_bits',[-1.,0.,1.,2.,6.,8.]),('destination_speed_bits',[-1.,0.,1.,2.,6.,8.]),
                   ('wall_factor_bits',[-1.,0.,.5,1.,20.]),('cost_bits',[0.,1e-7,16777216.,-2.]),
                   ('penalty_bits',[-1.,0.,.71,2.])]:
  for value in values:specs.append({**base,'name':f'{key}_{value}',key:bits(value)})
 for key in ['collision_owner','outer_owner','has_wall','has_body']:
  specs.append({**base,'name':key+'_false',key:False})
 for a,b in [(0,0),(0,2),(1,1),(2,1)]:specs.append({**base,'name':f'actors_{a}_{b}','source_actor':a,'destination_actor':b})
 for direction,(x,y) in enumerate([[-1,-1],[-1,0],[-1,1],[0,1],[1,1],[1,0],[1,-1],[0,-1]],1):
  specs.append({**base,'name':f'direction_{direction}','to':[3+x,3+y]})
 for index,change in enumerate([{'collision_owner':False},{'maximum_speed_bits':0},{'maximum_speed_bits':bits(12.)},
                                {'cost_bits':bits(200.)},{'health_bits':bits(2.)}]):
  specs.append({**base,'name':f'outer_mutation_{index}','mutation':change})
 specs.append({**base,'name':'outer_returns_null_and_clears_owner','outer_owner':False,'mutation':{'collision_owner':False}})
 rows=[]
 for p in ['Single24','Double53','Extended64']:
  for spec in specs:
   c={**spec,'precision':p};rows.append({'input':c,'output':CostMachine(c).run()})
 report={'schema':1,'source_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),
         'method':'Unchanged53CB70/53C5C0 and leaf tile/Body/terrain access. Only virtual outer-actor callback is supplied; setting cache pointers are initialized to explicit authored input values.',
         'ranges':{name:{'start':hex(a),'end_exclusive':hex(b),'sha256':sha(original.range(a,b-a))}for name,(a,b)in ranges.items()},'cases':rows}
 (HERE/'native-cost-fixtures.json').write_text(json.dumps(report,indent=2),encoding='utf-8')
 print(json.dumps({'cases':len(rows),'sha256':sha((HERE/'native-cost-fixtures.json').read_bytes())}))
if __name__=='__main__':main()
