"""Full7515A0 client description with real765E00/766380 source reads.

Both executable images at all precision controls execute the caller/getters,
CString retain/copy/release. The405D60 CString destination/storage boundary
uses the independently executed original96C7EC wide CRT formatter in a second
machine, with its named C-locale thread-data seam. No color-source loader or
native404F60 logger threshold/ABI completion is claimed by these rows.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/development10/kingdom-glyph'))
from native_kingdom_glyph import GlyphMachine,PE0,PE1,PC
from unicorn import UC_HOOK_MEM_READ
from unicorn.x86_const import UC_X86_REG_EIP

def units(s):
 b=s.encode('utf-16-le','surrogatepass');return list(struct.unpack('<'+'H'*(len(b)//2),b))
def text(v):return struct.pack('<'+'H'*len(v),*v).decode('utf-16-le','surrogatepass')
def sha(b):return hashlib.sha256(b).hexdigest()

class Machine(GlyphMachine):
 def __init__(self,image):
  self.description_ready=False;super().__init__(image)
  self.u.hook_add(UC_HOOK_MEM_READ,self.read)
 def read(self,u,access,address,size,value,user):
  if not self.description_ready:return
  if address==self.player+0x28:
   assert self.world_present,'765E00 must not read Player28 with GWorld NULL'
  if address==self.color+0x10:self.trace.append(['color',self.color_units])
  if address==self.player+0x20:self.trace.append(['id',self.player_id])
 def hook(self,u,pc,size,user):
  if self.description_ready:
   if pc==0x765e00:
    self.getters+=1
    self.trace.append(['kingdom' if self.getters==1 else 'kingdom_again',self.world_present and self.kingdom_present])
   if pc==0x766380:self.trace.append(['name',self.name_units])
   if pc==0x405d60:
    fmt=self.text(self.arg(1));expected='[%s,%d,%s]'if self.world_present and self.kingdom_present else '[%s,%d]'
    assert fmt==expected,(fmt,expected)
    args=[self.arg(i)for i in range(2,5 if expected.endswith('%s]')else 4)]
    assert self.units(args[0])==self.name_units and args[1]==self.player_id
    if len(args)==3:assert self.units(args[2])==self.color_units
    self.format_args={'format':units(fmt),'name':self.units(args[0]),'id':args[1],'color':self.units(args[2])if len(args)==3 else None}
    self.w(self.arg(),self.string(text(self.format_result)));self.ret(value=self.arg());return
  super().hook(u,pc,size,user)

def run(m,crt,pc,c):
 m.fpu(pc);crt.fpu(pc);m.description_ready=False
 m.player=m.alloc(0x80);m.client=m.alloc(0x80);m.kingdom=m.alloc(0x300);m.color=m.alloc(0x24)
 m.name_units=c['name'];m.color_units=c['color'];m.player_id=c['id'];m.world_present=c['world'];m.kingdom_present=c['kingdom']
 m.cs(m.player+8,text(c['name']));m.w(m.player+0x20,c['id']);m.w(m.player+0x28,m.kingdom if c['kingdom']else 0)
 m.w(m.client+8,m.player);m.w(m.kingdom+0x1e8,m.color);m.cs(m.color+0x10,text(c['color']))
 m.w(0xb36534,m.kingdom if c['world']else 0)
 has=c['world']and c['kingdom'];fmt='[%s,%d,%s]'if has else'[%s,%d]'
 raw=struct.pack('<II',crt.string(text(c['name'])),c['id'])
 if has:raw+=struct.pack('<I',crt.string(text(c['color'])))
 m.format_result=crt.fmt(fmt,raw)
 m.trace=[];m.getters=0;m.format_args=None;m.actual={};m.record_actual=True;m.description_ready=True
 out=m.alloc(4);m.call(0x7515a0,[out],this=m.client,callee_pop=4)
 result=m.rs(out);assert result==m.format_result
 m.description_ready=False
 return {'text':result,'events':m.trace,'format_args':m.format_args},m.actual.copy()

def main():
 cases=[]
 for world,kingdom in [(False,True),(True,False),(True,True)]:
  for name,color in [([],[]),(units('Player'),units('Blue')),(units('A\0ignored'),units('B\0ignored')),(units('\ud800\udfff'),units('\ud800X'))]:
   for player_id in [0,1,0x7fffffff,0x80000000,0xffffffff]:cases.append({'world':world,'kingdom':kingdom,'name':name,'color':color,'id':player_id})
 rows=[];pins={};crt_pins={}
 for image in ['original','v14']:
  m=Machine(image);crt=GlyphMachine(image);crt.record_actual=True
  for pc in PC:
   for c in cases:
    output,visited=run(m,crt,pc,c);pins.update(visited);rows.append({'image':image,'precision':pc,'input':c,'output':output})
  crt_pins.update(crt.actual)
 a=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in rows if r['image']=='v14'];assert a==b
 def proof(v):
  out=[]
  for va,size in sorted(v.items()):
   if PE0.base<=va<PE0.base+PE0.image_size:
    raw=PE0.range(va,size);assert raw==PE1.range(va,size);out.append({'va':hex(va),'bytes':raw.hex()})
  return out
 doc={'schema':1,'scope':__doc__,'producer_sha256':sha(Path(__file__).read_bytes()),'original_sha256':sha(PE0.b),'v14_sha256':sha(PE1.b),'rows':rows,'instructions':proof(pins),'crt_instructions':proof(crt_pins)}
 p=HERE/'native-description.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(doc['instructions']),'crt_pins':len(doc['crt_instructions']),'sha256':sha(p.read_bytes())}))
if __name__=='__main__':main()
