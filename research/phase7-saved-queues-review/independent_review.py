"""Independent queue/table replay without rewriting producer artifacts."""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase7-runtime'))
import native_updatables as tables
import native_saved_queues as saved
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
sha=lambda b:hashlib.sha256(b).hexdigest()
def sentinel(m):
 values={r:0x55aa0000+i*123 for i,r in enumerate([UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])}
 for r,v in values.items():m.u.reg_write(r,v)
 return values
def preserved(m,values):
 for r,v in values.items():assert m.u.reg_read(r)==v,(r,m.u.reg_read(r),v)
def ownership(m,expected):
 count=0
 for owner,ptr in m.owners.items():
  p=m.read(ptr+4);previous=ptr+4;seen=set();actual=[]
  while p:
   assert p not in seen;seen.add(p)
   assert m.read(p)==ptr and m.read(p+12)==previous and m.read(previous)==p
   actual.append([m.u.mem_read(p+24,1)[0],m.read(p+20)])
   previous=p+16;p=m.read(p+16);count+=1
  assert actual==expected.get(owner,[]),(owner,actual,expected.get(owner,[]))
 return count
def expected_ownership(snapshot,order):
 out={}
 for qid in order:
  for bucket in snapshot[qid]['buckets']:
   for owner,data in bucket:out.setdefault(owner,[]).insert(0,[qid,data])
 return out

def main():
 table_file=ROOT/'research/phase7-runtime/native-updatables.json';queue_file=ROOT/'research/phase7-runtime/native-saved-queues.json'
 table=json.loads(table_file.read_text());queues=json.loads(queue_file.read_text());table_cases=[];maps=[]
 assert len({json.dumps(c,sort_keys=True)for c in table['cases']})==18
 for c in reversed(table['cases']):
  m=tables.Machine(c['count']);rng=m.read(m.world+0xd4)
  for op in c['ops']:
   registers=sentinel(m)
   if op['operation']=='load':ok=m.load(op['version'],op['id'],op['owner'])
   else:ok=bool(m.call(0x50b120,m.table,[m.owner(op['owner'])])&255)
   assert ok==op['ok'] and m.slots()==op['slots']
   if ok or op['operation']=='remove':preserved(m,registers)
   assert m.read(m.world+0xd4)==rng
  table_cases.append({'count':c['count'],'scenario':c['scenario'],'operations':len(c['ops'])})
 for index,c in enumerate(reversed(queues['maps'])):
  source=ROOT.parent/c['map'];chunks=saved.read_chunks(source);data=chunks['UPDATABLES'][0]
  assert data.hex()==c['payload'] and sha(data)==c['sha256']
  assert struct.unpack_from('<I',chunks['GWORLD'][0])[0]==c['world_version']
  m=saved.Machine(data);before=m.read(m.world+0xd4);registers=sentinel(m);m.load(c['world_version']);preserved(m,registers)
  assert m.bound==c['boundaries'] and m.snapshot()==c['snapshot']
  assert m.read(m.world+0xd4)==before
  order=[3,4]+([0,1,2]if c['world_version']>=22 else[])
  count=ownership(m,expected_ownership(c['snapshot'],order))
  maps.append(dict(map=c['map'],source_sha256=sha(source.read_bytes()),chunk_sha256=sha(data),owner_links=count))
  if(index+1)%20==0:print('validated original maps',index+1,flush=True)
 assert 0x85d840 not in saved.q.VISITED,'registration/load unexpectedly executed RNG'
 #Re-load a real chunk into nonempty native queues. Verify clearing leaves no
 #dangling owner links, saved file order is restored, and reset precedes clear.
 c=queues['maps'][0];m=saved.Machine(bytes.fromhex(c['payload']));reset_observations=[]
 for qid in range(5):m.action({'type':'add','queue':qid,'owner':100000+qid,'data':9000+qid,'delay_bits':saved.q.bits(.01)})
 for qid in [3,4]:m.write(m.queues[qid]+28,saved.q.bits(17.25+qid))
 rng=m.read(m.world+0xd4)
 clear_addresses={m.read(m.read(m.queues[qid])):qid for qid in [3,4]}
 def observe(u,pc,size,_):
  if pc in clear_addresses:
   ecx=u.reg_read(UC_X86_REG_ECX)
   if ecx in [m.queues[3],m.queues[4]]:
    current=m.queues.index(ecx);value=m.read(ecx+28);assert value==0
    reset_observations.append({'queue':current,'last_end_before_clear':value})
 m.u.hook_add(UC_HOOK_CODE,observe);m.load(c['world_version'])
 assert m.snapshot()==c['snapshot'] and m.read(m.world+0xd4)==rng
 ownership(m,expected_ownership(c['snapshot'],[3,4]+([0,1,2]if c['world_version']>=22 else[])))
 assert len(reset_observations)==2,reset_observations
 visited=[]
 for pc,code in sorted(saved.q.VISITED.items()):
  raw=bytes.fromhex(code);assert saved.q.ORIGINAL.range(pc,len(raw))==raw==saved.q.ACCEPTED.range(pc,len(raw))
  visited.append({'va':hex(pc),'sha256':sha(raw)})
 report={'status':'native-replay-passed','table_count_domains':6,'table_sequence_variants':len(table_cases),'tables':table_cases,'maps':maps,'reset_before_clear':reset_observations,'visited_windows':len(visited),'native_rng_unchanged_during_load':True,'producer_pins':{str(p.relative_to(ROOT)).replace('\\','/'):sha(p.read_bytes())for p in [table_file,queue_file,Path(tables.__file__),Path(saved.__file__)]},'scope':['Initial queue headers/vtables/counts are manually established fixture state; full original queue Load and node construction execute.','Original table50B470 executes. Duplicate registration stops before native diagnostic throw; id==count out-of-allocation is excluded.','Every original saved ID maps to a fixture Updatable, not actual restored game actors/components.','RNG equality is checked before/after each native registration/load; prepopulation uses real periodic RNG before the measured restoration.']}
 p=HERE/'native-replay.json';p.write_text(json.dumps(report,indent=2)+'\n');print('PASS',sha(p.read_bytes()))
if __name__=='__main__':main()
