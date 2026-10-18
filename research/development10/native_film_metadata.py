"""Native application metadata constants,410D80 numeric fold and5A14C0
metadata prefix. CString transport, atexit and four already parsed CRT components
are named seams; general swscanf parsing and complete Header creation are not
claimed. Header prefix stops at the real first6FB1F0 clock call.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent/'skirmish-20261016-fx-graph'))
import native_fx_graph as g
nm=g.nm;O=nm.original;V=nm.modified
class PrefixStop(Exception):pass

def zero_tail(image):
 rva=0xb3d8cc-image.base
 for virtual,address,raw,offset in image.sections:
  if address<=rva<address+max(virtual,raw):
   assert raw<=rva-address and rva-address+4<=virtual
   return dict(va='0xb3d8cc',virtual_size=virtual,rva=address,raw_size=raw,
    raw_offset=offset,word_offset=rva-address,initialization='PE loader zero-filled section tail')
 raise AssertionError('Header count is not in image')

def machine(pc,image):
 m=nm.Machine(pc)
 if image=='v14':
  old=(O.image_size+4095)&~4095;new=(V.image_size+4095)&~4095
  if new>old:m.u.mem_map(O.base+old,new-old)
  for vs,rva,raw,off in V.sections:m.u.mem_write(V.base+rva,V.b[off:off+raw])
 return m

def utf(m,p):
 out=[]
 while True:
  b=bytes(m.u.mem_read(p,2));p+=2
  if b==b'\0\0':return b''.join(out).decode('utf-16-le')
  out.append(b)

def cstring(m,text):
 p=m.alloc(16+2*(len(text)+1));m.w(p+12,1000000)
 m.u.mem_write(p+16,(text+'\0').encode('utf-16-le'))
 return p+16

def run(c,image,pc):
 m=machine(pc,image);events=[]
 if c['kind']=='static':
  def ctor(mm):
   value=utf(m,mm.arg(1));events.append(['CString',hex(mm.ecx()),value]);m.w(mm.ecx(),cstring(m,value));mm.ret(mm.ecx(),4)
  m.seam(0x401bb0,ctor);m.seam(0x96cab0,lambda mm:mm.ret(0))
  for fn in [0x9c7a60,0x9c7ac0,0x9c7b00]:m.call(fn,0)
  out={'version':utf(m,m.r(0xb3888c)),'suffix':utf(m,m.r(0xb38898)),'application':utf(m,m.r(0xb388a0)),'build_bits':m.r(0xb27a80),'events':events}
 elif c['kind']=='version':
  text='.'.join(map(str,c['components']));value=cstring(m,text)
  def scan(mm):
   assert utf(m,mm.arg(1))==text and utf(m,mm.arg(2))=='%d.%d.%d.%d'
   for i,v in enumerate(c['components']):m.w(mm.arg(3+i),v)
   mm.ret(4)
  m.seam(0x96c99a,scan)
  m.call(0x410d80,0,[value],pop=0);out={'numeric':m.u.reg_read(nm.UC_X86_REG_EAX)}
 else:
  target=m.alloc(0x68);m.u.mem_write(target,b'\xcd'*0x68)
  for address,name in [(0xb3888c,'version'),(0xb388a0,'application'),(0xb38894,'release')]:m.w(address,cstring(m,c[name]))
  m.w(0xb3650c,c['numeric']);m.w(0xb3d8cc,c['count'])
  def copy(mm):m.w(mm.ecx(),m.r(mm.arg(1)));mm.ret(mm.ecx(),4)
  def stop(mm):raise PrefixStop()
  m.seam(0x405b80,copy);m.seam(0x6fb1f0,stop)
  try:m.call(0x5a14c0,target);raise AssertionError('prefix stop not reached')
  except PrefixStop:pass
  out={'words':{str(off):m.r(target+off) for off in [0xc,0x10,0x14,0x40,0x50,0x54]},
       'version':utf(m,m.r(target+0x58)),'application':utf(m,m.r(target+0x5c)),
       'release':utf(m,m.r(target+0x60)),'count':m.r(0xb3d8cc)}
 return out,m.visited

def main():
 cases=[{'kind':'static'}]
 for values in [[1,0,5,0],[0,0,0,0],[99,99,99,99],[2,13,7,42],[0,0,1,0],[10,1,2,3]]:cases.append({'kind':'version','components':values})
 for count in [0,1,0xffffffff]:
  cases.append(dict(kind='header_prefix',version='1.0.5.0',application='A&A',release='English AA',numeric=1000500,count=count))
 rows=[];pins={}
 for pc in nm.PRECISIONS:
  for c in cases:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b
   pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for va,raw in pins.items():assert O.range(va,len(raw))==V.range(va,len(raw))==raw
 assert O.range(0xb27a80,4)==V.range(0xb27a80,4)==bytes.fromhex('9a0d8145')
 assert zero_tail(O)==zero_tail(V)
 doc=dict(schema=1,scope=__doc__,rows=rows,executions=len(rows)*2,
  instructions=[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())],
  build_data=dict(va='0xb27a80',bytes='9a0d8145'),header_count_zero_tail=zero_tail(O),
  original_sha256=nm.sha(O.b),v14_sha256=nm.sha(V.b))
 path=HERE/'native-film-metadata.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=2)+'\n',encoding='utf-8')
 else:assert json.loads(path.read_text(encoding='utf-8'))==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(path.read_bytes()).hexdigest())))
if __name__=='__main__':main()
