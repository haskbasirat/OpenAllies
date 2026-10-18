"""Native interactive Application+30 kind stores before later local Host.
These are explicit reached native instruction slices, not full App Init.
4196EC stores1;41A4EA stores2 before41ABCF creates simulation thread.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase3-orders'))
from original_orders import Machine,PE0,PE1
from unicorn.x86_const import *

def main():
 rows=[]
 for image,pe in [('original',PE0),('v14',PE1)]:
  m=Machine()
  #Only these identical source instruction windows execute. No SDK code.
  for pc in [24,53,64]:
   for initial in [0,3,0xa5a5a5a5,0xffffffff]:
    app=m.alloc(0x40);m.w(app+0x30,initial)
    m.u.reg_write(UC_X86_REG_ESI,app);m.u.reg_write(UC_X86_REG_ESP,m.stack)
    m.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[pc])
    m.u.emu_start(0x4196ec,0x4196f3,count=3);first=m.r(app+0x30)
    m.u.emu_start(0x41a4ea,0x41a4f1,count=3);second=m.r(app+0x30)
    assert(first,second)==(1,2)
    rows.append(dict(image=image,precision=pc,initial=initial,after_initial_store=first,after_post_parse_store=second))
 windows=[]
 for start,size in [(0x4196ec,7),(0x41a4ea,7),(0x41abcf,0x12)]:
  a=PE0.range(start,size);b=PE1.range(start,size);assert a==b
  windows.append(dict(va=f'{start:08X}',bytes=a.hex()))
 out=dict(passed=True,scope=__doc__,rows=rows,windows=windows,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),original_sha256=hashlib.sha256(PE0.b).hexdigest(),v14_sha256=hashlib.sha256(PE1.b).hexdigest())
 (HERE/'native-application-kind.json').write_text(json.dumps(out,indent=2)+'\n');print(len(rows),'kind producer rows')
if __name__=='__main__':main()
