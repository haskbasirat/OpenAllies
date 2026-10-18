"""Independent reverse-order replay, ABI checks and actual Flag11->4D1350 bridge.
No producer corpus, production source or original game file is modified.
"""
from pathlib import Path
import copy,hashlib,importlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools/python-packages'))
sys.path.insert(0,str(HERE))
from unicorn.x86_const import *
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()
REGS=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP]
SENT=[0x718294a5,0x8293a5b6,0x93a4b6c7,0xa4b5c7d8]
def seed(machine):
 for r,v in zip(REGS,SENT):machine.u.reg_write(r,v)
def preserved(machine):assert[machine.u.reg_read(r)for r in REGS]==SENT
def main():
 ledger=json.loads((HERE/'actor-live-evidence.json').read_text());counts={};observed={};pins={}
 for name,h in ledger['source_sha256'].items():assert sha(ROOT/name)==h,name;pins[name]=h
 for name,h in ledger['dependency_sha256'].items():assert sha(ROOT/name)==h,name;pins[name]=h
 modules={name:importlib.import_module(name)for name in ['ability_flags','organization_status','state_effects','property_applicability','property_flags']}
 base=modules['ability_flags'].Base;original_invoke=base.invoke
 def checked_invoke(self,*args):
  seed(self);value=original_invoke(self,*args);preserved(self);return value
 base.invoke=checked_invoke
 for module,filename in [('ability_flags','native-ability-flags.json'),('organization_status','native-organization-status.json'),('state_effects','native-state-effects.json'),('property_applicability','native-property-applicability.json'),('property_flags','native-property-flags.json')]:
  data=json.loads((HERE/filename).read_text());mod=modules[module]
  for index,row in enumerate(reversed(data['cases'])):
   machine=mod.Machine(copy.deepcopy(row['input']))
   if module=='property_applicability':seed(machine)
   result=machine.run();assert result==row['output'],(module,index,result,row['output'])
   if module=='property_applicability':preserved(machine)
   for a,b in getattr(machine,'observed',{}).items():observed[a]=b
   if hasattr(machine,'visited'):
    for a,n in machine.visited.items():observed[a]=mod.base.IMAGE.at(a,n)
  counts[module]=len(data['cases'])
  print(module,counts[module],flush=True)
 flags=modules['property_flags'];data=json.loads((HERE/'native-property-flags.json').read_text())
 for row in reversed(data['loader_cases']):
  machine=flags.Loader(copy.deepcopy(row['input']));seed(machine);result=machine.run();assert result==row['output']
  if 'error'not in result:preserved(machine)
  for a,n in machine.visited.items():observed[a]=flags.base.IMAGE.at(a,n)
 counts['flag_loaders']=len(data['loader_cases'])
 machine=flags.Register(next(flags.cases()));machine.registered=[];machine.call(0x4c1110,0x3703000,[]);assert machine.registered==data['registration'];counts['flag_registrations']=len(machine.registered)
 for a,n in machine.visited.items():observed[a]=flags.base.IMAGE.at(a,n)
 data=json.loads((ROOT/'research/phase7-properties/native-effects-fixtures.json').read_text())
 for row in reversed(data['cases']):
  machine=flags.base.Machine(copy.deepcopy(row['input']));assert machine.run()==row['output']
  for a,n in machine.visited.items():observed[a]=flags.base.IMAGE.at(a,n)
 counts['numeric_regression']=len(data['cases'])
 # Unlike the producer's observed Actions service, execute4D1350 and571640.
 class Integrated(flags.Machine):
  def __init__(self,c,dead):
   super().__init__(c);self.bridge=[];actor=flags.base.ACTOR;self.w(actor+8,dead);self.w(actor+0xf0,0x123);self.w(actor+0xf4,0x15)
   element=0x3706000;self.w(actor+0x80,element);self.w(element+4,actor);self.w(element+0x14,0x3707000);self.w(element+0x1c,1)
  def hook(self,u,a,n,user):
   if a==0x4d1350:
    self.visited[a]=n;self.bridge.append({'entry_flags':self.r(flags.base.PROP+0x1c)});return
   if a==0x4cfcc0:
    self.visited[a]=n;self.bridge.append({'changed_actions':self.r(flags.base.ACTOR+0xf0)|(self.r(flags.base.ACTOR+0xf4)<<32)});self.ret();return
   super().hook(u,a,n,user)
 integrated=[]
 for dead in [False,True]:
  c=next(c for c in flags.cases()if c['name']=='mixed_order_Double53');c['mover']=c['character']=c['economy']=False
  machine=Integrated(c,int(dead));result=machine.run();expected=0 if dead else 1<<30
  assert [e['changed_actions']for e in machine.bridge if 'changed_actions'in e]==[expected]
  # During removal Flag9 has not yet been recomputed: source descriptor order
  # is Flag11, Add3, Flag9, Multiply19. Flag0 belongs to the second property.
  assert [e['entry_flags']for e in machine.bridge if 'entry_flags'in e]==[1<<11,(1<<9)|1]
  assert machine.r(flags.base.ACTOR+0xf0)==expected and machine.r(flags.base.ACTOR+0xf4)==0
  integrated.append({'actor_dead':dead,'events':machine.bridge,'final_actions':expected,'property_final':result[-1]})
  for a,n in machine.visited.items():observed[a]=flags.base.IMAGE.at(a,n)
 counts['actual_mixed_flag11_rebuild_sequences']=len(integrated)
 P=modules['ability_flags'].P;V=modules['ability_flags'].V
 for a,b in observed.items():assert b==P.range(a,len(b))==V.range(a,len(b)),hex(a)
 for name,h in pins.items():assert sha(ROOT/name)==h,('source_changed',name)
 report={'schema':1,'status':'native replay passed; source review and Rust test acceptance recorded separately','counts':counts,'independent_nonvolatile_checks':'Actor entrypoints and recursive applicability/valid flag loaders add new EBX/ESI/EDI/EBP sentinels; property lifetime already checks each call.', 'actual_visited_windows':len(observed),'integrated_flag11_rebuild':integrated,'pins':pins,'producer_ledger_sha256':sha(HERE/'actor-live-evidence.json'),'reviewer_script_sha256':sha(Path(__file__))}
 p=HERE/'independent-replay.json';p.write_text(json.dumps(report,indent=2));print(json.dumps({'counts':counts,'visited':len(observed),'sha256':sha(p)}))
if __name__=='__main__':main()
