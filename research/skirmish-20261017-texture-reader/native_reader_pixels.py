"""Full native reader byte successor; frozen metadata producer is unchanged.
Captures actual returned PixelData bytes and native-format-normalized RGBA.
DDS compressed GPU expansion is NOT synthesized or claimed here.
"""
from pathlib import Path
import sys,json,struct,zlib,itertools
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE));import native_reader_metadata as base
n=base.n

def tga(depth,descriptor,rle):
 #Distinct channels/alpha including transparent and half-alpha colors. The
 #repeated middle pair makes the mixed raw/run packet path meaningful.
 rgba=[(9,31,87,0),(211,49,19,127),(17,129,245,255),(17,129,245,255),(0,255,3,7),(251,2,143,193)]
 pixels=[bytes((b,g,r,a)if depth==32 else(b,g,r))for r,g,b,a in rgba]
 h=bytearray(18);h[0]=3;h[2]=10 if rle else 2
 struct.pack_into('<HHHH',h,8,13,29,3,2);h[16]=depth;h[17]=descriptor
 if rle:payload=bytes([1])+pixels[0]+pixels[1]+bytes([0x81])+pixels[2]+bytes([1])+pixels[4]+pixels[5]
 else:payload=b''.join(pixels)
 return bytes(h)+b'ID!'+payload

def png(color):
 def chunk(k,d):return struct.pack('>I',len(d))+k+d+struct.pack('>I',zlib.crc32(k+d)&0xffffffff)
 colors={
  0:[bytes([0,19,255]),bytes([201,7,132])],
  2:[bytes([9,31,87,211,49,19,17,129,245]),bytes([7,19,251,0,255,3,251,2,143])],
  4:[bytes([0,0,19,127,255,255]),bytes([201,7,7,193,132,63])],
  6:[bytes([9,31,87,0,211,49,19,127,17,129,245,255]),bytes([7,19,251,7,0,255,3,193,251,2,143,63])],
 }
 raw=b''.join(b'\0'+row for row in colors[color])
 return b'\x89PNG\r\n\x1a\n'+chunk(b'IHDR',struct.pack('>IIBBBBB',3,2,8,color,0,0,0))+chunk(b'IDAT',zlib.compress(raw))+chunk(b'IEND',b'')

def dds(padded,bgra=False,alpha=True):
 h=bytearray(base.dds(2,2,1,'RGBA32')[:128]);struct.pack_into('<I',h,8,0x100f)
 struct.pack_into('<I',h,20,16 if padded else 8)
 if bgra:
  struct.pack_into('<I',h,92,0xff0000);struct.pack_into('<I',h,100,0xff)
 if not alpha:struct.pack_into('<I',h,80,0x40)
 rows=[bytes([9,31,87,0,211,49,19,127]),bytes([17,129,245,255,251,2,143,193])]
 pad=bytes([7,19,251,13,23,31,41,53])
 return bytes(h)+b''.join(row+(pad if padded else b'')for row in rows)

def cases():
 for depth,descriptor,rle in itertools.product([24,32],[0,0x20,0x10,0x30,0x28],[False,True]):
  yield dict(label=f'tga{depth}-descriptor{descriptor}-rle{rle}',extension='tga',mode=0,bytes=tga(depth,descriptor,rle).hex())
 for color in [0,2,4,6]:yield dict(label=f'png8-color{color}',extension='png',mode=0,bytes=png(color).hex())
 for padded,bgra,alpha in itertools.product([False,True],[False,True],[False,True]):
  yield dict(label=f'dds32-padded{padded}-bgra{bgra}-alpha{alpha}',extension='dds',mode=0,bytes=dds(padded,bgra,alpha).hex())

def run(c,image,pc):
 original=n.Machine; machines=[]
 class Capture(original):
  def __init__(self,p):super().__init__(p);machines.append(self);self.pixel=None
  def call(self,entry,*args,**kwargs):
   result=super().call(entry,*args,**kwargs)
   if entry==0x897670:self.pixel=result
   return result
 n.Machine=Capture
 try:out,pins=base.run(c,image,pc)
 finally:n.Machine=original
 m=machines[0];pixel=m.pixel;assert pixel and out['accepted'],c['label']
 size=out['offsets'][-1];raw=bytes(m.u.mem_read(m.r(pixel+0x34),size));stride=out['stride']
 assert stride in [3,4],out
 masks=out['format'][2:6];rgba=[]
 for offset in range(0,len(raw),stride):
  word=int.from_bytes(raw[offset:offset+stride],'little')
  for i,mask in enumerate(masks):
   if mask==0:assert i==3;rgba.append(255)
   else:
    shift=(mask&-mask).bit_length()-1;assert mask>>shift==255
    rgba.append((word&mask)>>shift)
 out.update(native_bytes=raw.hex(),normalized_rgba=bytes(rgba).hex())
 return out,pins

def main():
 rows=[];pins={}
 for c in cases():
  for pc in n.PRECISIONS:
   a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b,(c,pc,a,b)
   rows.append(dict(precision=pc,input=c,output=a));pins.update(pa);pins.update(pb)
  print(c['label'],a['normalized_rgba'],flush=True)
 for va,raw in pins.items():assert n.original.range(va,len(raw))==n.modified.range(va,len(raw))==raw
 doc=dict(schema=1,scope=__doc__,base_producer_sha256=n.sha(Path(base.__file__).read_bytes()),
  original_sha256=n.sha(n.original.b),v14_sha256=n.sha(n.modified.b),rows=rows,native_executions=len(rows)*2,instructions=n.instructions(pins))
 p=HERE/'native-reader-pixels.json'
 if '--write'in sys.argv:
  with p.open('x')as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),native_executions=2*len(rows),pins=len(pins),sha256=n.sha(p.read_bytes()))))
if __name__=='__main__':main()
