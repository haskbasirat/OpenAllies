"""Complete original7B4830 fresh AI startup; only diagnostic and allocation seams."""
from pathlib import Path
import json,sys
from state_oracle import Machine as Base,ROOT,OUT,H,original,modified
from controller_oracle import list_values,base_goal
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
VT={0xa9e560:'Kingdom',0xa9cac8:'LandRegion',0xa9cb58:'WaterRegion',0xa9cd80:'RegionAction',0xa9e950:'Ability',0xa9e9d8:'Recruitment',0xa9eb10:'UnitPrimary',0xa9eb98:'UnitSecondary'}
class Machine(Base):
 def setup(self,c):
  world=self.alloc(0x1000);self.write(0xb36534,world);self.write(world+0x30,self.grid)
  self.call(0x7aa9b0,self.graph,[self.stream-0x3c,self.grid])
  for i in range(self.read(self.graph+0x18)):self.call(0x7bca60,self.read(self.read(self.graph+0x14)+4*i))
  self.registry=self.alloc(0x70000);self.write(0xb364c0,self.registry)
  self.kingdoms=[]
  for k in c['kingdoms']:
   if k is None:self.kingdoms.append(0);continue
   p=self.alloc(0x300);self.write(p+0x14,k['native_id']);self.write(p+0x18c,k['major_index']);self.write(p+0x190,k['category']);self.kingdoms.append(p)
   index=k['native_id']&65535;self.u.mem_write(self.registry+4+2*index,(k['native_id']>>16).to_bytes(2,'little'));self.write(self.registry+0x20004+4*index,p)
  arr=self.alloc(4*len(self.kingdoms));self.write(world+0x154,arr);self.write(world+0x158,len(self.kingdoms))
  for i,p in enumerate(self.kingdoms):self.write(arr+4*i,p)
  self.player=self.alloc(0x200);self.write(self.player,c['player_index']);kp=self.kingdoms[c['own_slot']];self.write(self.player+8,kp)
  profile=self.alloc(0x1000);self.write(self.player+0x18,profile);self.write(self.player+0x1c,profile);self.write(profile+0x3f4,c['neighbor_depth'])
  for i,n in enumerate(c['ability_counts']):self.write(profile+[0x464,0x528,0x5ec][i],n)
  self.option_keys={}
  if c.get('recruitment')is not None:
   opts=c['recruitment'];nation=self.alloc(0x40);oa=self.alloc(4*len(opts['options']));self.write(kp+0x230,nation);self.write(nation+0x10,oa);self.write(nation+0x14,len(opts['options']))
   types=self.alloc(4*len(opts['enabled_types']));self.write(profile+0x884,types)
   for i,value in enumerate(opts['enabled_types']):self.write(types+4*i,value)
   for i,option in enumerate(opts['options']):
    if option is not None:
     p=self.alloc(0x20);self.option_keys[p]=option['object'];self.write(oa+4*i,p)
     if option['definition_type']is not None:
      d=self.alloc(0x20);self.write(p+4,d);self.write(d+12,option['definition_type'])
  game=self.alloc(0x600);defs=self.alloc(0x100);self.write(0xb36530,game);self.write(game+0x570,defs);self.write(defs+0x28,c['script_count'])
  self.units=[];ua=self.alloc(4*len(c['owned_units']));self.write(kp+0x214,ua);self.write(kp+0x218,len(c['owned_units']))
  for i,key in enumerate(c['owned_units']):
   p=self.alloc(0x20)if key is not None else 0;self.units.append((p,key));self.write(ua+4*i,p)
  sv=self.alloc(0x200);value=self.alloc(4);self.write(0xb39ab0,sv);self.write(sv+0x108,value)
  sai=self.alloc(0x100)
  for p in self.kingdoms:
   if p:self.call(0x79e170,sai,[p])
  for k,p in zip(c['kingdoms'],self.kingdoms):
   if k and k['native_id'] not in c['registered_ids']:self.write(self.registry+0x20004+4*(k['native_id']&65535),0)
  self.name=self.alloc(64)+16;self.write(self.name-4,1000)
  for pc in [0x4bedf0,0x404f60]:self.u.hook_add(UC_HOOK_CODE,self.diagnostic,begin=pc,end=pc)
 def diagnostic(self,u,pc,size,_):
  if pc==0x4bedf0:self.write(self.arg(),self.name);self.ret(4,self.arg());return
  if pc==0x404f60:self.ret();return
 def summarize(self,c):
  controller=self.read(self.player+12);groups=[];ptrs=[]
  for i in range(23):
   group=self.read(self.read(controller+8)+i*4);lists=[list_values(self,self.read(group+8*j))for j in range(3)];groups.append(lists)
   for ls in lists:ptrs+=ls
  ptrs.sort();keys={p:i for i,p in enumerate(ptrs)};regions={self.read(self.read(self.graph+0x14)+4*i):i for i in range(self.read(self.graph+0x18))};units=dict(self.units);goals=[]
  for p in ptrs:
   kind=VT[self.read(p)];tail={}
   if kind=='Kingdom':tail={'target_id':self.read(p+60),'words40_44':[self.read(p+64),self.read(p+68)]}
   elif kind in ['LandRegion','WaterRegion']:tail={'words3c_44':[self.read(p+x)for x in [60,64,68]],'region':regions[self.read(p+72)],'target_id':self.read(p+76),'neighbors':[keys[n]for n in list_values(self,self.read(p+80))],'hash_buckets':self.read(p+92),'hash_count':self.read(p+96),'hash_flag':bool(self.u.mem_read(p+100,1)[0]),'byte6c':bool(self.u.mem_read(p+108,1)[0])}
   elif kind=='RegionAction':tail={'word3c':self.read(p+60),'region':regions[self.read(p+64)],'word44':self.read(p+68),'scalars48_4c':[self.read(p+72),self.read(p+76)],'byte50':bool(self.u.mem_read(p+80,1)[0])}
   elif kind=='Ability':tail={'category':self.read(p+60),'index':self.read(p+64),'words44_54':[self.read(p+x)for x in [68,72,76,80,84]]}
   elif kind=='Recruitment':tail={'options':[self.option_keys[self.read(self.read(p+60)+4*i)]for i in range(self.read(p+64))],'selected':list(self.u.mem_read(self.read(p+72),self.read(p+76))),'weights':[[self.read(self.read(p+84)+8*i),self.read(self.read(p+84)+8*i+4)]for i in range(self.read(p+88))],'assignments':[self.read(self.read(p+96)+4*i)for i in range(self.read(p+100))]}
   else:tail={'unit':units[self.read(p+60)],'words40_44':[self.read(p+64)]+([self.read(p+68)]if kind=='UnitSecondary'else[])}
   goals.append({'base':base_goal(self,p,100),'kind':kind,'data':tail})
  bindings=[];ais=[]
  for r in range(len(regions)):
   region=self.read(self.read(self.graph+0x14)+4*r);binding=self.read(self.read(self.player+32)+4*r);bindings.append(None if not binding else {'region':r,'kingdom':c['kingdoms'][c['own_slot']]['object']})
   ai=self.read(self.read(region+28)+4*c['player_index']);ais.append({'region':r,'player':200,'land':keys[self.read(ai+44)],'water':keys[self.read(ai+48)],'word34':self.read(ai+52),'arrays':[self.read(ai+x)for x in [12,24,36,60]]})
  return {'ready':bool(self.u.mem_read(self.player+4,1)[0]),'script_words':[self.read(self.read(self.player+0x144)+4*i)for i in range(self.read(self.player+0x148))],'word10':self.read(self.player+16),'word14':self.read(self.player+20),'pending_count':len(list_values(self,self.read(controller+12))),'groups':[[[keys[p]for p in ls]for ls in g]for g in groups],'goals':goals,'region_kingdoms':bindings,'region_players':ais}
