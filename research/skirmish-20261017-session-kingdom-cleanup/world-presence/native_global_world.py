"""Reached original World publication and subsequent Sim alias instruction slices.

The actual4F7D00 prefix publishes B36534 before any child call. The separate
76AB1C store follows successful World construction in76AA90. The interval
between them has positive globalWorld and untouched constructor-null Sim3C.
This is NOT execution of the full World constructor or a destructor/global
absence proof. Explicit input allocation and OS/SEH transport are laboratory.
"""
from pathlib import Path
import sys,json,hashlib
HERE=Path(__file__).resolve().parent
ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/phase3-orders'))
from original_orders import Machine,PE0,PE1
from unicorn.x86_const import *

def main():
 rows=[];windows=[]
 intervals=[(0x4f7d00,0x35),(0x76ab1c,3)]
 for start,size in intervals:
  a=PE0.range(start,size);b=PE1.range(start,size);assert a==b
  windows.append(dict(va=hex(start),bytes=a.hex()))
 for image,pe in [('original',PE0),('v14',PE1)]:
  m=Machine()
  for start,size in intervals:m.u.mem_write(start,pe.range(start,size))
  for precision in [24,53,64]:
   for old in [False,True]:
    world=m.alloc(0x300);sim=m.alloc(0x80);other=m.alloc(0x300)
    m.w(0xb36534,other if old else 0);m.w(sim+0x3c,0)
    m.u.reg_write(UC_X86_REG_ESP,m.stack);m.u.reg_write(UC_X86_REG_ECX,world)
    m.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[precision])
    m.u.emu_start(0x4f7d00,0x4f7d35,count=100)
    partial=dict(global_is_new=m.r(0xb36534)==world,sim_is_null=m.r(sim+0x3c)==0,
      singleton_backlink=m.r(0xb3cc30)==0xb36534,constructor_this=m.r(0xb3cc34)==world)
    assert all(partial.values())
    m.u.reg_write(UC_X86_REG_ESI,sim);m.u.reg_write(UC_X86_REG_EAX,world)
    m.u.emu_start(0x76ab1c,0x76ab1f,count=2)
    final=dict(global_is_new=m.r(0xb36534)==world,sim_is_new=m.r(sim+0x3c)==world)
    assert all(final.values())
    rows.append(dict(image=image,precision=precision,old_global_present=old,partial=partial,after_constructor_return_store=final))
 doc=dict(scope=__doc__,producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  original_sha256=hashlib.sha256(PE0.b).hexdigest(),v14_sha256=hashlib.sha256(PE1.b).hexdigest(),rows=rows,windows=windows)
 path=HERE/'native-global-world.json'
 if '--write' in sys.argv:
  with path.open('x')as f:f.write(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),instruction_slices=2,sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
