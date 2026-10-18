"""True4B7870 TerrainData ctor and bounded4B87A0 Property-list producer.
CString allocator/operator-new seams are the existing native-machine helpers.
Typed View count/scalar getters are explicit source facts, not a TGI parser
proof. Positive reference lookup is supplied before native prepend fragment;
this does not execute the compiled Property modifiers/criteria.
"""
from pathlib import Path
import argparse,hashlib,json,sys
sys.dont_write_bytecode=True
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-conversion'))
import native_machine as nm
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EDI,UC_X86_REG_EBP,UC_X86_REG_ESP

def run(image,pc,refs):
 m=nm.Machine(pc)
 old=(nm.P.image_size+4095)&~4095;new=(image.image_size+4095)&~4095
 if new>old:m.u.mem_map(image.base+old,new-old)
 for _,rva,size,offset in image.sections:m.u.mem_write(image.base+rva,image.b[offset:offset+size])
 pins={}
 def observe(_,address,size,__):
  if image.base<=address<image.base+image.image_size:
   raw=image.range(address,size);assert raw==nm.P.range(address,size)==nm.V.range(address,size);pins[f'{address:08x}']=raw.hex()
 m.u.hook_add(UC_HOOK_CODE,observe)
 obj=m.alloc(0xb8,0xa5);ids=m.cstring('explicitTerrainData')
 assert m.call(0x4b7870,obj,[ids],4)==obj and m.r(obj+0xb0)==m.r(obj+0xb4)==0
 constructor_null=True
 if not refs:
  m.seam(0x5b4ed0,lambda x:x.ret(4,0))
  m.seam(0x5b4220,lambda x:x.ret(4,0))
  m.w(nm.STACK,nm.STOP);m.u.reg_write(UC_X86_REG_ESP,nm.STACK);m.u.reg_write(UC_X86_REG_ESI,obj);m.u.reg_write(UC_X86_REG_EDI,m.alloc(0x100,0xa5))
  m.u.emu_start(0x4b8bf8,0x4b8cf4,count=10000)
 else:
  for ref in refs:
   m.w(nm.STACK,nm.STOP);m.u.reg_write(UC_X86_REG_ESP,nm.STACK);m.u.reg_write(UC_X86_REG_ESI,obj);m.u.reg_write(UC_X86_REG_EBP,ref)
   m.u.emu_start(0x4b8c8e,0x4b8cb9,count=10000)
 out=[];cursor=m.r(obj+0xb0);last=0
 while cursor:out.append(m.r(cursor));last=cursor;cursor=m.r(cursor+4)
 assert last==m.r(obj+0xb4) and out==list(reversed(refs))
 assert not m.null_accesses and m.outcome is None
 return dict(constructor_null=constructor_null,head_order=out,head_null=m.r(obj+0xb0)==0,tail_null=m.r(obj+0xb4)==0),pins

def main():
 ap=argparse.ArgumentParser(description=__doc__);ap.add_argument('--write',action='store_true');args=ap.parse_args();rows=[];pins={}
 for pc in ['Single24','Double53','Extended64']:
  for refs in [[],[0x11111111],[0x11111111,0x22222222,0x11111111],[0,0x11111111]]:
   a,pa=run(nm.P,pc,refs);b,pb=run(nm.V,pc,refs);assert a==b and pa==pb;pins.update(pa);rows.append(dict(precision=pc,references=refs,output=a))
 out=dict(schema=1,scope=__doc__,paired_rows=len(rows),machines=len(rows)*2,cases=rows,instruction_pins=pins)
 p=HERE/'native-terrain-property-source.json'
 if args.write:assert not p.exists();p.write_text(json.dumps(out,indent=2)+'\n')
 else:assert json.loads(p.read_bytes())==out
 print(json.dumps(dict(paired_rows=len(rows),machines=len(rows)*2,pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