def main():
 source=json.loads((ROOT/'research/phase7-navigation/synthetic-topology.json').read_text())['fixtures'];rows=[];visited={}
 for variant in range(18):
  t=source[variant%7];c={'name':t['name']+'_'+str(variant),'input_hex':t['input_hex'],'layout':t['layout'],'kingdoms':[{'object':1,'native_id':0x10001,'major_index':0,'category':0},None,{'object':3,'native_id':0x10003,'major_index':2,'category':0}, {'object':4,'native_id':0x10004,'major_index':3,'category':1}],'own_slot':0,'player_index':variant%7,'neighbor_depth':[-1,0,1,2,5][variant%5],'ability_counts':[variant%3,1,2],'script_count':variant,'owned_units':[71,None,72],'recruitment':None,'registered_ids':[0x10001,0x10003,0x10004]}
  if variant>=7:c['recruitment']={'options':[None,{'object':90,'definition_type':None},{'object':91,'definition_type':0},{'object':92,'definition_type':1},{'object':93,'definition_type':2}],'enabled_types':[[0,0,0],[0,1,0],[1,1,1]][variant%3]}
  if variant%6==5:c['registered_ids']=[0x10001,0x10004]
  if variant%5==3:c['ability_counts']=[0,-1,0];c['owned_units']=[]
  if variant==17:c['kingdoms']=[c['kingdoms'][0]]
  m=Machine(bytes.fromhex(c['input_hex']),c['layout'],[0x200]*64);m.setup(c)
  try:m.call(0x7b4830,m.player);got=m.summarize(c)
  except Exception:print('pc',hex(m.u.reg_read(UC_X86_REG_EIP)));raise
  rows.append({'input':c,'native':got});visited.update(m.executed)
 pins=[]
 for a,n in sorted(visited.items()):
  raw=original.range(a,n);assert raw==modified.range(a,n);pins.append({'va':hex(a),'bytes':raw.hex()})
 (OUT/'native-ai.json').write_text(json.dumps({'schema':1,'passed':True,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(original.b),'v14_sha256':H(modified.b),'fixtures':rows,'native_basic_block_pins':pins,'scope':'Complete7B4830 with concrete constructors,7A4F00 virtual goal registration/deletion,7C8F70 native region neighbor lists and7C40C0 recruitment selection. Source graph and typed fixture profile. Diagnostic name/log, allocation/free, version/byte stream seams only.'},indent=2)+'\n');print({'passed':True,'cases':len(rows)})
if __name__=='__main__':main()
