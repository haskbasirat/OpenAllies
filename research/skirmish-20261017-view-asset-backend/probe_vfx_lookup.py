from pathlib import Path
import sys
sys.path.insert(0,str(Path('research/phase8-definitions').resolve()));import native_definition_registry as h
m=h.Machine();m.thiscall(0x4013e0,0xb43b40,[])
mgr=h.make_thing_manager(m);m.w(0xb36530,mgr-12)
for i,kind in enumerate([0,5,6,7,8,9,10,11,12,13,14,15,16,17,22,23]):
 category=m.alloc(0x40);m.w(category+0x18,kind);m.w(category+0xc,i)
 ids='kind'+str(kind);r=m.thiscall(0x4bca00,mgr,[m.cstring(ids),category]);print('ctor',kind,r,flush=True)
 if r.get('thrown'):raise RuntimeError(r)
 obj=r['eax'];vt=m.r(obj);fn=m.r(vt+0x48);print('vtable',hex(vt),hex(fn),m.thiscall(fn,obj,[])['eax'],flush=True)
 result=m.thiscall(0x4bdb70,mgr,[m.cstring(ids)]);print('lookup',kind,result,flush=True)
