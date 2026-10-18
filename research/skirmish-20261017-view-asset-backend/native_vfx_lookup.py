"""Real4BD9C0 Manager +4BCA00 Thing/VFX constructors +427BD0/4BDB70.
No DataLoadData/payload/preload/request completion is claimed. Constructor
category kind/index are explicit canonical descriptor inputs. Host heap,
CString/thread temporary and mutex transports inherited from native_machine.
"""
from pathlib import Path
import sys,json,hashlib,gc
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-definitions'));import native_definition_registry as h
sys.path.insert(0,str(ROOT/'research/skirmish-20261017-svar-field-order'));import native_full_sources as full
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_FPCW
KINDS=[0,*range(5,18),22,23]
def run(image,pc):
 full.order.T.P=image;m=h.Machine();m.u.reg_write(UC_X86_REG_FPCW,full.order.PCS[pc]);pins={};seams=set()
 def executed(u,at,size,_):
  if image.base<=at<image.base+image.image_size:
   if u.reg_read(UC_X86_REG_EIP)==at:pins[at]=bytes(u.mem_read(at,size)).hex()
   else:seams.add(at)
 m.u.hook_add(UC_HOOK_CODE,executed)
 m.thiscall(0x4013e0,0xb43b40,[]);mgr=h.make_thing_manager(m);objects={};types=[]
 for index,kind in enumerate(KINDS):
  category=m.alloc(0x40);m.w(category+0x18,kind);m.w(category+0xc,index)
  ids='kind'+str(kind);r=m.thiscall(0x4bca00,mgr,[m.cstring(ids),category]);assert not r.get('thrown'),r
  obj=r['eax'];objects[ids]=obj;assert m.r(obj+0x18)==category
  fn=m.r(m.r(obj)+0x48);value=m.thiscall(fn,obj,[])['eax'];assert value==int(kind!=0)
  types.append(dict(kind=kind,vtable=hex(m.r(obj)),virtual48=hex(fn),value=value,descriptor_kind=m.r(m.r(obj+0x18)+0x18)))
 for old,new in [('old','kind12'),('kind12','kind0'),('missing_alias','missing')]:h.insert_translation(m,mgr,old,new)
 queries=[*objects,'old','OLD','kind12','missing_alias','missing',''];results=[]
 reverse={ptr:ids for ids,ptr in objects.items()}
 for q in queries:
  raw=m.thiscall(0x427bd0,mgr,[m.cstring(q)])['eax'];found=m.thiscall(0x4bdb70,mgr,[m.cstring(q)])['eax']
  results.append(dict(query=q,canonical=reverse.get(raw),vfx=reverse.get(found),kind=m.r(m.r(found+0x18)+0x18)if found else None))
 assert next(r for r in results if r['query']=='old')['vfx']=='kind12'
 assert next(r for r in results if r['query']=='kind12')['vfx']is None
 return dict(types=types,queries=results),pins,seams

def main():
 rows=[];pins={};seams=set()
 for pc in full.order.PCS:
  a,pa,sa=run(h.P,pc);gc.collect();b,pb,sb=run(h.V,pc);gc.collect();assert a==b and pa==pb
  rows.append(dict(precision=pc,native=a));pins.update(pa);seams|=sa|sb
 for at,raw in pins.items():assert h.P.range(at,len(bytes.fromhex(raw)))==h.V.range(at,len(bytes.fromhex(raw)))==bytes.fromhex(raw)
 doc=dict(schema=1,scope=__doc__,rows=rows,constructors_per_machine=len(KINDS),executed={hex(a):b for a,b in sorted(pins.items())},intercepted=[hex(a)for a in sorted(seams)],producer_sha256=hashlib.sha256(Path(__file__).read_bytes()).hexdigest())
 p=HERE/'native-vfx-lookup.json';raw=(json.dumps(doc,indent=1)+'\n').encode()
 if '--write'in sys.argv:p.write_bytes(raw)
 else:assert p.read_bytes()==raw
 print(json.dumps(dict(rows=len(rows),machines=2*len(rows),constructors=2*len(rows)*len(KINDS),queries_per_machine=len(rows[0]['native']['queries']),executed=len(pins),intercepted=len(seams),sha256=hashlib.sha256(raw).hexdigest())))
if __name__=='__main__':main()
