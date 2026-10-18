"""Actual856DC0 backing flush dispatch on8764F0's real file vtable, then
real876600 buffer drain and8766D0 FlushFileBuffers. The fixture has no pending
native user-buffer bytes; actual File transport writes are independently tested.
OS FlushFileBuffers is an explicit transport; FALSE stops before the required
native localized error/exception graph, not a successful flush.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stopped(Exception):pass
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc]);events=[]
 child=m.alloc(0x34);assert m.call(0x8764f0,child)==child
 assert m.r(child)==0xab7980 and m.r(0xab7980+0x44)==0x8766d0
 wrapper=m.alloc(0x40);m.w(wrapper,0xa440e0);m.w(wrapper+4,child)
 assert m.r(0xa440e0+0x3c)==0x856dc0
 m.w(child+0xc,0xffffffff if case['closed'] else 0x11223344)
 def flush():events.append('FlushFileBuffers');assert m.arg(0)==0x11223344;m.ret(4,int(case['success']))
 stub=L.STUBS+0xc00;m.seam(stub,flush);m.w(0x9f0180,stub)
 def before(u,at,size,_):
  if at==0x876600:events.append('drain')
  elif at==0x8766ee:raise Stopped('required_native_flush_error')
 m.u.hook_add(UC_HOOK_CODE,before)
 stopped=None
 try:m.call(0x856dc0,wrapper)
 except Stopped as e:stopped=str(e)
 return dict(events=events,stopped=stopped,handle=m.r(child+0xc)),m.windows()
def main():
 rows=[];blocks={}
 for closed,success in [(False,True),(False,False),(True,True)]:
  case=dict(closed=closed,success=success)
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 raw=(json.dumps(dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)]),indent=1)+'\n').encode()
 path=HERE/'native-film-file-flush.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
