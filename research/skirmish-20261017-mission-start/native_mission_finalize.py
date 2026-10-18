"""Native CityVictoryRatio arithmetic and zero-duration Sun update.

4FDF1B..4FDF8E executes retained x87 compare, binary32 temporary reload and
the real MSVC96CC4C truncation.4FDF8E is a captured boundary before Rule writes;
404F60 is explicit log transport. Positive list ownership is not fabricated.
4EDB10(duration0) executes raw target/current copy without scheduling seams.
4B5E10 source-store order uses explicit base/typedPoint3 getter seams; source
lookup/type/parse/provenance is bound independently by the runtime adapter.
"""
from pathlib import Path
import sys,json,itertools,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P
class Boundary(Exception):pass
def machine(image,pc):h.P=ORIGINAL if image=='original'else h.V;return h.Machine(pc)
def scalar(image,pc,maximum,available,current):
 m=machine(image,pc);manager=m.alloc(0x200);var=m.alloc(0x30);frame=m.alloc(0x100)+0x80
 m.w32(0xb39ab0,manager);m.w32(manager+0x130,var);m.w32(var,current)
 m.w32(frame-0x14,maximum);m.w32(frame-0x1c,available)
 logs=[];out=None
 def log(_):assert m.arg(2)==0x146f;logs.append([m.arg(5),m.arg(6),m.arg(7)]);m.ret()
 def end(_):
  nonlocal out
  out=m.u.reg_read(UC_X86_REG_EDI);raise Boundary()
 m.seams[0x404f60]=log;m.seams[0x4fdf8e]=end
 m.u.reg_write(UC_X86_REG_EBP,frame);m.u.reg_write(UC_X86_REG_ESP,h.STACK)
 try:m.u.emu_start(0x4fdf1b,0x4fdfe9,count=10000);assert m.u.reg_read(UC_X86_REG_EIP)==0x4fdfe9
 except Boundary:pass
 return {'new_ratio':out,'temporary_bits':m.r32(frame-0x28),'logs':logs}
def sunlight(image,pc,colors):
 m=machine(image,pc);sun=m.alloc(0x88);before=[(0x12340000+i*0x10101)&0xffffffff for i in range(34)]
 for i,v in enumerate(before):m.w32(sun+4*i,v)
 inputs=[]
 for triple in colors:
  a=m.alloc(12);m.w32(a,*triple);inputs.append(a)
 m.call(0x4edb10,sun,(*inputs,0));after=[m.r32(sun+4*i)for i in range(34)]
 return {'before':before,'after':after}
def source(image,pc,colors):
 m=machine(image,pc);obj=m.alloc(0x40);node=m.alloc(0x20);seen=[]
 m.seams[0x47d960]=lambda _:m.ret(4)
 def getter(_):
  assert m.ecx()==node
  ptr=m.arg(1);raw=bytearray()
  while bytes(m.u.mem_read(ptr,2))!=b'\0\0':raw+=bytes(m.u.mem_read(ptr,2));ptr+=2
  name=raw.decode('utf-16le');seen.append(name);index=['diffuse','ambient','specular'].index(name)
  output=m.arg();m.w32(output,*colors[index]);m.ret(8,value=output)
 m.seams[0x5b4c30]=getter;m.call(0x4b5e10,obj,(node,))
 return {'fields':seen,'colors':[[m.r32(obj+0x18+12*i+4*j)for j in range(3)]for i in range(3)]}
def main():
 ratios=[];suns=[];sources=[]
 matrices=[[[0x3f800000,0,0x80000000],[0x3f4ccccd]*3,[0x7fc12345,0x7f800000,0xff800000]],[[i+1,i+2,i+3]for i in(0,10,20)]]
 for image,pc in itertools.product(('original','v14'),h.PRECISION):
  for maximum,available,current in [(1,3,33),(1,3,34),(0,5,1),(1,1,100),(4,3,99),(3,7,42),(3,7,43),(21474836,100000,99)]:
   ratios.append({'image':image,'precision':pc,'input':[maximum,available,current],'output':scalar(image,pc,maximum,available,current)})
  for colors in matrices:
   suns.append({'image':image,'precision':pc,'colors':colors,'output':sunlight(image,pc,colors)})
   sources.append({'image':image,'precision':pc,'colors':colors,'output':source(image,pc,colors)})
 seams={0x404f60,0x4fdf8e,0x47d960,0x5b4c30}
 doc={'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'seams_not_executed':[hex(a)for a in sorted(seams)],'executed_pins':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams],
  'ratios':ratios,'suns':suns,'source_stores':sources}
 p=HERE/'native-mission-finalize.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'ratios':len(ratios),'suns':len(suns),'source_stores':len(sources),'pins':len(doc['executed_pins']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
