"""Bounded direct-address audit; does not prove absence of arbitrary alias writes."""
from pathlib import Path
import bisect,csv,hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'))
from native_machine import original,modified,Machine
from capstone import Cs,CS_ARCH_X86,CS_MODE_32,CS_AC_WRITE
cs=Cs(CS_ARCH_X86,CS_MODE_32);cs.detail=True
functions=list(csv.DictReader((ROOT.parent/'reconstruction/research/native-aa/functions.tsv').open(),delimiter='\t'))
functions.sort(key=lambda r:int(r['address'],16));starts=[int(r['address'],16)for r in functions]
def audit(image,target):
 pattern=struct.pack('<I',target);at=0;refs=[]
 while True:
  at=image.b.find(pattern,at)
  if at<0:break
  address=None
  for _,rva,size,offset in image.sections:
   if offset<=at<offset+size:address=image.base+rva+at-offset;break
  if address is not None:
   index=bisect.bisect_right(starts,address)-1;function=functions[index]if index>=0 else None;match=None
   if function is not None:
    entry=int(function['address'],16);size=int(function['bodyBytes'])
    if entry<=address<entry+size:
     for ins in cs.disasm(image.range(entry,size),entry):
      if ins.address<=address<ins.address+ins.size:
       match=dict(address=hex(ins.address),bytes=ins.bytes.hex(),mnemonic=ins.mnemonic,operands=ins.op_str,operand_access=[o.access for o in ins.operands],write=any(o.access&CS_AC_WRITE for o in ins.operands if o.type==3));break
   if match is None:
    #Several tiny vtable getters are absent from Ghidra's function inventory.
    #Record the exact absolute-MOV opcode separately, without claiming a
    #function-boundary or executed-xref proof for this fallback.
    for start in [address-1,address-2]:
     decoded=list(cs.disasm(image.range(start,8),start))
     if decoded:
      ins=decoded[0]
      if ins.mnemonic=='mov' and ins.address+ins.size==address+4 and any(o.type==3 and o.mem.disp==target for o in ins.operands):
       match=dict(address=hex(ins.address),bytes=ins.bytes.hex(),mnemonic=ins.mnemonic,operands=ins.op_str,operand_access=[o.access for o in ins.operands],write=any(o.access&CS_AC_WRITE for o in ins.operands if o.type==3),boundary='absolute-MOV opcode fallback; function boundary/execution not claimed');break
   refs.append(dict(immediate_address=hex(address),function=function['address']if function else None,instruction=match))
  at+=4
 try:initial=image.range(target,4).hex()
 except ValueError:
  #Mapped virtual-section tail is zero-filled by the PE loader; no arbitrary
  #observed/captured current value is inferred from this constructor seed.
  assert any(image.base+rva<=target<image.base+rva+virtual and target>=image.base+rva+raw for virtual,rva,raw,_ in image.sections)
  initial='00000000'
 return dict(initial_bytes=initial,references=refs)
report=dict(scope=__doc__,targets={})
for target in [0xb282f0,0xb3da08]:
 a=audit(original,target);b=audit(modified,target);assert a==b
 report['targets'][hex(target)]=a
report['qualification']='B282F0 is an initial1 fresh-Application owner with no ported mutators; direct references are audited, not an arbitrary-alias/perpetual native immutability proof. B3DA08 is initial0 and advances in real5AE160. Specialized external task publications must share this same current counter before later actual-view certification.'
path=HERE/'global-direct-audit.json';path.write_text(json.dumps(report,indent=2)+'\n')
print(json.dumps(dict(sha256=hashlib.sha256(path.read_bytes()).hexdigest(),targets={k:dict(refs=len(v['references']),unresolved=sum(r['instruction']is None for r in v['references']))for k,v in report['targets'].items()})))
