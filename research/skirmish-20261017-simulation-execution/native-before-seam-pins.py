"""Original/v14 sync-thread branch and explicit thread-ID publication slices.
Windows GetCurrentThreadId is a scripted OS seam; actual retained OS-thread
creation/identity is tested separately in Rust. No claim to execute the full
Application Init or Thread recurring simulation loop. Native logger body runs.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-net'))
from native_sync import SyncMachine,PE0,PE1,SYNC,STUB
from unicorn.x86_const import *
from capstone import Cs,CS_ARCH_X86,CS_MODE_32

def main():
 rows=[];published=[];pins={}
 for image,pe in [('original',PE0),('v14',PE1)]:
  m=SyncMachine()
  mapped=set()
  for begin,end,_ in m.u.mem_regions():mapped.update(range(begin&~4095,(end+4096)&~4095,4096))
  for vs,r,size,off in pe.sections:
   for page in range((pe.base+r)&~4095,(pe.base+r+size+4095)&~4095,4096):
    if page not in mapped:m.u.mem_map(page,4096);mapped.add(page)
   m.u.mem_write(pe.base+r,pe.b[off:off+size])
  m.w(0x9f01e4,STUB);m.w(0xb39828,0x1234)
  for precision,tid,flags in itertools.product([24,53,64],[0x1234,0x5678],[None,[0,0],[1,0],[0,128],[255,255]]):
   m.u.reg_write(UC_X86_REG_FPCW,{24:0x7f,53:0x27f,64:0x37f}[precision]);m.thread=tid
   sai=m.alloc(0x64)if flags is not None else 0;m.w(0xb36528,sai)
   if sai:m.u.mem_write(sai+0x5c,bytes(flags))
   m.call(0x77ccb0,[32],this=SYNC,callee_pop=4);m.u.mem_write(SYNC,b'\x01')
   label=m.string('thread-gate');before=m.state()
   m.call(0x77da70,[label,17,0],this=SYNC,callee_pop=12);after=m.state()
   expected=tid==0x1234 and(flags is None or flags==[0,0])
   assert after['hash']==( -20 if expected else -1)
   rows.append(dict(image=image,precision=precision,current=tid,simulation=0x1234,sai_flags=flags,gate=expected,before=before,after=after))
  #Actual caller role: LEA EBX,[ESI+150] makes B397E0 on ApplicationB39690.
  m.u.reg_write(UC_X86_REG_ESI,0xb39690);m.u.reg_write(UC_X86_REG_ESP,m.stack)
  m.u.emu_start(0x41abcf,0x41abd5,count=10);base=m.u.reg_read(UC_X86_REG_EBX)
  assert base==0xb397e0 and base+0x48==0xb39828
  for old,os_output in itertools.product([0xa5,0xff],[0,0x1234,0xffffffff]):
   m.u.mem_write(base,bytes([old])*0x70);m.w(m.stack+0xc,os_output)
   parent=m.alloc(4);m.w(base+0x4c,parent)
   m.u.reg_write(UC_X86_REG_ESI,base);m.u.reg_write(UC_X86_REG_ESP,m.stack)
   m.u.emu_start(0x85e4a6,0x85e4b7,count=10)
   assert m.r(base+0x48)==os_output and m.r(base+0x64)==2
   published.append(dict(image=image,prior_byte=old,actual_beginthreadex_output=os_output,base=base,published_id=m.r(base+0x48),state=m.r(base+0x64)))
  pins.update(m.visited)
 d=Cs(CS_ARCH_X86,CS_MODE_32)
 for start,end in [(0x41abc5,0x41abdc+5),(0x85e430,0x85e4d8)]:
  for ins in d.disasm(PE0.range(start,end-start),start):pins[ins.address]=ins.size
 instructions=[]
 for va,size in sorted(pins.items()):
  a=PE0.range(va,size);b=PE1.range(va,size);assert a==b
  instructions.append(dict(va=f'{va:08X}',bytes=a.hex()))
 result=dict(passed=True,scope=__doc__,rows=rows,publication_slices=published,
  original_sha256=hashlib.sha256(PE0.b).hexdigest(),v14_sha256=hashlib.sha256(PE1.b).hexdigest(),
  producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),instructions=instructions)
 (HERE/'native-gate.json').write_text(json.dumps(result,indent=2)+'\n');print(len(rows),'logger rows;',len(published),'publication slices')
if __name__=='__main__':main()
