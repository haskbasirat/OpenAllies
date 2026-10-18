"""Real85A550/859B70/85ABB0 path helpers and5A6FD0 model filename rewrite.
73A560 and40F7B0 ordinal50 are explicit current-renderer/currentCString
inputs at actual boundaries; false MUST NOT read suffix;
no observed hardware capability/default value is claimed by this fixture.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPCW,UC_X86_REG_ESP
NAMES=['EnvironmentMap.tga','Art/Units/gbr_corps.nif','Art\\Units\\gbr_corps.nif','C:model.nif','C:/model.nif','dir.name/model','foo.nif','noext','.nif','folder/']
SUBFOLDERS=['','winter','winter/','/winter','//winter//']
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set();predicate=[];current=[False];suffix=["-h"];suffix_reads=[]
 def hardware(m,u):predicate.append(current[0]);m.ret(int(current[0]))
 def current_suffix(m,u):
  esp=u.reg_read(UC_X86_REG_ESP);out=m.r(esp+4);index=m.r(esp+8);assert index==50
  suffix_reads.append(suffix[0]);m.w(out,m.cstring(suffix[0]));m.ret(out,pop=8)
 m.stubs[0x73a560]=hardware;m.stubs[0x40f7b0]=current_suffix
 def executed(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed);m.thiscall(0x4013e0,0xb43b40,[])
 def box(text):p=m.alloc(4);m.w(p,m.cstring(text));return p
 def call(at,*args):out=m.alloc(4);m.cdecl(at,[out,*args]);return out
 paths=[];models=[]
 for text in NAMES:
  name=box(text)
  for folder in SUBFOLDERS:
   directory=call(0x85a550,name);part=call(0x85abb0,directory,box(folder));base=call(0x859b70,name);out=call(0x85abb0,part,base)
   paths.append(dict(name=text,subfolder=folder,candidate=m.wstr(m.r(out))))
  for enabled,value in [(False,"unread"),(True,"-h"),(True,""),(True,"-custom")]:
   current[0]=enabled;suffix[0]=value;before=len(predicate);reads=len(suffix_reads);out=call(0x5a6fd0,name);assert len(predicate)==before+1
   assert len(suffix_reads)==reads+int(enabled)
   models.append(dict(name=text,hardware=enabled,suffix=value if enabled else None,result=m.wstr(m.r(out))))
 return dict(paths=paths,models=models,predicate_calls=len(predicate),suffix_reads=len(suffix_reads)),pins,seams

def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,native=a));pins.update(pa);seams|=sa|sb
 doc=dict(schema=1,scope=__doc__,rows=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 p=HERE/'native-resource-paths.json';raw=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),path_rows=2*len(rows)*len(rows[0]['native']['paths']),model_rows=2*len(rows)*len(rows[0]['native']['models']),executed=len(pins),intercepted=len(seams),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
