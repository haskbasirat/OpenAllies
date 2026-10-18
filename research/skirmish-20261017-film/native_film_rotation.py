"""Full76FE60 film rotation with REAL CString operations, CRTintegerformat,
4B2E60 typed lookup,4B29B0 permission and mutable LastFilmIndex raw store.
Native TGI/TextData/4B9F40 localization executes. Named Var storage is a seam.
"""
from pathlib import Path
import hashlib,json,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'))
from native_mapstrings import MapStrings,L
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
class Machine(MapStrings):
 def __init__(self):self.live_vars={};self.current_reads=[];super().__init__()
 def _var(self):
  name=self.text(self.r(self.arg(0))).lower()
  if name in self.live_vars:
   expected=0xb3994c if name=='lastfilmindex'else 0xb3999c
   assert self.u.reg_read(UC_X86_REG_ECX)==expected,(name,hex(self.u.reg_read(UC_X86_REG_ECX)))
   self.current_reads.append(name);self.ret(4,self.live_vars[name][0])
  else:super()._var()
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=Machine();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
 localized=case['localized'];m.load(('[Text language=Default]\n{\nids=film\nname="Film"\nlast_film_filename="'+localized+'"\n}\n').encode())
 session=m.alloc(0x200);m.w(session+0x150,m.cstring(case['old_name']))
 for name,value,flags in [('FilmRotationQueueSize',case['queue'],0),('LastFilmIndex',0,0x10)]:
  p=m.alloc(0x40);m.call(0x401650,p,[m.cstring(name),value&0xffffffff,flags]);node=m.alloc(4);m.w(node,p);m.live_vars[name.lower()]=(node,p)
 p=m.alloc(0x40);m.w(p+0x10,m.cstring(case['fallback']));m.w(p+0x24,4);node=m.alloc(4);m.w(node,p);m.live_vars['lastfilmname']=(node,p)
 last=m.live_vars['lastfilmindex'][1];m.w(last,case['last']&0xffffffff)
 m.call(0x76fe60,session,[case['rotate']]);assert m.u.reg_read(UC_X86_REG_EAX)==session+0x150
 assert all(m.u.reg_read(r)==0x5a5a0000 for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])
 assert (m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 out=dict(name_utf16=[ord(c)for c in m.text(m.r(session+0x150))],last_index=m.r(last),reads=m.current_reads)
 return out,m.windows()
def cases():
 for queue,last in [(1,-1),(5,-123),(5,0),(5,3),(5,4),(5,999),(0x7fffffff,0x7fffffff)]:
  yield dict(queue=queue,last=last,localized='LastFilm',fallback='LastFilm.RFM',old_name='previous',rotate=1)
 yield dict(queue=5,last=0,localized='',fallback='LastFilm.RFM',old_name='previous',rotate=1)
 yield dict(queue=0,last=-1,localized='LastFilm',fallback='LastFilm.RFM',old_name='unchanged',rotate=0)
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   blocks.update({r['address']:r for r in pa+pb});rows.append(dict(input=case,pc=pc,output=a))
 doc=dict(schema=1,scope=__doc__,original_sha256=L.pe.ORIGINAL_SHA,v14_sha256=L.pe.ACCEPTED_SHA,
  cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-rotation.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
