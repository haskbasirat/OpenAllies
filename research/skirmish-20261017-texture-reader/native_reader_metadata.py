"""Actual dev converter/reader dispatch and NiPixelData metadata.
Runs original game instructions in fresh original/v14 machines. Allocation,
free and OS mutex/thread APIs are success-only transports; stream read/seek
is an explicit in-memory byte transport. No production executable is run.
"""
from pathlib import Path
import sys,struct,json,zlib,itertools
HERE=Path(__file__).resolve().parent; ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as n

def dds(w,h,levels=1,kind='DXT1',caps=None):
 b=bytearray(128);b[:4]=b'DDS '
 def put(off,v):struct.pack_into('<I',b,off,v)
 put(4,124);put(8,0x21007);put(12,h);put(16,w);put(28,levels)
 put(76,32);put(80,4 if kind.startswith('DXT')else 0x41)
 if kind.startswith('DXT'):b[84:88]=kind.encode()
 else:
  for off,v in [(88,32),(92,255),(96,65280),(100,16711680),(104,4278190080)]:put(off,v)
 put(108,(0x401008 if levels>1 else 0x1000)if caps is None else caps)
 #Enough exact source payload for these finite test dimensions/mips.
 for i in range(levels):
  x=max(1,w>>i);y=max(1,h>>i)
  size=((x+3)//4)*((y+3)//4)*(8 if kind=='DXT1'else 16)if kind.startswith('DXT')else x*y*4
  b.extend(bytes([0x31+i])*size)
 return bytes(b)

def tga(w,h,depth=24,rle=False):
 b=bytearray(18);b[2]=10 if rle else 2
 struct.pack_into('<HH',b,12,w,h);b[16]=depth;b[17]=0x20
 pixels=bytes(range(depth//8))
 if rle:
  left=w*h
  while left:
   take=min(left,128);b.append(0x80+take-1);b.extend(pixels);left-=take
 else:b.extend(pixels*(w*h))
 return bytes(b)

def png(w,h,color=6):
 def chunk(k,d):return struct.pack('>I',len(d))+k+d+struct.pack('>I',zlib.crc32(k+d)&0xffffffff)
 channels={0:1,2:3,4:2,6:4}[color]
 raw=b''.join(b'\0'+bytes([0x63])*(w*channels)for _ in range(h))
 return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',w,h,8,color,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b'')

def run(c,image,pc):
 m=n.Machine(pc)
 if image=='v14':
  old=(n.original.image_size+4095)&~4095;new=(n.modified.image_size+4095)&~4095
  if new>old:m.u.mem_map(n.original.base+old,new-old)
  for vs,rva,raw,off in n.modified.sections:m.u.mem_write(n.modified.base+rva,n.modified.b[off:off+raw])
 def alloc(mm):
  size=mm.arg(1);p=mm.alloc(size);mm.u.mem_write(p,b'\xcd'*size);mm.ret(p)
 m.seam(0x6f7810,alloc);m.seam(0x6f7430,lambda mm:mm.ret())
 def heap_alloc(mm):
  size=mm.arg(3);p=mm.alloc(size);mm.u.mem_write(p,bytes(size));mm.ret(p,12)
 m.w(0x9f0188,m.stub(heap_alloc))
 m.w(0x9f0184,m.stub(lambda mm:mm.ret(1,12)))
 #Successful non-throwing libpng path only: host setjmp returns initial zero.
 m.seam(0x96f864,lambda mm:mm.ret(0))
 def cstr(p):
  b=bytearray()
  while m.rb(p):b.append(m.rb(p));p+=1
  return bytes(b)
 #ASCII-only fixture C splitpath and stricmp transport. The actual converter
 #list, per-reader virtual supports callback and selection execute unchanged.
 def split(mm):
  name=cstr(mm.arg(1));stem,ext=name.rsplit(b'.',1)
  for i,s in [(2,b''),(3,b''),(4,stem),(5,b'.'+ext)]:
   p=mm.arg(i)
   if p:mm.u.mem_write(p,s+b'\0')
  mm.ret()
 m.seam(0x971bb0,split)
 m.seam(0x970817,lambda mm:mm.ret(0 if cstr(mm.arg(1)).lower()==cstr(mm.arg(2)).lower()else 1))
 m.w(0x9f0214,m.stub(lambda mm:mm.ret(1,4)))
 m.w(0x9f01e4,m.stub(lambda mm:mm.ret(0x1729)))
 for a in [0x9f01f0,0x9f01f4,0x9f0210]:m.w(a,m.stub(lambda mm:mm.ret(1,4)))
 m.call(0x9e75c0);m.call(0x9e75f0)
 converter=m.alloc(0x400);m.call(0x89a950,converter)
 m.call(0x896110,converter,[c['mode']])
 data=bytes.fromhex(c['bytes']);pos=0;events=[]
 def read(mm):
  nonlocal pos
  destination,size=mm.arg(1),mm.arg(2);part=data[pos:pos+size]
  if len(part)!=size:raise RuntimeError(('short-read',pos,size,len(data)))
  mm.u.mem_write(destination,part);pos+=size;mm.ret(size,8)
 def seek(mm):
  nonlocal pos
  off,origin=mm.arg(1),mm.arg(2)
  if off&0x80000000:off-=0x100000000
  pos=off if origin==0 else(pos+off if origin==1 else len(data)+off)
  assert 0<=pos<=len(data),(pos,len(data),origin)
  mm.ret(1,8)
 vtable=m.array([0,0,m.stub(read),0,m.stub(seek)])
 stream=m.array([vtable]);name=m.alloc(64);m.u.mem_write(name,('actual.'+c['extension']+'\0').encode())
 for tag,va in [('PNG',0x8dc310),('TGA',0x8dcd30),('DDS',0x8de7f0)]:
  m.observe(va,lambda mm,t=tag:events.append(['reader',t]))
 try:pixel=m.call(0x897670,converter,[name,0,stream],count=8_000_000)
 except Exception:
  from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
  print('FAILED',hex(m.u.reg_read(UC_X86_REG_EIP)),hex(m.u.reg_read(UC_X86_REG_ESP)),[hex(x)for x in list(m.visited)[-20:]],flush=True)
  raise
 if not pixel:return dict(accepted=False,events=events,position=pos),m.visited
 count=m.r(pixel+0x44);assert count<=16,count
 output=dict(accepted=True,events=events,position=pos,
  format=[m.r(pixel+12+i*4)for i in range(9)],
  widths=[m.r(m.r(pixel+0x38)+i*4)for i in range(count)],
  heights=[m.r(m.r(pixel+0x3c)+i*4)for i in range(count)],
  offsets=[m.r(m.r(pixel+0x40)+i*4)for i in range(count+1)],stride=m.r(pixel+0x48))
 return output,m.visited

def cases():
 result=[]
 for kind,shape,mode in itertools.product(['DXT1','DXT3','DXT5','RGBA32'],[(16,8,5),(7,5,3),(4,4,1)],[0,1,3]):
  w,h,l=shape;result.append(dict(extension='dds',mode=mode,bytes=dds(w,h,l,kind).hex()))
 for depth,rle in itertools.product([24,32],[False,True]):result.append(dict(extension='tga',mode=0,bytes=tga(3,2,depth,rle).hex()))
 for color in [0,2,4,6]:result.append(dict(extension='png',mode=0,bytes=png(3,2,color).hex()))
 #Additional dimension/mode and native header refusals. No failed longjmp path
 #or short stream transport is presented as successful read equivalence.
 for kind,shape,mode in [('DXT1',(32,32,6),3),('DXT3',(9,17,4),1),('DXT5',(1,8,4),8),('RGBA32',(8,8,4),1)]:
  w,h,l=shape;result.append(dict(extension='DDS',mode=mode,bytes=dds(w,h,l,kind).hex()))
 for extension,mode,data in [('TGA',7,tga(2,1,24,True)),('PNG',7,png(2,3,2))]:result.append(dict(extension=extension,mode=mode,bytes=data.hex()))
 for offset,value in [(0,0),(4,123),(8,0),(84,0x30315844),(112,0x200)]:
  b=bytearray(dds(4,4));struct.pack_into('<I',b,offset,value);result.append(dict(extension='dds',mode=0,bytes=b.hex()))
 return result

def main():
 rows=[];pins={};cs=cases()
 if '--probe'in sys.argv:cs=cs[:1]
 for i,c in enumerate(cs):
  for pc in (['Double53']if'--probe'in sys.argv else n.PRECISIONS):
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(i,pc,a,b)
   pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
  print(i,a,flush=True)
 for va,raw in pins.items():assert n.original.range(va,len(raw))==n.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),
  rows=rows,native_executions=len(rows)*2,instructions=n.instructions(pins))
 if '--probe'in sys.argv:return
 path=HERE/'native-reader-metadata.json'
 if '--write'in sys.argv:
  with path.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),executions=len(rows)*2,pins=len(pins),sha256=n.sha(path.read_bytes()))))
if __name__=='__main__':main()

