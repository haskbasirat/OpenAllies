"""Actual4017D0 Bool(type1,flags10) construction and4B3190/4B29B0 byte
setter. Current source assignment and portable embedded-only assignment use
the SAME byte; native default byte1 is not overwritten by either setter.
No whole variable registry or TGI source-loader claim.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);var=m.alloc(0x30)
 m.call(0x4017d0,var,[m.cstring('EditorMapStringsSeparate'),0,0x10])
 assert m.r(var+0x24)==1 and m.r(var+0x28)==0x10
 m.call(0x4b3190,var,[case['before']]);m.call(0x4b3190,var,[case['requested']])
 assert all(m.u.reg_read(r)==0x5a5a0000 for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])
 return dict(current=m.r(var)&255,default=(m.r(var)>>8)&255,type=m.r(var+0x24),flags=m.r(var+0x28)),m.windows()
def main():
 rows=[];blocks={}
 for before in [0,1,128,255]:
  for requested in [0,1,128,255]:
   c=dict(before=before,requested=requested)
   for pc in PC:
    a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
    rows.append(dict(input=c,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 raw=(json.dumps(dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)]),indent=1)+'\n').encode()
 path=HERE/'native-film-separate-variable.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
