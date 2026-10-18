"""Original value aggregation and award routing. Only promotion application is mocked."""
from pathlib import Path
import copy,hashlib,json,random,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-body'))
from original_body import IMAGE,ACCEPTED,pe,bits
from unicorn import Uc,UC_ARCH_X86,UC_MODE_32,UC_HOOK_CODE
from unicorn.x86_const import *
ACTOR,COMP,STACK,STOP,OUTPUT=0x2000000,0x2100000,0x2208000,0x2209000,0x220a000
actor=lambda k:0 if k is None else ACTOR+k*0x1000
component=lambda k:0 if k is None else COMP+k*0x1000
class Machine:
 def __init__(self,c):
  self.c=copy.deepcopy(c);self.events=[];self.visited={};self.u=Uc(UC_ARCH_X86,UC_MODE_32);IMAGE.load(self.u);self.u.mem_map(0x2000000,0x300000)
  self.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[c['precision']]);self.u.hook_add(UC_HOOK_CODE,self.hook)
  if c['method']=='value':
   for i,(name,offset,field) in enumerate([('regiment',0x7c,0x6c),('wall',0x98,0x34),('combat',0x74,0x60),('component88',0x88,0x14)]):
    value=c['fields'][name];self.w(actor(1)+offset,component(i+1) if value is not None else 0)
    if value is not None:self.w(component(i+1)+field,value)
  else:self.write_world()
 def w(self,a,v):self.u.mem_write(a,struct.pack('<I',v&0xffffffff))
 def r(self,a):return struct.unpack('<I',self.u.mem_read(a,4))[0]
 def arg(self,i=0):return self.r(self.u.reg_read(UC_X86_REG_ESP)+4+i*4)
 def write_world(self):
  for ident,a in self.c['actors'].items():
   address=actor(int(ident));self.w(address+0xa4,component(a['experience_component']));self.w(address+0x7c,component(a['regiment']));self.w(address+0x80,address+0x500 if a['organization_leader'] is not None else 0);self.w(address+0x510,actor(a['organization_leader']))
  for ident,values in self.c['regiments'].items():
   address=component(int(ident));self.w(address+0x2c,len(values));self.w(address+0x28,address+0x500)
   for i,value in enumerate(values):self.w(address+0x500+i*4,actor(value))
 def hook(self,u,a,n,_):
  if 0x400000<=a<0xc00000:self.visited[a]=n
  if a==0x571a60:
   ident=(u.reg_read(UC_X86_REG_ECX)-COMP)//0x1000
   self.events.append({'component':ident,'amount':self.arg(),'argument':bool(self.arg(1))})
   mutations=self.c['mutations']
   if mutations and mutations[0]['on_component']==ident:
    m=mutations.pop(0)
    for k,changes in m.get('actors',{}).items():self.c['actors'][k].update(changes)
    self.c['regiments'].update(m.get('regiments',{}));self.write_world()
   sp=u.reg_read(UC_X86_REG_ESP);u.reg_write(UC_X86_REG_EIP,self.r(sp));u.reg_write(UC_X86_REG_ESP,sp+12)
 def run(self):
  self.w(STACK,STOP);self.u.reg_write(UC_X86_REG_ESP,STACK);self.u.reg_write(UC_X86_REG_ECX,actor(1));regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
  for i,r in enumerate(regs):self.u.reg_write(r,0x11111111*(i+1))
  if self.c['method']=='value':entry=0x4d01e0;args=[0]
  else:
   entry=0x4ce770;r=self.c['request'];args=[r['amount'],r['argument1'],r['argument2']]
  for i,value in enumerate(args):self.w(STACK+4+i*4,value)
  self.u.emu_start(entry,STOP,count=100000)
  assert self.u.reg_read(UC_X86_REG_EIP)==STOP and self.u.reg_read(UC_X86_REG_ESP)==STACK+4+len(args)*4
  assert [self.u.reg_read(r) for r in regs]==[0x11111111*(i+1) for i in range(4)]
  if self.c['method']=='value':
   self.u.mem_write(STOP,b'\xd9\x15'+struct.pack('<I',OUTPUT)+b'\xdb\x3d'+struct.pack('<I',OUTPUT+8));self.u.emu_start(STOP,STOP+12)
   sig,exp=struct.unpack('<QH',self.u.mem_read(OUTPUT+8,10));return {'stored':self.r(OUTPUT),'extended_significand':sig,'extended_sign_exponent':exp,'abi_preserved':True}
  assert not self.c['mutations'];return {'events':self.events,'actors':self.c['actors'],'regiments':self.c['regiments'],'abi_preserved':True}
