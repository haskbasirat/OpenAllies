"""Original economy eligibility and actor/layout/definition resource vectors.
All gate, vector traversal/addition, definition getter and modifier arithmetic
execute original instructions. Heap allocation/free and pure4CF110 ownership
lookup are the only intercepted services. Native component cache writes execute.
"""
from pathlib import Path
import copy,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-rng'))
from original_rng import Machine,original,modified
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ACTOR,DEF,ORG,ECO,MOD,TRANSPORT,DOMAIN,OUTPUT=0x4000000,0x4100000,0x4200000,0x4300000,0x4400000,0x4500000,0x4600000,0x4610000
ptr=lambda base,n:0 if n is None else base+n*0x1000
identity=lambda base,p:None if p==0 else(p-base)//0x1000
bits=lambda f:struct.unpack('<I',struct.pack('<f',f))[0]
sha=lambda b:hashlib.sha256(b).hexdigest()
class EconomyMachine(Machine):
 def __init__(self,c):
  super().__init__({'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.c=copy.deepcopy(c);self.visited={};self.heap=0x4700000;self.u.mem_map(0,4096);self.u.mem_map(0x4000000,0x800000)
  self.w(0xb3a660,DOMAIN);self.w(DOMAIN+0x28,c['resource_count']);self.w(0xb36530,DOMAIN+0x100);self.w(DOMAIN+0x138,c['resource_count'])
  for key,d in c['definitions'].items():
   p=ptr(DEF,int(key));self.w(p+0x174,d['flags174']);self.w(p+0x178,d['class178']);self.w(p+0x17c,d['word17c']);self.u.mem_write(p+0x348,bytes([d['supply_required']]));self.w(p+0x548,ptr(DEF,d['packed_definition']))
   self.w(p+0x2f0,p+0x600);self.w(p+0x608,p+0x700);self.w(p+0x718,0 if d['layout_kind18'] is None else d['layout_kind18'])
   for offset,name in [(0x324,'cost'),(0x330,'upkeep'),(0x33c,'production')]:self.write_array(p+offset,d[name])
  for key,a in c['actors'].items():
   p=ptr(ACTOR,int(key));self.w(p+4,ptr(DEF,a['definition']));self.w(p+8,a['flags']);self.w(p+0x100,a['state_bits']&0xffffffff);self.w(p+0x104,a['state_bits']>>32)
   self.w(p+0x80,p+0x500);self.w(p+0x510,ptr(ACTOR,a['element_organization']));self.w(p+0x7c,ptr(ORG,a['organization']));self.w(p+0x84,ptr(ECO,a['economy']));self.w(p+0xc0,ptr(TRANSPORT,a['transport']));self.w(p+0x8c,p+0x600 if a['constructor18'] is not None else 0);self.w(p+0x618,a['constructor18'] or 0)
  for key,org in c['organizations'].items():
   p=ptr(ORG,int(key));layout=p+0x600;self.w(p+0x18,layout);self.w(layout+0x28,layout+0x100);self.w(layout+0x2c,len(org['slots']));self.w(p+0x1c,layout+0x200)
   for i,definition in enumerate(org['slots']):self.w(layout+0x100+i*4,i);self.w(layout+0x200+i*4,ptr(DEF,definition))
   self.w(p+0x28,p+0x900);self.w(p+0x2c,len(org['members']))
   for i,member in enumerate(org['members']):self.w(p+0x900+i*4,ptr(ACTOR,member))
  for key,m in c['modifiers'].items():
   p=ptr(MOD,int(key));self.w(p,m['flags']);self.w(p+0x4c,m['factor4c']);self.w(p+0x60,m['factor60']);self.write_array(p+0x80,m['production_adjustments'])
  for key,u in c['transports'].items():self.write_vector(ptr(TRANSPORT,int(key))+8,u['Cost']);self.write_vector(ptr(TRANSPORT,int(key))+0x18,u['Upkeep'])
  for key,values in c['caches'].items():self.write_vector(ptr(ECO,int(key))+0x44,values)
  self.write_vector(OUTPUT,c['initial_output']);self.u.hook_add(UC_HOOK_CODE,self.hook)
 def alloc(self,count):p=self.heap;self.heap+=(max(count,16)+15)&~15;assert self.heap<0x4800000;return p
 def write_array(self,p,values):
  data=self.alloc(max(16,len(values))*4);self.w(p,data);self.w(p+4,len(values));self.w(p+8,max(16,len(values)))
  for i,v in enumerate(values):self.w(data+i*4,v)
 def write_vector(self,p,values):self.w(p,0xa26f44);self.write_array(p+4,values)
 def vector(self,p):return[self.r(self.r(p+4)+i*4)for i in range(self.r(p+8))]
 def ret(self,value=0,pop=0):s=self.u.reg_read(UC_X86_REG_ESP);self.u.reg_write(UC_X86_REG_EAX,value);self.u.reg_write(UC_X86_REG_EIP,self.r(s));self.u.reg_write(UC_X86_REG_ESP,s+4+pop)
 def hook(self,u,pc,size,_):
  if original.base<=pc<original.base+original.image_size:self.visited[pc]=bytes(u.mem_read(pc,size))
  cx=u.reg_read(UC_X86_REG_ECX);s=u.reg_read(UC_X86_REG_ESP)
  if pc==0x6f7810:self.ret(self.alloc(self.r(s+4)))
  elif pc==0x6f7430:self.ret()
  elif pc==0x4cf110:self.ret(ptr(MOD,self.c['actor_modifiers'].get(str(identity(ACTOR,cx)))))
 def run(self):
  self.reset(0);op=self.c['operation'];actor=ptr(ACTOR,self.c['actor']);args=[];this=0
  if op in ['eligible','packed']:
   entry={'eligible':0x56f9a0,'packed':0x56f960}[op];args=[actor]
  elif op=='supplied':entry=0x4d0c90;this=actor
  elif op=='recruitable':entry=0x45e5a0;this=ptr(DEF,self.c['definition'])
  elif op in ['Cost','Upkeep','Production']:
   entry={'Cost':0x56fa30,'Upkeep':0x56fb10,'Production':0x56fc20}[op];args=[OUTPUT,actor]
   if op=='Production':args.append(int(self.c['cache_members']))
  elif op.startswith('definition_'):
   entry={'Cost':0x45f4d0,'Upkeep':0x45f570,'Production':0x45f680}[op[11:]];this=ptr(DEF,self.c['definition']);args=[OUTPUT+4,ptr(MOD,self.c['context'])]
  else:raise ValueError(op)
  self.u.reg_write(UC_X86_REG_ECX,this)
  for i,value in enumerate(args):self.w(self.STACK+4+i*4,value)
  regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  self.u.emu_start(entry,self.STOP,count=200000)
  expected=self.STACK+4+(len(args)*4 if op.startswith('definition_')else 0)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.STOP and self.u.reg_read(UC_X86_REG_ESP)==expected,(self.c['name'],hex(self.u.reg_read(UC_X86_REG_EIP)),hex(self.u.reg_read(UC_X86_REG_ESP)),hex(expected))
  assert[self.u.reg_read(r)for r in regs]==[0x11111111*(i+1)for i in range(4)]
  return {'predicate':bool(self.u.reg_read(UC_X86_REG_EAX)&255)if op in['eligible','packed','supplied','recruitable']else None,'values':self.vector(OUTPUT),'caches':{key:self.vector(ptr(ECO,int(key))+0x44)for key in self.c['caches']}}
def cases():
 d={'flags174':1<<9,'class178':1,'word17c':0,'layout_kind18':0,'supply_required':False,'packed_definition':None,'cost':list(map(bits,[10.1,20.2,30.3])),'upkeep':list(map(bits,[1.1,2.2,3.3])),'production':list(map(bits,[5.5,6.6,7.7]))}
 a={'definition':1,'flags':0,'state_bits':0,'element_organization':None,'organization':None,'economy':1,'constructor18':None,'transport':None}
 base={'name':'base','operation':'eligible','actor':1,'definition':1,'context':1,'resource_count':3,'actors':{str(i):{**copy.deepcopy(a),'economy':i}for i in range(1,5)},'definitions':{'1':d,'2':{**copy.deepcopy(d),'class178':0},'3':{**copy.deepcopy(d),'flags174':0}},'organizations':{'1':{'slots':[1,None,2,1],'members':[2,None,3]}},'modifiers':{'1':{'flags':0,'factor4c':bits(.3333333),'factor60':bits(1.3333333),'production_adjustments':list(map(bits,[.1,.2,.3]))}},'actor_modifiers':{'1':1,'2':None,'3':1,'4':None},'transports':{'1':{'Cost':list(map(bits,[41,42,43])),'Upkeep':list(map(bits,[11,12,13]))}},'caches':{str(i):list(map(bits,[-10-i,-20-i,-30-i]))for i in range(1,5)},'initial_output':list(map(bits,[91,92,93])),'cache_members':False}
 rows=[]
 def add(name,change):c=copy.deepcopy(base);c['name']=name;change(c);rows.append(c)
 for op in ['eligible','packed','supplied']:
  for state in [0,1<<19,1<<21,1<<39]:
   for progress in [None,0,bits(-1),bits(.1),0x7f800000,0x7fc00001]:add(f'{op}_state{state}_progress{progress}',lambda c,o=op,s=state,v=progress:(c.update(operation=o),c['actors']['1'].update(state_bits=s,constructor18=v)))
 for cls in [0,1,2,3]:
  for flag in [0,1<<7]:
   for layout in [-1,0,1]:add(f'recruitable{cls}_{flag}_{layout}',lambda c,cl=cls,f=flag,l=layout:(c.update(operation='recruitable'),c['definitions']['1'].update(class178=cl,flags174=f,layout_kind18=l)))
 for leader in [None,2]:
  for dead in [0,1]:
   for supply in [False,True]:add(f'eligible{leader}_{dead}_{supply}',lambda c,l=leader,d=dead,s=supply:(c['actors']['1'].update(element_organization=l,flags=d,state_bits=1<<19),c['definitions']['1'].update(supply_required=s)))
 add('packed_replacement',lambda c:(c.update(operation='eligible'),c['actors']['1'].update(state_bits=1<<19),c['definitions']['1'].update(packed_definition=2,supply_required=True)))
 for op in ['Cost','Upkeep','Production','definition_Cost','definition_Upkeep','definition_Production']:
  for modifier in [None,1]:
   for organization in [None,1]:add(f'{op}_modifier{modifier}_org{organization}',lambda c,o=op,m=modifier,g=organization:(c.update(operation=o,context=m),c['actor_modifiers'].update({'1':m}),c['actors']['1'].update(organization=g)))
 for op in ['Cost','Upkeep']:
  add(f'{op}_transport',lambda c,o=op:(c.update(operation=o),c['definitions']['1'].update(flags174=1<<24),c['actors']['1'].update(transport=1,flags=1,element_organization=2)))
 for op in ['Cost','Upkeep','Production']:
  add(f'{op}_null',lambda c,o=op:c.update(operation=o,actor=None))
  add(f'{op}_noeconomy',lambda c,o=op:(c.update(operation=o),c['actors']['1'].update(economy=None)))
  add(f'{op}_mixed_layout',lambda c,o=op:(c.update(operation=o),c['actors']['1'].update(organization=1),c['organizations']['1'].update(slots=[3,None,2,1])))
 for flags in [1<<15,(1<<15)|(1<<13),0]:add(f'costflags{flags}',lambda c,f=flags:(c.update(operation='Cost'),c['definitions']['1'].update(flags174=f)))
 for op in ['Upkeep','Production']:
  add(f'{op}_modifier_flags',lambda c,o=op:(c.update(operation=o),c['modifiers']['1'].update(flags=1<<9)))
 for cache in [False,True]:
  add(f'production_nested{cache}',lambda c,k=cache:(c.update(operation='Production',cache_members=k),c['actors']['1'].update(organization=1),c['actors']['2'].update(organization=2),c['organizations'].update({'2':{'slots':[],'members':[4,None,4]}})))
  add(f'production_packed{cache}',lambda c,k=cache:(c.update(operation='Production',cache_members=k),c['actors']['1'].update(organization=1,state_bits=1<<19),c['definitions']['1'].update(packed_definition=2)))
 for length in [0,1,2,4,7]:add(f'production_table_length{length}',lambda c,n=length:(c.update(operation='Production'),c['modifiers']['1'].update(production_adjustments=[bits(.11111)]*n)))
 for count in [0,1,4,5,7]:
  for op in ['Cost','Upkeep','Production']:
   def variable(c,count=count,op=op):
    c.update(operation=op,resource_count=count);c['actors']['1']['organization']=1
    for k,d in c['definitions'].items():
     for j,kind in enumerate(['cost','upkeep','production']):d[kind]=[bits((i-2.11111)*(j+1.77777))for i in range(count)]
    c['modifiers']['1']['production_adjustments']=[bits(-.3333333)]*count
   add(f'bulk_{op}_{count}',variable)
 for op in ['Cost','Upkeep','Production']:
  add(f'mismatched_add_{op}',lambda c,o=op:(c.update(operation=o,resource_count=5),c['actors']['1'].update(organization=1),c['actors']['2'].update(definition=2),c['definitions']['2'].update(cost=[bits(7)],upkeep=[bits(8)],production=[bits(9)])))
 for p in ['Single24','Double53','Extended64']:
  for c in rows:yield {**copy.deepcopy(c),'precision':p,'name':c['name']+'_'+p}
def main():
 rows=[];visited={}
 for c in cases():
  try:m=EconomyMachine(c);result=m.run()
  except Exception:print(c['name']);raise
  rows.append({'input':c,'output':result});visited.update(m.visited)
 for a,b in visited.items():assert b==original.range(a,len(b))==modified.range(a,len(b)),hex(a)
 d={'schema':1,'method':__doc__,'original_sha256':sha(original.b),'v14_sha256':sha(modified.b),'producer_sha256':sha(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(visited.items())],'cases':rows}
 p=HERE/'native-values.json';p.write_text(json.dumps(d,indent=2),encoding='utf-8');print(json.dumps({'cases':len(rows),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
