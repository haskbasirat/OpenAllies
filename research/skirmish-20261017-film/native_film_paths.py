"""Full85AB30/85ABB0/85A1A0 counted UTF16 path helpers. Real CString code;
only existing loader allocator/string-manager/OS/log seams. No path IO claim.
"""
from pathlib import Path
import hashlib,json,sys,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-triggers'))
import loader as L
from unicorn.x86_const import *
ORIGINAL=L.IMAGE;EXPANDED=L.ACCEPTED
PC={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
def units(text):return [ord(c)for c in text]
def string(m,words):
 d=m._data(len(words));m.u.mem_write(d+16,struct.pack('<'+'H'*len(words),*words)+b'\0\0');m.w(d+4,len(words));m.w(d+12,0x10000);p=m.alloc(4);m.w(p,d+16);return p
def read(m,p):
 p=m.r(p);n=m.r(p-12);return list(struct.unpack('<'+'H'*n,bytes(m.u.mem_read(p,n*2))))if n else[]
def run(case,image,pc):
 L.IMAGE=ORIGINAL if image=='original'else EXPANDED;m=L.Loader();m.u.reg_write(UC_X86_REG_FPCW,PC[pc])
 left=string(m,case['left']);right=string(m,case['right']);output=m.alloc(4)
 if case['operation']=='extension':m.call(0x85ab30,0,[left,right]);result=read(m,left)
 else:
  m.call(0x85abb0,0,[output,left,right]);assert m.u.reg_read(UC_X86_REG_EAX)==output;result=read(m,output)
 assert read(m,right)==case['right']
 if case['operation']=='join':assert read(m,left)==case['left']
 assert all(m.u.reg_read(r)==0x5a5a0000 for r in [UC_X86_REG_EBX,UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP])
 assert(m.u.reg_read(UC_X86_REG_FPSW)>>11)&7==0
 return result,m.windows()
def cases():
 for left,right in [('', ''),('LastFilm_0','RFM'),('LastFilm.RFM_0','RFM'),('file',''),('','RFM'),('file','.RFM')]:
  yield dict(operation='extension',left=units(left),right=units(right))
 for left in ['', 'Films','Films/','Films\\','Films//','C:\\Films\\']:
  for right in ['', 'LastFilm_0.RFM','/LastFilm_0.RFM','\\LastFilm_0.RFM','//nested']:
   yield dict(operation='join',left=units(left),right=units(right))
 for operation in ['extension','join']:
  yield dict(operation=operation,left=[0,0xd800,47,0xffff],right=[0,0xdc00,92])
  yield dict(operation=operation,left=[47,47],right=[47,0,0xdfff])
def main():
 rows=[];blocks={}
 for case in cases():
  for pc in PC:
   a,pa=run(case,'original',pc);b,pb=run(case,'v14',pc);assert a==b
   blocks.update({r['address']:r for r in pa+pb});rows.append(dict(input=case,pc=pc,output=a))
 doc=dict(schema=1,scope=__doc__,original_sha256=L.pe.ORIGINAL_SHA,v14_sha256=L.pe.ACCEPTED_SHA,cases=rows,blocks=[blocks[k]for k in sorted(blocks)])
 data=(json.dumps(doc,indent=1)+'\n').encode();path=HERE/'native-film-paths.json'
 if '--write'in sys.argv:path.write_bytes(data)
 else:assert path.read_bytes()==data
 print(json.dumps(dict(rows=len(rows),executions=2*len(rows),blocks=len(blocks),sha256=hashlib.sha256(data).hexdigest())))
if __name__=='__main__':main()
