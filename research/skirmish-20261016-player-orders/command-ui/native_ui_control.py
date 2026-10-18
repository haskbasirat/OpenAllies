"""Original bounded48CF0A..48CF97 ActorCommand UI-control writes.
Explicit typed node GetBoolean/GetInteger seams supply raw fixture fields;
496F30 actual behavior-flag test runs unmodified. Stop-before-field dependency
rows retain native partial stores. No prior UI resources/full loader/ctor claim.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
P=h.P;V=h.V;NAMES=['non_group_command','apply_group_offset','aggressive_command','can_hot_command_actor','can_hot_command_location','high_importance','preferred_position']
class Stop(Exception):pass
class Machine(h.Machine):
 def run(self,c):
  obj=self.alloc(0x50);node=self.alloc(0x20);self.u.mem_write(obj,b'\xcd'*0x50);self.w32(obj+0xc,c['kind']);self.w32(obj+0x2c,c['behavior']);self.trace=[]
  def get(m):
   key=m.arg();raw=bytearray()
   while int.from_bytes(m.u.mem_read(key,2),"little"):raw.extend(m.u.mem_read(key,2));key+=2
   name=raw.decode('utf-16-le');self.trace.append(name)
   if name==c.get('stop'):raise Stop(name)
   index=NAMES.index(name);value=c['fields'][index];m.ret(4,value=value)
  self.seams[0x5b5380]=get;self.seams[0x5b4ed0]=get
  self.u.reg_write(UC_X86_REG_ESI,obj);self.u.reg_write(UC_X86_REG_EDI,node);self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  status='complete'
  try:self.u.emu_start(0x48cf0a,0x48cf97,count=10000);assert self.u.reg_read(UC_X86_REG_EIP)==0x48cf97 and self.esp()==h.STACK and self.fpu_top()==0
  except Stop as e:status=str(e)
  assert self.u.reg_read(UC_X86_REG_ESI)==obj and self.u.reg_read(UC_X86_REG_EDI)==node
  return {'status':status,'bytes':list(self.u.mem_read(obj+0x3c,6)),'position':self.r32(obj+0x44),'reads':self.trace}
def main():
 cases=[{'kind':5,'behavior':0,'fields':list(bits)+[0xffffffff]}for bits in itertools.product([0,1],repeat=6)]
 cases +=[{'kind':k,'behavior':b,'fields':[0]*6+[0x80000000]}for k,b in itertools.product([0,5,11,12,34],[0,0x10,0x10000,0x800000,0x1000000,0xffffffff])]
 cases +=[{'kind':5,'behavior':b,'fields':[1,0,1,0,1,0,0x7fffffff],'stop':stop}for b,stop in itertools.product([0,0x10000],NAMES)]
 rows=[]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for c in cases:rows.append({'image':label,'precision':pc,'input':c,'output':Machine(pc).run(c)})
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 H=lambda b:hashlib.sha256(b).hexdigest();out={'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'rows':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-ui-control.json';p.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()

