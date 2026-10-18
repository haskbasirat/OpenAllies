"""Original state enter/leave, view selection and state FX matching branches."""
from pathlib import Path
import sys,json,random,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];sys.path.insert(0,str(ROOT/'research/phase7-actors'));from actor_states import Machine as Base,P,V,H
from unicorn.x86_const import *
F=lambda x:struct.unpack('<I',struct.pack('<f',x))[0]
class Machine(Base):
 def __init__(self,c):
  super().__init__();self.c=c;self.events=[];self.nodes=0x20f0000;self.body=0x20b2000;self.props=0x20b1000;self.org=0x20b0000;self.effects=0x20d0000;self.property_base=0x20e1000
  self.register();self.events=[]
  self.w(self.thing+4,self.definition);self.w(self.thing+8,c['flags']);self.w(self.definition+0x1c,c['def_flags']);self.w(self.thing+0x100,c['state']&0xffffffff);self.w(self.thing+0x104,c['state']>>32);self.w(self.thing+0x15c,c['old_view']&0xffffffff)
  self.w(self.thing+0x60,self.body if c['has_body'] else 0);self.w(self.body+0x10,c['health']);self.w(self.body+0x14,c['maximum']);self.w(self.thing+0x68,0x20b4000 if c['character'] else 0)
  self.w(self.thing+0x7c,self.org if c['organization'] else 0);self.w(self.org+4,self.thing);self.w(self.org+0xb4,F(2));self.w(self.org+0x70,c['pending']);self.w(0xb36534,self.world);self.w(self.world+0xe0,F(10))
  self.w(self.thing+0x84,0x20b5000 if c['economy'] else 0);self.w(self.thing+0x78,self.props if c['properties'] else 0);self.w(self.props+0x10,0x20e3000);self.w(self.props+0x14,len(c['available_properties']))
  for i,p in enumerate(c['available_properties']):r=0x20e3100+i*0x40;self.w(0x20e3000+i*4,r);self.w(r+0xc,self.prop(p));self.w(r+0x10,1 if i%2 else 0)
  for key,requirements in c['requirements'].items():self.w(self.prop(int(key))+0x38,self.list([self.prop(r) for r in requirements]))
  self.desc=self.records[c['index']]['address'];self.w(self.desc+0x34,self.list([self.prop(x) if x is not None else 0 for x in c['state_properties']]))
  self.u.mem_write(self.desc+0x1e,bytes([int(c['entrenchable'])]));self.u.mem_write(self.desc+0x1f,bytes([int(c['remove_group'])]))
  if c['old'] is not None:self.u.mem_write(self.records[c['old']]['address']+0x1e,bytes([int(c['old_entrenchable'])]))
  self.w(self.definition+0x188,0x20b6000);self.w(self.definition+0x18c,len(c['views']))
  for i,v in enumerate(c['views']):p=0x20b7000+i*0x40;self.w(0x20b6000+4*i,p);self.w(p+0x10,v['limit']);self.w(p+0x14,v['mask']);self.w(p+0xc,self.prop(v['property']) if v['property']is not None else 0)
  for off in [0x194,0x19c]:self.w(self.definition+off,self.list([self.fx(i,f) for i,f in enumerate(c['fx'])]))
  self.w(0x20d1000+0x20,0x301f500);self.w(self.thing+0x164,0x90002 if c['group'] else 0);self.w(0xb364c0,self.manager);self.w(self.manager+8,9);self.w(self.manager+0x2000c,0x20e9000 if c['group_valid'] else 0)
  self.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[c['precision']])
 def list(self,values):
  head=0
  for value in reversed(values):p=self.nodes;self.nodes+=8;self.w(p,value);self.w(p+4,head);head=p
  return head
 def prop(self,id):return self.property_base+id*0x100
 def propid(self,p):return (p-self.property_base)//0x100 if p else None
 def fx(self,index,f):p=self.effects+index*0x100;self.w(p,0x20d1000);self.w(p+0x34,f['state']);return p
 def hook(self,u,pc,size,data):
  if P.base<=pc<P.base+P.image_size:self.observed[pc]=bytes(u.mem_read(pc,size))
  if pc in [0x4d4a90,0x4d16b0]:return
  if pc==0x570020:
   self.events.append(['economy'])
   if self.c['economy_health'] is not None:self.w(self.body+0x10,self.c['economy_health'])
   self.ret()
  elif pc==0x5817a0:
   self.events.append(['add',self.propid(self.arg()),self.arg(1)]);self.ret(pop=12)
  elif pc==0x581810:self.events.append(['remove',self.propid(self.arg())]);self.ret(pop=8)
  elif pc==0x5638b0:self.events.append(['character',self.arg()if self.arg()<0x80000000 else self.arg()-0x100000000]);self.w(self.thing+0x15c,self.arg());self.ret(pop=4)
  elif pc==0x57bff0:self.events.append(['pending',self.arg()]);self.ret(pop=4)
  elif pc==0x4f0460:self.ret(int(self.c['active_only']))
  elif pc==0x301f500:
   index=(u.reg_read(UC_X86_REG_ECX)-self.effects)//0x100;self.events.append(['fx_active',index]);self.ret(int(self.c['fx'][index]['active']))
  elif pc==0x4293c0:self.events.append(['fx_emit','enter' if u.reg_read(UC_X86_REG_ECX)==self.definition+0x194 else 'leave',self.arg(2),bool(self.arg(3))]);self.ret(pop=16)
  elif pc==0x4dff30:self.events.append(['remove_group']);self.ret(pop=4)
  else:super().hook(u,pc,size,data)
 def run(self):
  if self.c['mode']=='enter':self.invoke(0x4d4a90,self.thing,self.desc,self.records[self.c['old']]['address']if self.c['old']is not None else 0)
  else:self.invoke(0x4d16b0,self.thing,self.desc)
  return {'events':self.events,'view':self.r(self.thing+0x15c) if self.r(self.thing+0x15c)<0x80000000 else self.r(self.thing+0x15c)-0x100000000,'flags':self.r(self.thing+8),'organization_time':self.r(self.org+0xb4),'health':self.r(self.body+0x10)}
