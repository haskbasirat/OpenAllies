"""Prove when original ground movement consumes the constructor-unwritten +48."""
from pathlib import Path
import copy,hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase4-movement'))
import native_oracle as native
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP
H=lambda b:hashlib.sha256(b).hexdigest()
def run(pe,case,poison=None):
 m=native.Machine(pe,case);reads=[]
 if poison is not None:m.put(native.M+0x48,poison)
 def read(u,access,address,size,value,user):
  if address<=native.M+0x48<address+size:reads.append(hex(u.reg_read(UC_X86_REG_EIP)))
 m.u.hook_add(UC_HOOK_MEM_READ,read);return m.run(),reads
def main():
 p=native.Pe(ROOT.parent/'AA.exe',native.ORIGINAL_SHA);v=native.Pe(ROOT.parent/'reconstruction/native-game/runtime-expanded-v14/AA.exe',native.ACCEPTED_SHA)
 for a,z in [(0x5773c0,0x577495),(0x5774a0,0x577a6c),(0x5767c0,0x5767e6)]:assert p.at(a,z-a)==v.at(a,z-a)
 rows=[];executions=0
 for case in native.cases():
  output,reads=run(p,case);executions+=1
  poison_results=[]
  if not reads:
   for poison in [0xcccccccc,0x7fc12345]:
    result,actual_reads=run(p,case,poison);executions+=1;assert not actual_reads
    result['mover_bits']['target_range']=output['mover_bits']['target_range']
    assert result==output
    poison_results.append(poison)
  rows.append({'input':case,'range_read_addresses':reads,'poison_values_proven_unread':poison_results})
 out={'original_sha256':native.ORIGINAL_SHA,'v14_sha256':native.ACCEPTED_SHA,'producer_sha256':H(Path(__file__).read_bytes()),'dependency_sha256':H(Path(native.__file__).read_bytes()),
 'native_executions':executions,'cases':rows,'scope':'Native original range reads instrumented. For every unconsumed path both poison and NaN +48 words leave all other fields and callbacks bit-identical. Target paths are instrumented with a valid finite range; Rust None rejection is an explicit boundary, not native invalid-memory parity.'}
 path=HERE/'mover-range-reads.json';path.write_text(json.dumps(out,indent=2)+'\n');print(json.dumps({'cases':len(rows),'executions':executions,'unread':sum(not r['range_read_addresses'] for r in rows),'sha256':H(path.read_bytes())}))
if __name__=='__main__':main()
