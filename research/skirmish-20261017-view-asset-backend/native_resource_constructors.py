"""Native5AA890/5A60E0 Texture and5A7DB0 Model successful constructors and9D0900→5AC5F0
class table. Real image bodies, heap/CString and Win32 atomic transports only.
No worker/decode/ready/native whole ResourceManager completion is claimed.
Resource input allocations are poisoned; Texture54/58 MUST remain unwritten. Model fields/string-pairs are initialized by actual bodies.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_FPCW
NAMES=['EnvironmentMap.tga','Textures\\UI\\Circle.TGA','Art/texture.dds','Images/Test.png']
EXTENSIONS=['TGI','TGA','DDS','PNG','KF','NIF','LST','TGR','TXT','KMP','RMP','RCG','RSG','RFM','','WAV','MP3','VSO','RTI','BIK','TTF']
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set();atomic=[]
 def inc(m,u):
  ptr=m.r(u.reg_read(UC_X86_REG_ESP)+4);value=(m.r(ptr)+1)&0xffffffff;m.w(ptr,value);atomic.append(value);m.ret(value,pop=4)
 m.stubs[0x301f750]=inc;m.w(0x9f01e8,0x301f750)
 def executed(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed)
 m.thiscall(0x4013e0,0xb43b40,[])
 globals={hex(a):m.r(a) for a in [0xb282e0,0xb282e4,0xb282e8,0xb282ec,0xb282f0,0xb3209c,0xb320a0,0xb320a4,0xb43d04]}
 rows=[]
 for kind,entry,names in [('Texture',0x5a60e0,NAMES),('Model',0x5a7db0,['test.nif','Art/Units/gbr_corps.nif','Art\\Objects\\house.nif','obj.NIF'])]:
  for poison in [0xcccccccc,0x5a96e1f0]:
   for text in names:
    name=m.alloc(4);m.w(name,m.cstring(text));resource=m.alloc(0x100);m.u.mem_write(resource,poison.to_bytes(4,'little')*64)
    out=m.thiscall(entry,resource,[name]);assert out['eax']==resource
    task=m.r(resource+0x20);assert m.r(task+0x1c)==resource
    row=dict(kind=kind,name=text,poison=poison,base={hex(i):m.r(resource+i)for i in [4,8,12,16,20,24,28,40,44,48,52]},name_retained=m.wstr(m.r(resource+0x38)),task={hex(i):m.r(task+i)for i in [4,8,12,16,20,24]},task_resource_same=True)
    if kind=='Texture':
     render=m.r(resource+0x5c);assert m.r(resource+0x54)==m.r(resource+0x58)==poison
     row.update(texture={hex(i):m.r(resource+i)for i in [60,64,68,72,76,80,84,88,96,100]},render=[m.r(render+i*4)for i in range(9)])
    else:
     slots=m.r(resource+0x80)
     row.update(model={hex(i):m.r(resource+i)for i in range(0x3c,0x80,4)},slot_strings=[m.wstr(m.r(slots)),m.wstr(m.r(slots+4))],count84=m.r(resource+0x84),capacity88=m.r(resource+0x88),name8c=m.wstr(m.r(resource+0x8c)))
    rows.append(row)
 m.cdecl(0x9d0900,[]);table=[]
 for i,expected in enumerate(EXTENSIONS):
  text=m.raw_wstr(m.r(0xb282f8+i*8));assert text==expected
  table.append(dict(extension=text,descriptor=hex(m.r(0xb282fc+i*8))))
 classes=[]
 for text in [*NAMES,'obj.nif','anim.kf','noext','foo.unknown','file.TgA']:
  name=m.alloc(4);m.w(name,m.cstring(text));descriptor=m.cdecl(0x5ac5f0,[name]);classes.append(dict(name=text,descriptor=hex(descriptor)))
 assert all(v==1 for v in atomic)
 return dict(globals=globals,rows=rows,extension_table=table,classes=classes,atomic_ref_results=atomic),pins,seams

def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,native=a));pins.update(pa);seams|=sa|sb
 doc=dict(schema=1,scope=__doc__,rows=rows,executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(seams)],external_transports=['IAT9F01E8 InterlockedIncrement exact wrapping atomic update/return','inherited heap and counted CString allocation/CRT'],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 raw=(json.dumps(doc,indent=1)+'\n').encode();p=HERE/'native-resource-constructors.json'
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),constructor_calls=2*len(rows)*len(rows[0]['native']['rows']),executed=len(pins),intercepted=len(seams),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
