"""Native B28114/18/1C initialized unset-position sentinel, never overwritten.
Actual constructors428620/4286E0, comparators428170/428430 and pointer-escape
42A710->429080 execute on the original mapped data in both images/allPC.
"""
from pathlib import Path
import sys,json,hashlib,itertools,struct,csv,bisect
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn import UC_HOOK_MEM_WRITE
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
from capstone.x86_const import X86_OP_MEM,X86_OP_IMM
POINT=0xb28114;EXPECTED=[0xc2c80000]*3
def machine(image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 assert [m.r(POINT+4*i)for i in range(3)]==EXPECTED
 def immutable(*args):raise AssertionError('native wrote immutable FX sentinel')
 m.u.hook_add(UC_HOOK_MEM_WRITE,immutable,begin=POINT,end=POINT+11)
 return m
def run(c,image,pc):
 m=machine(image,pc);kind=c['kind']
 if kind=='compare':
  p=m.array(c['position']);value=m.call(c['function'],p,[POINT])&255
  output=dict(value=value,globals=[m.r(POINT+4*i)for i in range(3)])
 elif kind=='constructor':
  p=m.alloc(0x6c);m.u.mem_write(p,b'\xcd'*0x6c);position=m.array(c['position']);pair=m.array([0,0]);translation=m.array([1,2,3])
  args=[pair,c['time'],781,2696,position]
  if c['function']==0x4286e0:args +=[c['state'],c['active']]
  else:args +=[0x3f000000,0xbf800000,translation]
  assert m.call(c['function'],p,args)==p
  assert [m.r(p+0x24+4*i)for i in range(3)]==EXPECTED
  output=dict(target_position=[m.r(p+0x24+4*i)for i in range(3)],source_position=[m.r(p+0x14+4*i)for i in range(3)],state=m.r(p+0x54),byte58=m.rb(p+0x58),globals=[m.r(POINT+4*i)for i in range(3)])
 else:
  world=m.alloc(0x150);manager=m.alloc(16);app=m.alloc(0x60);view=m.alloc(0xa0)
  m.w(0xb36534,world);m.w(world+0x130,manager);m.wb(manager,c['muted']);m.w(world+0xe0,0x3f800001)
  m.w(0xb364d4,app);m.w(app+0x48,42);m.w(0xb36538,view);m.w(view+0x8c,0x40000002)
  pair=m.array([0x76543210 if c['nonempty'] else 0,0]);output_command=None
  m.w(0x9f01e4,m.stub(lambda mm:mm.ret(c['thread'])))
  def allocate(mm):
   assert mm.arg(1)==0x6c;p=m.alloc(0x6c);m.u.mem_write(p,b'\xcd'*0x6c);mm.ret(p)
  m.seam(0x6f7810,allocate)
  def submit(mm):
   nonlocal output_command
   p=mm.arg(1);output_command=dict(source_position=[m.r(p+0x14+4*i)for i in range(3)],target_position=[m.r(p+0x24+4*i)for i in range(3)],time=m.r(p+0x38),flag=mm.arg(2));mm.ret(pop=8)
  m.seam(0x50c800,submit);m.call(0x42a710,pair,[0,POINT]);output=dict(command=output_command,globals=[m.r(POINT+4*i)for i in range(3)])
 assert output['globals']==EXPECTED
 return output,m.visited
def direct_references():
 rows=list(csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t'));starts=[int(r['address'],16)for r in rows]
 decoder=Cs(CS_ARCH_X86,CS_MODE_32);decoder.detail=True;result=[]
 for address in range(POINT,POINT+12,4):
  needle=struct.pack('<I',address);hits=[]
  for vs,r,size,off in nm.original.sections:
   data=nm.original.b[off:off+size];at=data.find(needle)
   while at>=0:hits.append(nm.original.base+r+at);at=data.find(needle,at+1)
  other=[]
  for vs,r,size,off in nm.modified.sections:
   data=nm.modified.b[off:off+size];at=data.find(needle)
   while at>=0:other.append(nm.modified.base+r+at);at=data.find(needle,at+1)
  assert hits==other
  for hit in hits:
   f=rows[bisect.bisect_right(starts,hit)-1];entry=int(f['address'],16)
   # Ghidra bodyBytes can omit shared/noncontiguous tail blocks. Decode
   #through the next declared function boundary for the referenced instruction.
   next_entry=starts[bisect.bisect_right(starts,entry)]
   instructions=list(decoder.disasm(nm.original.range(entry,next_entry-entry),entry))
   current=next(i for i in instructions if i.address<=hit<i.address+i.size)
   assert bytes(current.bytes)==nm.modified.range(current.address,current.size)
   if current.mnemonic=='push':
    assert current.operands[0].type==X86_OP_IMM and current.operands[0].imm==POINT
    calls=[i for i in instructions if i.address>current.address and i.mnemonic=='call'];target=calls[0].operands[0].imm
    assert target in [0x428170,0x428430,0x42a710]
    mode='address passed to proved read-only point reader'
   else:
    assert current.mnemonic=='mov' and current.operands[-1].type==X86_OP_MEM and current.operands[-1].mem.disp==address
    mode='absolute source read'
   result.append(dict(global_address=hex(address),literal_location=hex(hit),function=hex(entry),pc=hex(current.address),bytes=bytes(current.bytes).hex(),instruction=current.mnemonic+' '+current.op_str,mode=mode))
 return result
def main():
 cases=[];positions=[EXPECTED,[0,0,0],[0x3f800001,0x40000002,0x40400003]]
 for function,position,time in itertools.product([0x428620,0x4286e0],positions,[0,0x3f123456]):
  if function==0x4286e0:
   for state,active in itertools.product([0,41,42],[0,1]):cases.append(dict(kind='constructor',function=function,position=position,time=time,state=state,active=active))
  else:cases.append(dict(kind='constructor',function=function,position=position,time=time))
 comparisons=[EXPECTED,[0,0,0]]
 for i in range(3):
  p=list(EXPECTED);p[i]^=1;comparisons.append(p)
 for function,position in itertools.product([0x428170,0x428430],comparisons):cases.append(dict(kind='compare',function=function,position=position))
 for muted,nonempty,thread in itertools.product([False,True],[False,True],[41,42]):cases.append(dict(kind='pointer_escape',muted=muted,nonempty=nonempty,thread=thread))
 rows=[];pins={}
 for pc,c in itertools.product(nm.PRECISIONS,cases):
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 references=direct_references();assert len(references)==34
 data=nm.original.range(POINT,12);assert data==nm.modified.range(POINT,12)
 doc=dict(schema=1,scope=__doc__,rows=rows,native_executions=2*len(rows),initialized_data=dict(va=hex(POINT),bytes=data.hex(),bits=EXPECTED),direct_reference_census=references,instructions=[dict(va=hex(v),bytes=b.hex())for v,b in sorted(pins.items())])
 path=HERE/'native-fx-static-position.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),references=len(references),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
