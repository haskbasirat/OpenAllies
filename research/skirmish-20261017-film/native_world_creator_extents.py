"""Isolated4F28A6..4F28B1 raw Creator+3C/+40 -> actual World+3C/+40
copies, with explicit live source/destination storage and a seeded x87 stack
entry consumed by the intervening FSTP. No full WorldInit/caller or terrain
geometry claim. Every pre-instruction cutoff preserves actual native stores.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
ADDRESSES=[0x4f28a6,0x4f28a9,0x4f28ac,0x4f28af,0x4f28b1]
class Stopped(Exception):pass
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc])
 creator=m.alloc(0x50);world=m.alloc(0x50)
 for i,value in enumerate(case['creator']):m.w(creator+0x3c+4*i,value)
 for i,value in enumerate(case['world_before']):m.w(world+0x3c+4*i,value)
 # FLD1 is an explicit transport of the prior GameSpeed calculation's live
 # register. It is not part of the native copy proof or a numeric default.
 stub=L.STUBS+0x500;m.u.mem_write(stub,b'\xd9\xe8');m.u.emu_start(stub,stub+2)
 assert (m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==7
 m.u.reg_write(UC_X86_REG_ESI,world);m.u.reg_write(UC_X86_REG_EDI,creator)
 pins=[];step=0
 def observe(u,at,size,_):
  nonlocal step
  if at not in ADDRESSES:return
  if step==case.get('stop',-1):raise Stopped()
  pins.append(dict(address=hex(at),bytes=bytes(u.mem_read(at,size)).hex()))
  step+=1
 m.u.hook_add(UC_HOOK_CODE,observe)
 try:m.u.emu_start(ADDRESSES[0],0x4f28b4)
 except Stopped:pass
 assert [m.r(creator+0x3c+4*i)for i in range(2)]==case['creator']
 return dict(world=[m.r(world+0x3c+4*i)for i in range(2)],executed=step,
             fpu_top=(m.u.reg_read(UC_X86_REG_FPSW)>>11)&7),pins
def cases():
 pairs=[[0,0],[0x80000000,0],[0x7f812345,0xff812346],[0x7fc12345,0xffc54321],
        [0x3f000001,0x3fc00001],[0x7f800000,0xff800000],[1,0x80000001],
        [0xffffffff,0x12345678],[0x44800000,0x44400000]]
 for pair in pairs:
  base=dict(creator=pair,world_before=[0xaabbccdd,0xdeadbeef])
  yield base
  for stop in range(5):yield dict(base,stop=stop)
def main():
 rows=[];pins={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b and pa==pb
   rows.append(dict(input=case,pc=pc,output=a))
   for p in pa:pins[p['address']]=p
 doc=dict(schema=1,scope=__doc__,cases=rows,instructions=[pins[k]for k in sorted(pins)])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-world-creator-extents.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),instructions=len(pins),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
