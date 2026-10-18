"""Legacy physical-memory projection6F7E90 and application4196F3..41970F.

GlobalMemoryStatusEx is a successful typed OS-input seam. Actual original/v14
instructions read all64-bit physical totals, clamp byte counts at0x7fffffff,
and on wide totals shift the PRIOR +278 word rather than derive total MiB.
Ctor's zero278 is separately byte-pinned, not claimed as a whole ctor run.
The modern backend deliberately uses actual u64 total RAM without that legacy
projection; the700 rounded-MiB pre-request predicate is retained, not a cap.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_ESI
STATUS=0x7700e00
class Policy(Machine):
 def hook(self,u,pc,size,user):
  if getattr(self,'policy_ready',False)and pc==STATUS:
   p=self.arg();assert self.r(p)==64
   self.u.mem_write(p,struct.pack('<IIQQQQQQQ',64,25,self.total,self.available,0x1000000000,0x800000000,0x100000000,0x80000000,0));self.ret(4,1);return
  super().hook(u,pc,size,user)
def main():
 rows=[];pins={};cases=[]
 for total in [1,699<<20,(699<<20)+1,700<<20,701<<20,0x7fffffff,0x80000000,8<<30,64<<30]:
  for prior in [0,0x12345678]:cases.append(dict(total=total,available=total//2,prior_mib_word=prior))
 for image in ['original','v14']:
  m=Policy(image)
  for precision in PC:
   for c in cases:
    m.fpu(precision);m.actual={};m.record_actual=True;m.total=c['total'];m.available=c['available'];m.policy_ready=True
    #6FB1D0 ignores the caller ECX and selects the SAME static System B36578.
    #Query/projection and the419701 comparison must therefore use that owner.
    system=0xb36578;m.w(system+0x270,STATUS);m.w(system+0x278,c['prior_mib_word']);m.w(0xb27a88,system)
    app=m.alloc(0x2b0);m.call(0x6f7e90,this=system)
    m.u.reg_write(UC_X86_REG_ESI,app);m.u.emu_start(0x4196f3,0x41970f,count=1000)
    output=dict(total_bytes_word=m.r(system+0x274),mib_word=m.r(system+0x278),available_bytes_word=m.r(system+0x27c),application_2a0=m.rb(app+0x2a0))
    rows.append(dict(image=image,precision=precision,input=c,output=output));pins.update(m.actual)
 a=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='original']
 b=[{k:v for k,v in row.items()if k!='image'}for row in rows if row['image']=='v14'];assert a==b
 for row in rows:assert row['output']['application_2a0']==int(row['output']['mib_word']<700)
 instructions=[]
 for va,size in sorted(pins.items()):
  if PE0.base<=va<PE0.base+PE0.image_size:
   raw=PE0.range(va,size);assert raw==PE1.range(va,size);instructions.append(dict(va=hex(va),bytes=raw.hex()))
 #Find the exact aligned constructor instruction, then prove its decoded
 #destination/value. This is a static source-owner pin, not executed evidence.
 from capstone import Cs,CS_ARCH_X86,CS_MODE_32
 ctor=[]
 for ins in Cs(CS_ARCH_X86,CS_MODE_32).disasm(PE0.range(0x6f8400,0x400),0x6f8400):
  if (ins.mnemonic=='mov' and ins.op_str=='dword ptr [esi + 0x278], ebx') or (ins.mnemonic=='xor' and ins.op_str=='ebx, ebx' and ins.address<0x6f8460):
   raw=PE0.range(ins.address,ins.size);assert raw==PE1.range(ins.address,ins.size);ctor.append(dict(va=hex(ins.address),bytes=raw.hex(),meaning=ins.op_str))
 assert len(ctor)==2,ctor
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),rows=rows,instructions=instructions,static_constructor_278=ctor)
 out=HERE/'native-request-policy.json'
 if '--write'in sys.argv:out.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(out.read_text())==doc
 print(json.dumps(dict(rows=len(rows),pins=len(instructions),sha256=hashlib.sha256(out.read_bytes()).hexdigest())))
if __name__=='__main__':main()
