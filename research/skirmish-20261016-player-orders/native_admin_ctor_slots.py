"""Bounded75CE80 Node constructor slot writes, not complete construction.
Execute each actual +268/+280 MOV with EBX explicitly zero as established by
that constructor, preserving poison elsewhere. No SDK/default-field inference.
"""
from pathlib import Path
import sys,json
HERE=Path(__file__).resolve().parent;sys.path.insert(0,str(HERE));import native_admin_ordering_owner as n
h=n.h;P=n.P;V=n.V
from unicorn.x86_const import *
def main():
 rows=[]
 for image,label in [(P,'original'),(V,'v14')]:
  h.P=image
  for pc in h.PRECISION:
   for entry,offset in [(0x75d074,0x268),(0x75d0c9,0x280)]:
    m=h.Machine(pc);a=m.alloc(0x400);m.u.mem_write(a,b'\xcd'*0x400)
    m.u.reg_write(UC_X86_REG_ESI,a);m.u.reg_write(UC_X86_REG_EBX,0);m.u.reg_write(UC_X86_REG_ESP,h.STACK)
    m.u.emu_start(entry,entry+6,count=100)
    assert m.u.reg_read(UC_X86_REG_EIP)==entry+6 and m.esp()==h.STACK and m.fpu_top()==0
    assert m.u.reg_read(UC_X86_REG_ESI)==a and m.u.reg_read(UC_X86_REG_EBX)==0
    assert m.r32(a+offset)==0 and m.r32(a+offset-4)==0xcdcdcdcd and m.r32(a+offset+4)==0xcdcdcdcd
    rows.append({'image':label,'precision':pc,'slot':hex(offset),'word':m.r32(a+offset)})
 h.P=P
 for va,raw in h.VISITED.items():assert bytes.fromhex(raw)==P.range(va,len(bytes.fromhex(raw)))==V.range(va,len(bytes.fromhex(raw)))
 out={'scope':__doc__,'producer_sha256':n.H(Path(__file__).read_bytes()),'rows':rows,'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())]}
 p=HERE/'native-admin-ctor-slots.json';p.open('x').write(json.dumps(out,indent=1)+'\n');print(json.dumps({'rows':len(rows),'pins':len(h.VISITED),'sha256':n.H(p.read_bytes())}))
if __name__=='__main__':main()