def cases():
 rows=[];names=['regiment','wall','combat','component88']
 for mask in range(16):rows.append({'method':'value','name':f'presence_{mask}','fields':{k:bits((i+1)*.1) if mask&(1<<i) else None for i,k in enumerate(names)}})
 for i,values in enumerate([[1.,2**-25,-1.,2**-27],[2**30,.25,-2**30,-.25],[-0.,None,None,None],[None,-0.,None,None],[1e20,-1e20,1.,2**-30],[-1e20,1.,1e20,1.],[.1,.2,.3,.4]]):
  rows.append({'method':'value','name':f'cancellation_{i}','fields':{k:None if v is None else bits(v) for k,v in zip(names,values)}})
 rng=random.Random(0x4d01e0)
 for i in range(32):rows.append({'method':'value','name':f'fractional_{i}','fields':{k:bits(rng.uniform(-10000,10000)) for k in names}})
 def actor_fields(xp=None,reg=None,leader=None):return {'experience_component':xp,'regiment':reg,'organization_leader':leader}
 base={'method':'award','name':'base','request':{'amount':bits(10),'argument1':True,'argument2':True},'actors':{'1':actor_fields(1),'2':actor_fields(2),'3':actor_fields(3),'4':actor_fields(4)},'regiments':{},'mutations':[]}
 def add(name,fn=None):
  c=copy.deepcopy(base);c['name']=name
  if fn:fn(c)
  rows.append(c)
 for amount in [0,-1,.00001,.0001,.00011,1,100000]:add(f'amount_{amount}',lambda c,v=amount:c['request'].update(amount=bits(v)))
 for a in [False,True]:
  for b in [False,True]:
   add(f'flags_without_leader_{a}_{b}',lambda c,a=a,b=b:c['request'].update(argument1=a,argument2=b))
   add(f'flags_with_leader_{a}_{b}',lambda c,a=a,b=b:(c['request'].update(argument1=a,argument2=b),c['actors']['1'].update(organization_leader=2)))
 add('no_components',lambda c:c['actors']['1'].update(experience_component=None))
 add('fallback_leader',lambda c:c['actors']['1'].update(experience_component=None,organization_leader=2))
 add('leader_chain',lambda c:(c['actors']['1'].update(experience_component=None,organization_leader=2),c['actors']['2'].update(experience_component=None,organization_leader=3)))
 add('self_leader_with_component',lambda c:c['actors']['1'].update(organization_leader=1))
 add('regiment',lambda c:(c['actors']['1'].update(experience_component=None,regiment=10),c['regiments'].update({'10':[2,None,3,2]})))
 add('nested_regiment',lambda c:(c['actors']['1'].update(experience_component=None,regiment=10),c['actors']['2'].update(experience_component=None,regiment=11),c['regiments'].update({'10':[2,3],'11':[4,None]})))
 add('regiment_empty',lambda c:(c['actors']['1'].update(experience_component=None,regiment=10),c['regiments'].update({'10':[]})))
 add('regiment_mixed_leaders',lambda c:(c['actors']['1'].update(experience_component=None,regiment=10),c['actors']['2'].update(organization_leader=4),c['regiments'].update({'10':[2,3]})))
 add('leader_changes_experience_component',lambda c:(c['actors']['1'].update(organization_leader=2),c['mutations'].append({'on_component':2,'actors':{'1':{'experience_component':4}}})))
 add('callback_shrinks_members',lambda c:(c['actors']['1'].update(experience_component=None,regiment=10),c['regiments'].update({'10':[2,3,4]}),c['mutations'].append({'on_component':2,'regiments':{'10':[2]}})))
 add('callback_replaces_members',lambda c:(c['actors']['1'].update(experience_component=None,regiment=10),c['regiments'].update({'10':[2,3]}),c['mutations'].append({'on_component':2,'regiments':{'10':[2,4,None]}})))
 add('callback_turns_ancestor_into_recipient',lambda c:(c['request'].update(argument2=False),c['actors']['1'].update(experience_component=None,regiment=10),c['regiments'].update({'10':[2,1]}),c['mutations'].append({'on_component':2,'actors':{'1':{'experience_component':4}}})))
 add('callback_clears_ancestor_regiment',lambda c:(c['request'].update(argument2=False),c['actors']['1'].update(experience_component=None,regiment=10),c['actors']['2'].update(experience_component=None,regiment=11),c['regiments'].update({'10':[2],'11':[3,1]}),c['mutations'].append({'on_component':3,'regiments':{'10':[]}})))
 for p in ['Single24','Double53','Extended64']:
  for row in rows:yield {**copy.deepcopy(row),'name':row['name']+'_'+p,'precision':p}
def main():
 rows=[];visited={}
 for c in cases():
  m=Machine(c);rows.append({'input':c,'output':m.run()});visited.update(m.visited)
 windows=[]
 for a,n in sorted(visited.items()):
  raw=IMAGE.at(a,n);assert raw==ACCEPTED.at(a,n);windows.append({'address':hex(a),'bytes':raw.hex()})
 doc={'original_sha256':pe.ORIGINAL_SHA,'v14_sha256':pe.ACCEPTED_SHA,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'cases':rows,'visited_windows':windows,'seams':['Only571A60 promotion application is replaced in award cases; value cases have no hooks.']}
 path=HERE/'native-experience-fixtures.json';path.write_text(json.dumps(doc,indent=2)+'\n');print(json.dumps({'cases':len(rows),'value':sum(r['input']['method']=='value' for r in rows),'award':sum(r['input']['method']=='award' for r in rows),'sha256':hashlib.sha256(path.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