def main():
 rows=[];observed={}
 for pc in [24,53,64]:
  for index in range(32):
   viewcount=index%4;views=[{'limit':F([.25,.5,1][v]),'mask':[0,1<<8,1<<12][v],'property':[2,None,3][v]}for v in range(viewcount)]
   for mode in ['enter','leave']:
    c={'mode':mode,'precision':pc,'index':[8,12,24,27,35][index%5],'old':[None,0,6,27][index%4],'flags':0x1020,'def_flags':0x1000 if index%2 else 0,'state':[0,1<<8,1<<12,(1<<8)|(1<<12)][index%4],
    'old_view':-1 if index%3==0 or not viewcount else 0,'has_body':index%7!=0,'health':F([1,25,50,100][index%4]),'maximum':F(100),'economy_health':F(10)if index%5==0 else None,'character':index%3==0,'organization':index%2==0,'properties':index%7!=0,'economy':index%4!=0,'pending':3 if index%3==0 else 0,
    'state_properties':[1,None,2,3],'available_properties':[1,3]if index%2 else [3],'requirements':{'1':[],'2':[1],'3':[1,3]},'entrenchable':index%3!=0,'old_entrenchable':index%5==0,'remove_group':index%2==0,'group':index%3!=0,'group_valid':index%4!=0,'views':views,'fx':[{'state':s,'active':bool((index+v)%2)}for v,s in enumerate([0,8,12,24,35])],'active_only':index%2==0}
    if c['old']==c['index']:c['old']=0
    m=Machine(c);rows.append({'input':c,'output':m.run()});observed.update(m.observed)
 for a,b in observed.items():assert b==P.range(a,len(b))==V.range(a,len(b))
 out={'schema':1,'cases':rows,'source_sha256':H(P.b),'v14_sha256':H(V.b),'producer_sha256':H(Path(__file__).read_bytes()),'instructions':[{'va':hex(a),'bytes':b.hex()}for a,b in sorted(observed.items())],'scope':'Full4D4A90/4D16B0 plus actual45E620/4B4590 view predicates,57C400 entrenchmenttime,4D0A00 firstFXgate,5035E0/581280/5032D0 propertyprerequisites with originaldisabledinstances. Propertyadd/remove, Economyrefresh,Character transition,FXemitter,Groupremove andpendingOrgapplication are observed external services. Definition lists stable; nativeinvalidindex/null domains excluded.'}
 p=HERE/'native-state-effects.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'cases':len(rows),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()

