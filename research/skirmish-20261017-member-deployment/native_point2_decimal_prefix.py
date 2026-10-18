"""Whole native TGI line parser5BD080 -> point2 assignment5BB7F0,
native CRT__wtof96D3E8, then actual node GetPoint2 5B4D80/vf40.
Additional decimal dots retain numeric classification and CRT consumes the
initial decimal prefix. The ported subset is finite ASCII signed decimals
with additional dots/digits only; arbitrary suffixes/references stay required.
Line source, CString pool/heap, C-locale TLS, mutex and empty registry setup
are the inherited explicit transports; no numeric conversion/getter is mocked.
The inherited unrelated GTID-registry initializer4EFF60 is skipped; this
parser does not query it and no registry operation/readiness is claimed.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-restoration'))
from native_tgi_parser import Machine as Base,lines_of,THROW
from original_creation import P,V
from unicorn.x86_const import UC_X86_REG_FPCW,UC_X86_REG_EIP
PCS={'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}
sha=lambda p:hashlib.sha256(p.read_bytes()).hexdigest()

class Machine(Base):
 def __init__(self,src,image,pc):
  self.target_image=image;self.image_installed=False;self.excluded=set();self.node_ptrs={}
  super().__init__(lines_of(src));self.u.reg_write(UC_X86_REG_FPCW,pc)
 def call(self,pc,args):
  # RegistryMachine's first native call occurs after mapping but before any
  # parser initialization. Install the full selected image here; restore only
  # the explicit inherited fixture registry pointer written just before call.
  if not self.image_installed:
   self.image_installed=True;image=self.target_image
   old=(P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
   if new>old:self.u.mem_map(P.base+old,new-old)
   for _,r,size,off in image.sections:
    if size:self.u.mem_write(image.base+r,image.b[off:off+size])
   self.w(0xb364c0,self.manager)
  if pc==0x4eff60:return 0
  return super().call(pc,args)
 def hook(self,u,pc,size,user):
  super().hook(u,pc,size,user)
  if u.reg_read(UC_X86_REG_EIP)!=pc or pc==THROW:self.excluded.add(pc)
 def node(self,p,depth=0):
  self.node_ptrs[self.cstr(self.r(p))]=p
  return super().node(p,depth)

def cases():
 actual=ROOT.parent/'reconstruction/assets/original/Data/Buildings/USA/usa_hq_infantry.tgi'
 line=next(v.strip()for v in actual.read_text().splitlines()if'creation_offset'in v)
 value=line.split('=',1)[1].strip();assert value=='2.5, 1.5.0'
 out=[dict(name='actual_usa_hq_infantry',text=value,supported=True)]
 for i,(x,y)in enumerate([
  ('1.5.0','2.5'),('-1.5.0','+2.25.7'),('1..2','3...4'),('.5.7','-.5.9'),
  ('1.5.','2.'),('0.0000000000001.4','16777217.0.2'),('0.1.2.3','-0.000.9'),
  ('+16777219.0.1','-.125.7'),('0001.5.000','0.0.0'),('1.000000059604644775390625.0','2.00000011920928955078125.4'),
  ('0','-0'),('1.25','-.75'),
 ]):out.append(dict(name=f'decimal_{i}',text=f'{x}, {y}',supported=True))
 for i,text in enumerate(['..5,1','1.5x,2','1e3,2','foo,1','1 + 2,3','1.5.0%,2','1.5.0f,2','1,2,3']):out.append(dict(name=f'outside_subset_{i}',text=text,supported=False))
 return out,actual,line

def run(c,image,pc):
 src=('[A]\n{\npoint2 creation_offset = '+c['text']+'\n}\n').encode('ascii')
 m=Machine(src,image,pc);parsed=m.parse();out={'error':parsed['error'],'point':None}
 if not parsed['error']:
  node=m.node_ptrs['A'];pointer=m.alloc(8)
  m.invoke(0x5b4d80,node,[pointer,m.string(list(map(ord,'creation_offset')))])
  out['point']=[m.r(pointer),m.r(pointer+4)]
  # The getter must leave the actual source point2 field allocation/type.
  matches=[]
  for b in range(m.r(node+0x44)):
   entry=m.r(m.r(node+0x40)+4*b)
   while entry:
    field=m.r(entry)
    if m.text(m.r(field+4))=='creation_offset':matches.append(m.r(field))
    entry=m.r(entry+4)
  assert matches==[0xa48610]
 out['control_word']=m.u.reg_read(UC_X86_REG_FPCW)
 pins={a:bytes(m.u.mem_read(a,n))for a,n in m.visited.items()if P.base<=a<P.base+P.image_size and a not in m.excluded}
 return out,pins

def main():
 cases_,actual,line=cases();rows=[];pins={}
 for precision,cw in PCS.items():
  for c in cases_:
   a,pa=run(c,P,cw);b,pb=run(c,V,cw);assert a==b,(c,a,b)
   rows.append(dict(precision=precision,input=c,output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert raw==P.range(va,len(raw))==V.range(va,len(raw)),hex(va)
 doc={'schema':1,'scope':__doc__,'original_sha256':hashlib.sha256(P.b).hexdigest(),'v14_sha256':hashlib.sha256(V.b).hexdigest(),'producer_sha256':sha(Path(__file__)),
  'basis_sha256':sha(ROOT/'research/phase8-restoration/native_tgi_parser.py'),'actual_source_sha256':sha(actual),'actual_source_line':line,
  'rows':rows,'instructions':[dict(va=hex(va),bytes=raw.hex())for va,raw in sorted(pins.items())]}
 path=HERE/'native-point2-decimal-prefix.json'
 if '--write'in sys.argv:path.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=sha(path))))
if __name__=='__main__':main()
