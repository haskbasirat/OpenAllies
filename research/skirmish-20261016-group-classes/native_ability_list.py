"""Original ActorData Performer list append and AbilityDefinition Flags fragments.

The folder name follows the initial task; these are AbilityData references,
not ActorClass. Original 4655E6..465797 performs ordered enumeration, local
AbilityData construction, duplicate-name warning, and append. The per-entry
4BF990 loader is a seam supplying a completed definition pointer. Original
4BF200 registers flag names into a supplied registry through a registration
leaf seam; 4BF67F..4BF6BE reads supplied typed bool values and sets +54 bits.
This is bounded native evidence, not complete source/ActorData initialization.
"""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1];GAME=ROOT.parent
sys.path.insert(0,str(GAME/'reconstruction/tools/python-packages'))
sys.path.insert(0,str(ROOT/'research/phase6-mapheader'))
from native_header import Machine as Base,P,V
from unicorn import UC_HOOK_MEM_READ,UC_HOOK_MEM_WRITE
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32
sha=lambda b:hashlib.sha256(b).hexdigest()
class Machine(Base):
 def __init__(self,pc):
  super().__init__(b'');self.visited={};self.rows=[];self.next_row=0;self.loads=[];self.warnings=[];self.names=[];self.bool_reads=[];self.bools=[]
  self.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[pc])
  self.u.hook_add(UC_HOOK_MEM_READ|UC_HOOK_MEM_WRITE,self.null_guard,begin=0,end=0xfff)
 def null_guard(self,u,access,address,size,value,data):
  assert bytes(u.mem_read(u.reg_read(UC_X86_REG_EIP),1))==b'\x64',f'non-FS pagezero at {u.reg_read(UC_X86_REG_EIP):x}'
 def text(self,p):
  b=bytearray()
  while (unit:=bytes(self.u.mem_read(p,2)))!=b'\0\0':b.extend(unit);p+=2;assert len(b)<4096
  return b.decode('utf-16-le')
 def hook(self,u,pc,size,data):
  if P.base<=pc<P.base+P.image_size:
   raw=bytes(u.mem_read(pc,size));assert raw==P.range(pc,size)==V.range(pc,size);self.visited[pc]=raw
  if pc==0x5b4220:assert self.text(self.arg())=='Ability';self.ret(4,len(self.rows));return
  if pc==0x42be10:self.ret(value=0x100000+self.next_row);return
  if pc==0x4bf990:
   local=u.reg_read(UC_X86_REG_ECX);index=self.next_row;assert self.arg(2)==0x100000+index
   self.write(local,self.rows[index]);self.loads.append(index);self.next_row+=1;self.ret(12);return
  if pc==0x404ed0:self.warnings.append(self.text(self.arg(4)));self.ret();return
  if pc==0x4849f0:
   vector=u.reg_read(UC_X86_REG_ECX);old,count=self.read(vector),self.read(vector+4);new=self.alloc(self.arg()*4)
   if count:self.u.mem_write(new,bytes(self.u.mem_read(old,count*4)))
   self.write(vector,new);self.write(vector+8,self.arg());self.ret(4);return
  if pc==0x4bf060:
   index,name=self.arg(),self.text(self.arg(1));assert index==len(self.names)
   self.names.append(name);entry=self.alloc(8);self.write(entry+4,self.arg(1));self.write(self.flag_table+index*4,entry);self.ret(12);return
  if pc==0x5b4410:assert self.text(self.arg())=='Flags' and self.arg(1)==1;self.ret(8,value=0x12345678);return
  if pc==0x5b5380:
   name=self.text(self.arg());index=len(self.bool_reads);assert name==self.names[index];self.bool_reads.append(name);self.ret(4,self.bools[index]);return
  super().hook(u,pc,size,data)
 def call(self,entry,this=0):
  self.write(self.stack,self.stop);self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_ECX,this);self.u.emu_start(entry,self.stop,count=1000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
 def run_list(self,rows):
  actor=self.alloc(0x600);self.write(actor+8,self.string(b'o\0w\0n\0e\0r\0'));self.write(actor+0x1ec,self.alloc(12));self.write(actor+0x1f0,0);self.write(actor+0x1f4,3)
  for name,flags in rows:
   definition=self.alloc(0xa0);self.write(definition+0x10,self.string(name.encode('utf-16-le')));self.write(definition+0x54,flags);self.rows.append(definition)
  self.u.reg_write(UC_X86_REG_ESP,self.stack);self.u.reg_write(UC_X86_REG_EBP,actor);self.u.reg_write(UC_X86_REG_EBX,0x12345678)
  self.u.emu_start(0x4655e6,0x465797,count=1000000);assert self.u.reg_read(UC_X86_REG_ESP)==self.stack
  local=[self.read(self.read(actor+0x1ec)+4*i) for i in range(self.read(actor+0x1f0))]
  defs=[self.read(p) for p in local];assert len(set(local))==len(rows)
  assert defs==self.rows and self.loads==list(range(len(rows)))
  return dict(input=rows,output_flags=[self.read(p+0x54) for p in defs],output_names=[self.text(self.read(p+0x10)) for p in defs],loads=self.loads,unique_local_abilities=len(set(local)),warnings=self.warnings)
 def run_flags(self,values):
  self.flag_table=self.alloc(44);self.write(0xb3b5e4,self.flag_table);self.call(0x4bf200)
  self.bools=values;definition=self.alloc(0xa0);self.write(definition+0x54,0)
  self.u.reg_write(UC_X86_REG_ESI,definition);self.u.reg_write(UC_X86_REG_EDI,0);self.u.reg_write(UC_X86_REG_EBP,0x12345678);self.u.reg_write(UC_X86_REG_ESP,self.stack)
  self.u.emu_start(0x4bf67f,0x4bf6be,count=100000);assert self.u.reg_read(UC_X86_REG_ESP)==self.stack
  bits=self.read(definition+0x54);assert bits==sum((1<<i) for i,v in enumerate(values) if v&255)
  return dict(input_bool_bytes=values,names=self.names,read_order=self.bool_reads,output_flags=bits)
def main():
 list_inputs=[[],[('one',8)],[('same',8),('same',0)],[('same',0),('Same',8),('same',16)],[('a',0),('b',8),('a',8),('c',0),('d',0)],[(str(i%3),8 if i==9 else i*16) for i in range(12)]]
 flag_inputs=[[0]*11]+[[int(i==j) for i in range(11)] for j in range(11)]+[[1]*11,[i%2 for i in range(11)],[255 if i==3 else 0 for i in range(11)]]
 lists=[];flags=[];visited={}
 for pc in [24,53,64]:
  for values in list_inputs:
   m=Machine(pc);lists.append(dict(precision=pc,**m.run_list(values)));visited.update(m.visited)
  for values in flag_inputs:
   m=Machine(pc);flags.append(dict(precision=pc,**m.run_flags(values)));visited.update(m.visited)
 md=Cs(CS_ARCH_X86,CS_MODE_32);windows=[]
 for label,start,end in [('ability_list',0x4655e6,0x465797),('organization_consumer',0x578a30,0x578ad0),('flags_registration',0x4bf200,0x4bf2b3),('flags_load',0x4bf67f,0x4bf6be),('ability_definition_full_loader',0x4bf390,0x4bf990),('performer_constructor',0x57f4d0,0x57f610)]:
  raw=P.range(start,end-start);assert raw==V.range(start,end-start)
  windows.append(dict(label=label,start=f'{start:08x}',end=f'{end:08x}',sha256=sha(raw),instructions=[f'{i.address:08x} {i.bytes.hex()} {i.mnemonic} {i.op_str}' for i in md.disasm(raw,start)]))
 out=dict(scope=__doc__,original_sha256=sha(P.b),v14_sha256=sha(V.b),producer_sha256=sha(Path(__file__).read_bytes()),list_cases=lists,flag_cases=flags,list_passed=len(lists),flags_passed=len(flags),instruction_pins=[dict(address=f'{a:08x}',bytes=b.hex()) for a,b in sorted(visited.items())],static_windows=windows,seams=['5B4220 count and42BE10 iteration return supplied ordered Ability nodes.','4BF990 supplies completed AbilityDefinition pointer; source loading, shared lookup and inline definition creation are not reexecuted.','4BE9B0 local constructor executes; allocator and vector growth leaves are seams, preserving existing vector prefix.','4BF060 flag registration leaf records original supplied index/name;5B4410 returns required Flags node;5B5380 returns supplied typed bool byte.','Duplicate warning4O4ED0 logging is recorded without host logging.','Ordinary pagezero memory access is rejected; only FS-prefixed accesses are allowed.'])
 (HERE/'native-ability-list.json').write_text(json.dumps(out,indent=2)+'\n');print('passed',len(lists),'lists',len(flags),'flags')
if __name__=='__main__':main()
