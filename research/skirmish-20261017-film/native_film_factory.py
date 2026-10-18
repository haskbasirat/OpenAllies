"""Full5A31D0->5A27A0 wrapper order with explicit Resource-base/chunk-base,
Creator-vf0C clone, fresh Header constructor, and full5A26E0 child seams.
These seams are required services, not successful native implementations of
the children. Actual derived stores, Creator+88/Header+90 publication and final
Rules.mode3 execute in both images. No depot/open/wholeWorld capture claim.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'));import loader as L
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Stopped(Exception):pass
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader(PC[pc])
 ids=m.alloc(4);m.w(ids,m.cstring('Films/LastFilm_0.RFM'))
 source=m.alloc(0x70);vt=m.alloc(0x20);m.w(source,vt)
 clone=m.alloc(0x70);world=m.alloc(0x29c);m.w(0xb36534,world)
 resource=m.alloc(0xb4);rules=m.alloc(0x28);m.w(rules+4,case['before'])
 events=[];header=None
 def event(name):
  if len(events)==case.get('stop',-1):raise Stopped(name)
  events.append(name)
 def base():
  assert m.u.reg_read(UC_X86_REG_ECX)==resource and m.arg(0)==ids
  m.w(resource+8,0x10);m.w(resource+0x38,m.r(ids));m.ret(4,resource)
 def chunk():assert m.u.reg_read(UC_X86_REG_ECX)==resource+0x3c;m.ret(0,resource+0x3c)
 def cloned():
  event('clone');assert m.u.reg_read(UC_X86_REG_ECX)==source and m.arg(0)==0;m.ret(4,clone)
 def constructed():
  nonlocal header
  event('header');assert m.arg(0)==0
  header=m.u.reg_read(UC_X86_REG_ECX);m.w(header+4,rules);m.ret(4,header)
 def captured():
  event('capture');assert m.u.reg_read(UC_X86_REG_ECX)==header and m.arg(0)==world
  m.w(rules+4,case['captured']);m.ret(4)
 m.seam(0x5aa890,base);m.seam(0x8584a0,chunk)
 stub=L.STUBS+0xc00;m.seam(stub,cloned);m.w(vt+0xc,stub)
 m.seam(0x5a23f0,constructed);m.seam(0x5a26e0,captured)
 stopped=None
 try:assert m.call(0x5a31d0,resource,[ids,source])==resource
 except Stopped as e:stopped=str(e)
 out=dict(events=events,stopped=stopped,creator_installed=m.r(resource+0x88)==clone,
          header_installed=header is not None and m.r(resource+0x90)==header,mode=m.r(rules+4),
          flags=m.r(resource+8),mode_a8=m.r(resource+0xa8),references_empty=[m.r(resource+0x7c),m.r(resource+0x80)])
 if stopped is None:
  assert out['mode']==3 and (m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
  assert m.r(resource)==0xa44130 and m.r(resource+0x3c)==0xa440e0
 return out,m.windows()
def cases():
 for before,captured in [(7,0),(0xffffffff,1),(3,0x89abcdef)]:
  case=dict(before=before,captured=captured);yield case
  for stop in range(3):yield dict(case,stop=stop)
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   rows.append(dict(input=case,pc=pc,output=a));blocks.update({r['address']:r for r in pa+pb})
 doc=dict(schema=1,scope=__doc__,cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 raw=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-factory.json'
 if '--write'in sys.argv:path.write_bytes(raw)
 else:assert path.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
