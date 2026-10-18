"""Actual KingdomParameter assignment +68 raw-byte copy4A4CC6..4A4CCD.
The preceding nested member copies are outside this leaf. EDI/ESI and a valid
saved-EDI stack word are explicit interval inputs. Original/v14/all x87 PCs;
no instruction seam or PE patch. This is not a full parameter-load proof.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P

def main():
 cases=[]
 for image,pc,value in itertools.product(('original','v14'),h.PRECISION,(0,1,127,128,255)):
  h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);src=m.alloc(0x6c);dst=m.alloc(0x6c)
  m.w32(src+0x68,0x11223300|value);m.w32(dst+0x68,0xaabbcc55)
  m.w32(h.STACK,0x12345678);m.u.reg_write(UC_X86_REG_ESP,h.STACK);m.u.reg_write(UC_X86_REG_EDI,src);m.u.reg_write(UC_X86_REG_ESI,dst)
  m.u.emu_start(0x4a4cc6,0x4a4ccd,count=10)
  assert m.u.reg_read(UC_X86_REG_EIP)==0x4a4ccd
  cases.append({'image':image,'precision':pc,'value':value,'source_word':m.r32(src+0x68),'target_word':m.r32(dst+0x68)})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'seams_not_executed':[],
  'executed_instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())],'cases':cases}
 p=HERE/'native-creator-parameter-flags.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'cases':len(cases),'executed_pins':len(h.VISITED),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
