"""Independent native replay and extra motion boundary cases; no producer edits."""
from pathlib import Path
import copy,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT.parent/'reconstruction/tools/python-packages'));sys.path.insert(0,str(HERE))
import native_motion as motion,native_range as ranges
from unicorn.x86_const import *
H=lambda b:hashlib.sha256(b).hexdigest()
def extra_cases():
 original=list(motion.cases())
 for pc in ['Single24','Double53','Extended64']:
  base=next(copy.deepcopy(c)for c in original if c['operation']=='pursue'and c['precision']==pc and c['moving']and c['target_decision']==4)
  for index,(s,t)in enumerate([([1,1],[1,-1]),([1.0000001192092896,1],[1,-1]),([1,1.0000001192092896],[1,-1]),([.3,.7],[-.7,.3]),([-.3,-.7],[.7,-.3]),([.001,.003],[.006,-.002]),([1e-20,-1e-20],[1e-20,1e-20])]):
   c=copy.deepcopy(base);c['name']=f'peer_dot_{index}_{pc}';c['actors']['1']['facing']=list(map(motion.bits,s));c['actors']['3']['facing']=list(map(motion.bits,t));yield c
  for index,change in enumerate([{'mind_actor':2,'actors':{'2':{'active':None}}},{'mind_actor':2,'actors':{'2':{'mover':None,'active':3}}},{'decision':{'target':4},'actors':{'1':{'modifier':False}}}]):
   c=copy.deepcopy(base);c['name']=f'peer_think_{index}_{pc}';c['mutations']=[{'event':'think',**change}];yield c
  c=copy.deepcopy(base);c['name']=f'peer_speed_replaces_mover_{pc}';c['mutations']=[{'event':'speed','actors':{'1':{'mover':2,'active':2}},'decision':{'target':4},'delta':motion.bits(.875)}];yield c
  base=next(copy.deepcopy(c)for c in original if c['operation']=='wait'and c['precision']==pc and c['decision']['kind']==1)
  for index,change in enumerate([{'decision':{'kind':6,'ability':2}},{'mind_actor':2,'decision':{'kind':6}},{'bombard_mode':2,'decision':{'kind':6}}]):
   c=copy.deepcopy(base);c['name']=f'peer_schedule_{index}_{pc}';c['mutations']=[{'event':'schedule',**change}];yield c
def main():
 ledger=json.loads((HERE/'findings.json').read_text());pins={};rebounds=[];visited={}
 for path,h in ledger['files'].items():
  raw=(ROOT/path).read_bytes();actual=H(raw)
  if actual!=h:
   assert path=='crates/aa-mind/src/lib.rs',path
   prior=raw.decode().replace('pub mod element_move_continuation;\n','');assert H(prior.encode())==h
   rebounds.append({'file':path,'prior':h,'current':actual,'reason':'Only additive element_move_continuation module export; removing that exact LF line reproduces original digest.'})
  pins[path]=actual
 doc=json.loads((HERE/'native-motion-fixtures.json').read_text());count={}
 for row in reversed(doc['cases']):
  m=motion.Machine(copy.deepcopy(row['input']));assert m.run()==row['output'],row['input']['name'];visited.update(m.visited);op=row['input']['operation'];count[op]=count.get(op,0)+1
 doc=json.loads((HERE/'native-range-fixtures.json').read_text())
 for row in reversed(doc['cases']):
  m=ranges.Machine(copy.deepcopy(row['input']));reset=m.reset;regs=[UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP];sent=[0x6758493a+i*0x1020304 for i in range(4)]
  def reviewer_reset(v):
   reset(v)
   for r,value in zip(regs,sent):m.u.reg_write(r,value)
  m.reset=reviewer_reset
  assert m.run_range()==row['output'],row['input']['name'];assert[m.u.reg_read(r)for r in regs]==sent
  for a,b in m.executed.items():visited[a]=bytes.fromhex(b)
 count['effective_range']=len(doc['cases']);extra=[]
 for c in extra_cases():
  m=motion.Machine(c);extra.append({'input':c,'output':m.run()});visited.update(m.visited)
 for a,b in visited.items():assert b==motion.original.range(a,len(b))==motion.modified.range(a,len(b)),hex(a)
 for name,record in ledger['ranges'].items():
  a=int(record['va'],16);raw=motion.original.range(a,record['length']);assert H(raw)==record['sha256']and raw==motion.modified.range(a,record['length']),name
 for p,h in pins.items():assert H((ROOT/p).read_bytes())==h,p
 (HERE/'independent-extra-motion.json').write_text(json.dumps({'schema':1,'cases':extra},indent=2))
 out={'schema':1,'status':'native replay passed','counts':count,'additional_native_cases':len(extra),'visited_instruction_windows':len(visited),'explicit_shared_pin_rebinds':rebounds,'source_pins':pins,'producer_ledger_sha256':H((HERE/'findings.json').read_bytes()),'reviewer_sha256':H(Path(__file__).read_bytes()),'extra_sha256':H((HERE/'independent-extra-motion.json').read_bytes())}
 p=HERE/'independent-replay.json';p.write_text(json.dumps(out,indent=2));print(json.dumps({'counts':count,'extra':len(extra),'visited':len(visited),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
