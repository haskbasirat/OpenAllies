"""Native522F60/523000 field order with real4364C0/570650, explicit522890
command child and5200B0/520190 predicates; callback mutation/cutoff prefixes.
Also actual derived Mind vf78 getters at535E70/6A71F0. No full world tick.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE,UC_HOOK_CODE
OLD=h.P;V14=h.V
class Cutoff(Exception):pass
class M(h.Machine):
 def run(self,c):
  mind=self.alloc(0x300);state=mind+0x34;actor=self.alloc(0x100);org=self.alloc(0x80);array=self.alloc(16)
  self.w32(mind+4,actor);self.w32(state+4,mind);self.w32(actor+0x7c,org);self.w32(org+0x28,array,4)
  self.w32(state+0x14,c['retry']);self.w8(state+0x10,128);self.w8(state+0x18,c['flag']);self.w32(mind+0x28c,c['mode'])
  ptrs={};reverse={0:0};el={}
  for i in [1,2,3]:
   p=self.alloc(0x100);e=self.alloc(0x20);ptrs[i]=p;reverse[p]=i;el[i]=e;self.w32(p+0x80,e);self.w32(e+4,p);self.w32(e+0x10,actor);self.w8(e+0x1c,int(i!=2 or c['attach2']))
  for i,v in enumerate([1,0,2,3]):self.w32(array+i*4,ptrs.get(v,0))
  self.w32(org+0x44,ptrs[c['primary']]);self.trace=[];calls=0
  def arrived(m):
   nonlocal calls
   member=reverse[m.arg(0)];flag=m.arg(1);m.trace.append(['dispatch',member,flag]);calls+=1
   if c['fail']==calls:raise Cutoff()
   if c['mutation']=='flag':m.w8(state+0x18,(m.r8(state+0x18)+1)&255)
   if c['mutation']=='mode':m.w32(mind+0x28c,2)
   if c['mutation']=='truncate':m.w32(org+0x2c,0)
   m.ret(8)
  def active(m):m.trace.append(['active',c['active']]);m.ret(value=int(c['active']))
  def close(m):m.trace.append(['close',c['close']]);m.ret(value=int(c['close']))
  def read(u,access,a,size,v,_):
   if a==state+0x14:self.trace.append(['read_retry',self.r32(a)])
   if a==mind+0x28c:self.trace.append(['mode',self.r32(a)])
   if a==state+0x18:self.trace.append(['flag',self.r8(a)])
  def write(u,access,a,size,v,_):
   if a==state+0x14:self.trace.append(['retry',v&0xffffffff])
   if a==state+0x10:self.trace.append(['pending',v&255])
  def code(u,a,size,_):
   if a==0x570650:self.trace.append(['primary',reverse[self.r32(self.ecx()+4)]])
  self.u.hook_add(UC_HOOK_MEM_READ,read);self.u.hook_add(UC_HOOK_MEM_WRITE,write);self.u.hook_add(UC_HOOK_CODE,code)
  self.seams[0x522890]=arrived;self.seams[0x5200b0]=active;self.seams[0x520190]=close
  error=None
  try:self.call(c['entry'],state)
  except Cutoff:error=c['fail']
  return dict(trace=self.trace,retry=self.r32(state+0x14),pending=self.r8(state+0x10),flag=self.r8(state+0x18),mode=self.r32(mind+0x28c),error=error)
def main():
 h.VISITED.clear();rows=[];getters=[]
 cases=[]
 for entry,mode,flag,retry,attach,mutation in itertools.product([0x522f60,0x523000],[0,2],[0,128],[0,0xffffffff],[False,True],[None,'flag','mode','truncate']):
  cases.append(dict(entry=entry,mode=mode,flag=flag,retry=retry,attach2=attach,primary=3,mutation=mutation,fail=None,active=False,close=False))
 for active,close,retry in [(True,False,0),(False,True,0),(False,False,2),(False,False,0x80000000)]:cases.append(dict(entry=0x523000,mode=2,flag=255,retry=retry,attach2=True,primary=1,mutation=None,fail=None,active=active,close=close))
 for fail in[1,2]:cases.append(dict(entry=0x522f60,mode=0,flag=255,retry=0xffffffff,attach2=True,primary=1,mutation='flag',fail=fail,active=False,close=False))
 for pc,c in itertools.product(h.PRECISION,cases):
  h.P=OLD;a=M(pc).run(c);h.P=V14;b=M(pc).run(c);assert a==b,(pc,c,a,b);rows.append(dict(pc=pc,input=c,output=a))
 for pc,kind,status in itertools.product(h.PRECISION,['Element','Regiment','Aircraft'],[0,1,2,0xffffffff]):
  c=dict(kind=kind,status=status);out=[]
  for image in[OLD,V14]:
   h.P=image;m=h.Machine(pc);mind=m.alloc(0x80);m.w32(mind+0x54,status);out.append(m.call(0x535e70 if kind=='Element' else 0x6a71f0,mind)&255)
  assert out[0]==out[1];getters.append(dict(pc=pc,input=c,output=out[0]))
 h.P=OLD;seams=[0x522890,0x5200b0,0x520190];pins={hex(a):v for a,v in sorted(h.VISITED.items())if a not in seams}
 construct_path=ROOT/'research/phase7-mind/native-constructors.json';construct_raw=construct_path.read_bytes();construct=json.loads(construct_raw);metadata=[]
 for kind in ['base','element','regiment','aircraft']:
  row=next(r for r in construct['cases']if r['input']['kind']==kind);vt=int.from_bytes(bytes.fromhex(row['output']['initialized_hex'])[:4],'little');entry=OLD.u32(vt+0x78);assert OLD.range(vt+0x78,4)==V14.range(vt+0x78,4)
  expected={'base':0x96cac2,'element':0x535e70,'regiment':0x6a71f0,'aircraft':0x6a71f0}[kind];assert entry==expected
  metadata.append(dict(kind=kind,vtable=hex(vt),word_va=hex(vt+0x78),word_bytes=OLD.range(vt+0x78,4).hex(),entry=hex(entry)))
 doc=dict(schema=1,scope=__doc__,constructor_fixture_sha256=hashlib.sha256(construct_raw).hexdigest(),static_vtable_words=metadata,static_qualification='These vtable words are read-only image pins, not executed in the getter rows; constructor identities cite the separately executed full constructor fixture.',rows=rows,getters=getters,pins=pins,excluded_entries=[hex(a)for a in seams],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 raw=(json.dumps(doc,indent=2)+'\n').encode();p=HERE/'native-sleep-fields.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),getter_rows=len(getters),runs=2*(len(rows)+len(getters)),executed_pins=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
