"""Whole76D820 over the actual captured231 registration shapes. Values are
explicit arbitrary laboratory words, NOT observed currentSVars. A disabled
or positively suspended actual synchronizer returns before OS/thread/SAI gates.
Actual String temporary refcounts execute and balance; the protected variable
storage, registry, SyncA and RNG bytes remain identical. Allocator/RNG/OS/log
append entry after the dump starts is forbidden. No complete SVar constructor,
positive logging traversal/current-value binding or battle is certified.
"""
from pathlib import Path
import collections,hashlib,itertools,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/resource-work-depot'))
from native_work_depot import Machine as Base,base
from native_bootstrap import SYNC_A
from unicorn import UC_HOOK_MEM_WRITE
CAPTURE=ROOT/'research/development10/native-svar-inventory-capture-02/capture.json'
CAPTURE_SHA='ca5037d596281761ae3c4b479c6746ffb771b20751f3e9543364bed80202edf8'
class Required(Exception):pass
class Machine(Base):
 def __init__(self,image):
  self.in_dump=False;super().__init__(image);self.changes=[]
  self.u.hook_add(UC_HOOK_MEM_WRITE,self.write_watch)
 def write_watch(self,u,access,address,size,value,user):
  if self.in_dump and address<0x10000000 and not 0x110000<=address<0x130000:
   self.changes.append([u.reg_read(26),address,size,value])
 def hook(self,u,pc,size,user):
  if self.in_dump and pc in(0x77dc50,0x77da70,0x77d930,0x77d9d0,0x77db10,0x40f8d0):
   self.actual[pc]=size;return #execute REAL logger/wrapper, bypass inherited logging seams
  if self.in_dump and pc in(0x6f7810,0x6f7430,0x85d840,0x77d680):raise AssertionError(('forbidden leaf',hex(pc)))
  super().hook(u,pc,size,user)
 def run(self,entries,precision,enabled,suspend,word):
  self.fpu(precision);self.begin(1,list(range(16)),0);self.watch=True
  self.w(0xb3655c,SYNC_A);self.b(SYNC_A,enabled);self.w(SYNC_A+4,suspend&0xffffffff)
  array=self.alloc(4*len(entries));self.w(0xb39ab0,array);self.w(0xb39ab4,len(entries))
  storage=[];string_refs=[]
  for e in entries:
   var=self.alloc(0x2c);self.w(array+4*e['ordinal'],var)
   for off in range(0,0x1c,4):self.w(var+off,word)
   self.cs(var+0x1c,e['name']);self.w(var+0x20,e['word20']);self.w(var+0x24,e['kind'])
   name=self.r(var+0x1c);storage.append((name-16,16+2*(len(e['name'])+1)))
   if e['kind']==4:
    self.cs(var+0x10,'fixture\u03bb_biome_path');value=self.r(var+0x10)
    storage.append((value-16,16+2*(len('fixture\u03bb_biome_path')+1)));string_refs.append((value-4,self.r(value-4)))
   storage.append((var,0x2c))
  storage += [(array,4*len(entries)),(0xb39ab0,8),(SYNC_A,0x1c),(self.S+0x128,4)]
  before=[bytes(self.u.mem_read(a,n))for a,n in storage]
  #Any reached OS call would jump to NULL and fail, instead of a successful seam.
  self.w(0x9f01e4,0)
  self.changes=[];self.actual.clear();self.in_dump=True
  try:self.call(0x76d820,[0xb39aa8])
  finally:self.in_dump=False
  after=[bytes(self.u.mem_read(a,n))for a,n in storage]
  assert before==after,[(hex(a),x.hex(),y.hex())for(a,n),x,y in zip(storage,before,after)if x!=y]
  assert all(self.r(a)==n for a,n in string_refs)
  return dict(precision=precision,enabled=enabled,suspend=suspend,word=word,
   registrations=len(entries),protected_ranges=len(storage),unchanged=True,
   string_refcounts=[n for a,n in string_refs],executed_native_string_copy=0x4012c0 in self.actual,
   executed_native_text_wrapper=0x40f8d0 in self.actual,executed_native_text_logger=0x77db10 in self.actual)
def inventory():
 raw=CAPTURE.read_bytes();assert hashlib.sha256(raw).hexdigest()==CAPTURE_SHA
 c=json.loads(raw);assert c['completed_source_return']and c['folder_redirection_observed']and c['original_archives_preserved']and c['original_error']is None
 assert c['source_exe_sha256']=='3bda77d7e516121f7c8904cf73cf72a23efe04b5c1d04ef273284695c8efc5c1'
 assert c['cleanup']['exit_process_event_observed']and c['cleanup']['exit_code']==0 and not c['cleanup']['errors']
 r=c['captures'][-1];assert r['label']=='source_return'and r['x87_control']==r['legacy_x87_control']==r['fxsave_x87_control']==0x27f and r['legacy_fxsave_match']
 v=r['registered_svars'];assert v['manager']==0xb39aa8 and v['count']==231 and len(v['entries'])==231
 entries=[]
 for i,e in enumerate(v['entries']):
  assert e['ordinal']==e['word20']==i and e['kind']in range(1,6)and '\0'not in e['name']
  h=struct.unpack('<IIII',bytes.fromhex(e['name_header']));assert h[1]==len(e['name'])and h[2]>=h[1]and h[3]>0
  if e['kind']==4:
   h=struct.unpack('<IIII',bytes.fromhex(e['value_string_header']));assert h[1]==e['value_string_length']and h[2]>=h[1]and h[3]>0
  entries.append({k:e[k]for k in('ordinal','name','kind','word20')})
 assert dict(collections.Counter(e['kind']for e in entries))=={1:21,2:169,3:36,4:1,5:4}
 assert [e['name']for e in entries if e['kind']==4]==['MapDefaultBiome']
 return entries
def produce():
 entries=inventory();rows=[];pins={}
 for image in('original','v14'):
  m=Machine(image)
  for pc,(enabled,suspend),word in itertools.product(base.PC,((0,0),(0,-1),(1,1),(1,7)),(0,0x80000000,0x7fc00001,0x7f800000,0xffffffff)):
   rows.append(dict(image=image,**m.run(entries,pc,enabled,suspend,word)));pins.update(m.actual)
 return dict(passed=True,scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  capture_sha256=CAPTURE_SHA,registrations=entries,cases=rows,executed_windows=base.proof_ranges(pins))
if __name__=='__main__':
 result=produce();p=HERE/'native-svar-inactive.json'
 if '--write'in sys.argv:p.write_text(json.dumps(result,indent=2)+'\n')
 else:assert result==json.loads(p.read_text())
 print('passed',len(result['cases']),'rows',len(result['executed_windows']),'windows')
