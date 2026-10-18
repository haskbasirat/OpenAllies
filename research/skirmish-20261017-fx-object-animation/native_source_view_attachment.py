"""42916C..429194 actual source Thing+0C -> current View+54 -> View virtualBC.
Native execution stops before event allocation. A positive selected attachment
requires45C740 later and is deliberately unsupported in the held adapter.
No immutable ObjectData flag or source needs_space getter is used here.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_ESP,UC_X86_REG_FPCW

def run(c,image,pc):
 m=nm.Machine(pc)
 if image=='v14':
  old=(nm.original.image_size+4095)&~4095;new=(nm.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,r,size,off in nm.modified.sections:
   if size:m.u.mem_write(nm.modified.base+r,nm.modified.b[off:off+size])
 thing=m.alloc(0x60);view=m.alloc(0x60);vt=m.alloc(0xc0);events=[];reads=[]
 m.w(thing+0xc,view if c['view'] else 0);m.w(view,vt);m.w(view+0x54,c['flags'])
 m.w(vt+0xbc,m.stub(lambda mm:(events.append('current_view_BC'),mm.ret(c['bc']))))
 m.u.hook_add(UC_HOOK_MEM_READ,lambda u,access,address,size,value,data:reads.append([address,size]))
 m.seam(0x429194,lambda mm:mm.u.emu_stop())
 m.u.reg_write(UC_X86_REG_ESI,thing if c['thing'] else 0);m.u.reg_write(UC_X86_REG_EDI,0xdeadbeef)
 m.u.reg_write(UC_X86_REG_ESP,nm.STACK);m.u.reg_write(UC_X86_REG_FPCW,nm.PRECISIONS[pc])
 m.u.emu_start(0x42916c,nm.STOP,count=100)
 selected=m.u.reg_read(UC_X86_REG_EDI)
 assert selected in [0,view]
 expected=c['thing'] and c['view'] and bool(c['flags']&0xc0000)
 assert events==(['current_view_BC'] if expected else [])
 assert any(address==view+0x54 for address,size in reads)==bool(c['thing'] and c['view'])
 assert bool(selected)==bool(expected and c['bc'])
 return {'selected_current_view':bool(selected),'events':events,'read_view_flags':bool(c['thing'] and c['view'])},m.visited

def main():
 rows=[];pins={}
 for pc,thing,view,flags,bc in itertools.product(nm.PRECISIONS,[False,True],[False,True],[0,0x40000,0x80000,0xc0000,0xffffffff],[0,1]):
  c=dict(thing=thing,view=view,flags=flags,bc=bc)
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
 for va,raw in pins.items():assert raw==nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))
 doc={**nm.header(__file__,__doc__),'rows':rows,'instructions':nm.instructions(pins),'required_stop':'429194 before allocation','virtual_BC':'explicit current View predicate input'}
 p=HERE/'native-source-view-attachment.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=1)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),runs=2*len(rows),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
