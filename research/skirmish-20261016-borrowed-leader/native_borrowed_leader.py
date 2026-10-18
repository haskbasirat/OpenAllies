"""Execute unchanged 579A00/5797E0 on both pinned images, all x87 PCs.
Field-boundary instruction observers are explicit mutation/dependency probes,
not claims that native ordinary field reads are callbacks. RNG step, native
logging and Mind+114 are required named service seams; arithmetic is native.
"""
from pathlib import Path
import copy, hashlib, json, random, struct, sys
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'))
import harness as h
from harness import UC_HOOK_CODE,UC_HOOK_MEM_WRITE,UC_X86_REG_EAX,UC_X86_REG_ECX,UC_X86_REG_EDX,UC_X86_REG_ESI,UC_X86_REG_EDI
import native_formation as nf
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
ORIGINAL=h.P
class Dependency(Exception):pass

class Borrowed(nf.FormationMachine):
 def __init__(self,c,image):
  h.P=ORIGINAL if image=='original' else h.V
  super().__init__(c)
  self.trace=[];self.counts={};self.slot=None
  self.arrays=[self.positions,self.alloc(max(8,self.n*8))]
  for i,xy in enumerate(c.get('second_positions',c['organization']['slot_positions'])):self.w32(self.arrays[1]+8*i,*xy)
  self.owner=self.r32(self.org+4)
  self.formations=[]
  fp=self.r32(self.org+0x74)
  if fp:self.formations.append(fp)
  for a in c.get('missing_elements',[]):self.w32(self.actors[a]+0x80,0)
  self.u.hook_add(UC_HOOK_CODE,self.reads)

 def identity(self,p):
  return next((k for k,v in self.actors.items() if v==p),None)
 def change(self,a):
  k=a['kind'];v=a.get('value')
  if k=='leader':self.w32(self.org+0x40,self.actors[v] if v is not None else 0)
  elif k=='positions':self.w32(self.org+0x78,self.arrays[v])
  elif k=='slot':self.w32(self.r32(self.actors[a['actor']]+0x80)+0x18,v)
  elif k=='offset':self.w32(self.arrays[a['array']]+8*a['slot']+4*a['axis'],v)
  elif k=='mind':self.w32(self.owner+0x70,0)
  elif k=='member':self.w32(self.r32(self.org+0x28)+4*a['slot'],self.actors[v] if v is not None else 0)
  else:raise AssertionError(k)
 def event_boundary(self,name,*args):
  self.trace.append([name,*args]);self.counts[name]=self.counts.get(name,0)+1
  for a in self.c.get('mutations',[]):
   if a['event']==name and a['occurrence']==self.counts[name]:self.change(a)
  stop=self.c.get('stop')
  if stop and stop['event']==name and stop['occurrence']==self.counts[name]:raise Dependency(name)
 def reads(self,u,pc,size,_):
  r=u.reg_read;e=self.event_boundary
  if pc==0x5797e0:self.slot=self.arg()
  elif pc in [0x579a03,0x579a22,0x5797fa]:e('leader')
  elif pc in [0x579a11,0x579a29]:e('element_slot',self.identity(r(UC_X86_REG_EAX)))
  elif pc==0x5797f4:e('member',self.slot)
  elif pc==0x579805:e('primary_slot')
  elif pc==0x57982f:e('formation')
  elif pc==0x579836:e('formation_oriented',self.formations.index(r(UC_X86_REG_EDX)))
  elif pc==0x57983f:e('owner_facing')
  elif pc==0x578250:e('formation_offset',self.slot)
  elif pc in [0x57980f,0x579862,0x579887,0x5798a7,0x5798da,0x579910,0x57992f]:e('positions')
  elif pc==0x5798fa:e('element_attached',self.identity(r(UC_X86_REG_EAX)))
  elif pc in [0x579907,0x57990d]:e('actor_position',self.identity(r(UC_X86_REG_EAX)),0 if pc==0x579907 else 1)
  elif pc in [0x57990a,0x579913]:e('actor_position',self.identity(r(UC_X86_REG_ECX)),0 if pc==0x57990a else 1)
  elif pc==0x579929:e('owner_id')
  elif pc==0x579936:e('actor_id',self.identity(r(UC_X86_REG_EAX)))
  elif pc in [0x5798e7,0x5798eb]:
   addr=r(UC_X86_REG_ESI)+(0 if pc==0x5798e7 else 4);self.offset_event(addr)
  elif pc==0x579932:self.offset_event(r(UC_X86_REG_ECX)+self.slot*8+4)
  elif pc==0x579943:self.offset_event(r(UC_X86_REG_ECX))
  elif pc==0x579a3a:e('owner_mind')
  elif pc==0x579a0c:e('store_leader',self.identity(r(UC_X86_REG_ECX)))
  elif pc in [0x57981b,0x579823,0x579893,0x57989b,0x444abb,0x444abd,0x57991b,0x57991d]:
   self.store_event(r(UC_X86_REG_EAX)+(4 if pc in [0x579823,0x57989b,0x444abd,0x57991d] else 0))
  elif pc in [0x57986a,0x57986f]:self.store_event(r(UC_X86_REG_ECX)+(4 if pc==0x57986f else 0))
  elif pc in [0x5798e9,0x5798ee]:self.store_event(r(UC_X86_REG_ESI)+(4 if pc==0x5798ee else 0))
 def offset_event(self,address):
  for j,a in enumerate(self.arrays):
   if a<=address<a+self.n*8:self.event_boundary('offset',j,(address-a)//8,((address-a)%8)//4);return
  raise AssertionError(hex(address))
 def store_event(self,address):
  for j,a in enumerate(self.arrays):
   if a<=address<a+self.n*8:self.event_boundary('store_offset',j,(address-a)//8,((address-a)%8)//4);return
  raise AssertionError(hex(address))
 def random(self,m):
  self.event_boundary('random');super().random(m)
 def diagnostic(self,m):
  self.event_boundary('diagnostic');super().diagnostic(m)
 def execute(self):
  mind=self.r32(self.owner+0x70)
  if mind:
   stub=self.r32(self.r32(mind)+0x114);old=self.seams[stub]
   self.seams[stub]=lambda m:(self.event_boundary('mind_leader_changed'),old(m))
  outcome='ok'
  try:self.run()
  except h.NullDereference:outcome='native_null'
  except Dependency as error:outcome=str(error)
  return {'outcome':outcome,'leader':self.identity(self.r32(self.org+0x40)),
   'positions':[[self.r32(a+8*i,2) for i in range(self.n)] for a in self.arrays],
   'current_positions':self.arrays.index(self.r32(self.org+0x78)),
   'events':self.events,'trace':self.trace,'remaining_draws':self.draws}

def cases():
 rows=[copy.deepcopy(c) for c in nf.cases() if c['operation'] in ['compute','set_leader']]
 rng=random.Random(0x579a00)
 for i in range(30):
  o=nf.organization(rng,6,[6],oriented=i%2,leader_slot=1,primary=0,empty=(4,),detached=(3,))
  old=copy.deepcopy(o['members'][1]);o['leader']=old
  new=copy.deepcopy(o['members'][[1,2,3,5][i%4]]) if i%5 else None
  if new is None:o['members'][1]['attached']=0
  rows.append({'name':f'leader_generated_{i}','operation':'set_leader','organization':o,'new_leader':new,'draws':[]})
 base=nf.organization(rng,4,[4],oriented=0,leader_slot=1,primary=0,empty=(2,))
 def add(name,o,new=None,**kw):rows.append({'name':name,'operation':'set_leader','organization':copy.deepcopy(o),'new_leader':copy.deepcopy(new),'draws':[],**kw})
 # Same identity still computes the same slot twice.
 add('same_pointer_twice',base,base['leader'])
 add('raw_store_before_old_element_fault',base,base['members'][3],missing_elements=[101])
 neg=copy.deepcopy(base);neg['leader']['slot']=-1;neg['members'][1]['slot']=-1
 add('negative_old_new_slot',neg,neg['leader'])
 # Old slot emptied means RNG service can mutate the next leader and array.
 scatter=copy.deepcopy(base);scatter['members'][1]=None
 add('rng_changes_leader',scatter,scatter['members'][3],draws_override=True)
 rows[-1]['draws']=[0x12345678,0x80000001];rows[-1].pop('draws_override')
 rows[-1]['mutations']=[{'event':'random','occurrence':2,'kind':'leader','value':100}]
 add('rng_replaces_positions',scatter,scatter['members'][3])
 rows[-1]['draws']=[2047,0xffffffff]
 rows[-1]['second_positions']=[[h.f2b(11+i),h.f2b(-17-i)] for i in range(4)]
 rows[-1]['mutations']=[{'event':'random','occurrence':1,'kind':'positions','value':1}]
 add('store_changes_leader',base,base['members'][3],mutations=[{'event':'store_offset','occurrence':2,'kind':'leader','value':100}])
 add('diagnostic_changes_leader',base,base['members'][3])
 rows[-1]['organization']['members'][1]['attached']=0
 rows[-1]['mutations']=[{'event':'diagnostic','occurrence':1,'kind':'leader','value':100}]
 add('mind_removed_by_slot_store',base,base['members'][3],mutations=[{'event':'store_offset','occurrence':2,'kind':'mind','value':None}])
 # Each lazy boundary is tested as a mandatory dependency error.
 probe=copy.deepcopy(rows[-4]);probe['mutations']=[];probe['name']='probe'
 probe['precision']='Double53';m=Borrowed(probe,'original');out=m.execute()
 seen={}
 for event in out['trace']:
  k=event[0];seen[k]=seen.get(k,0)+1
  c=copy.deepcopy(probe);c['name']=f'stop_{k}_{seen[k]}';c['stop']={'event':k,'occurrence':seen[k]};rows.append(c)
 return rows

def main():
 rows=[]
 for pc in h.PRECISION:
  for source in cases():
   c=copy.deepcopy(source);c['precision']=pc
   for image in ['original','v14']:
    c2=copy.deepcopy(c);c2['image']=image;c2['output']=Borrowed(c2,image).execute();rows.append(c2)
 for i in range(0,len(rows),2):assert rows[i]['output']==rows[i+1]['output'],rows[i]['name']
 doc={'schema':1,'scope':__doc__,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'case_count':len(rows),'cases':rows,'instructions':[{'va':hex(k),'bytes':v} for k,v in sorted(h.VISITED.items())]}
 out=HERE/'native-borrowed-leader.json';out.write_text(json.dumps(doc,indent=1)+'\n')
 asm=[]
 for start,size in [(0x579a00,0x52),(0x5797e0,0x194),(0x578250,0x3e)]:
  asm.extend(f'{i.address:08x}: {i.mnemonic} {i.op_str}' for i in Cs(CS_ARCH_X86,CS_MODE_32).disasm(ORIGINAL.range(start,size),start))
 (HERE/'native-instructions.txt').write_text('\n'.join(asm)+'\n')
 print(json.dumps({'cases':len(rows),'instructions':len(h.VISITED),'sha256':hashlib.sha256(out.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
