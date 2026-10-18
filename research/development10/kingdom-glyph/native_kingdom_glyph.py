"""Actual4E93C0 selection/RGB promotion plus original VC7.1 wide %g.

The native4E93C0 executes actual76B960/765E00 and76CE10/766380/wcscmp.
Named variable lookup,7666C0 player selection and48D500 Glyph lookup are
explicit supplied laboratory children. CString allocation/copy/format storage
is transport; the format child executes original96C7EC separately, in C locale
with the named9738A9 thread-data seam. No online-manager absence is inferred.
"""
from pathlib import Path
import sys,json,hashlib,struct,random
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[2]
sys.path.insert(0,str(ROOT/'research/development10/creator-labels'))
from native_creator_labels import Machine,PE0,PE1,PC
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP,UC_X86_REG_EAX

class GlyphMachine(Machine):
 def __init__(self,image):
  self.glyph_ready=False;super().__init__(image)
  self.call(0x96bd7f)
  self.out=self.alloc(2048);self.va=self.alloc(64)
  self.actual={};self.record_actual=False
 def fmt(self,fmt,args):
  f=self.string(fmt);self.u.mem_write(self.va,args)
  self.w(self.stack,self.stop);self.w(self.stack+4,self.out);self.w(self.stack+8,f);self.w(self.stack+12,self.va)
  self.u.reg_write(UC_X86_REG_ESP,self.stack)
  self.u.emu_start(0x96c7ec,self.stop,count=2_000_000)
  assert self.u.reg_read(UC_X86_REG_EIP)==self.stop
  n=self.u.reg_read(UC_X86_REG_EAX)
  assert n<1024,n
  units=list(struct.unpack('<'+'H'*n,self.u.mem_read(self.out,2*n)))
  return units
 def hook(self,u,pc,size,user):
  if self.glyph_ready:
   if pc==0x4b2e60:
    assert self.text(self.arg())=='KingdomGlyphInfo'and self.arg(1)==4
    self.trace.append('lookup_kingdom');self.ret(8,self.variable);return
   if pc==0x7666c0:
    self.trace.append('player_glyph:'+str(self.players.index(self.ecx())))
    self.ret(value=self.selected if self.selected_nonnull else 0);return
   if pc==0x48d500:
    self.trace.append('find:'+self.text(self.arg()));self.ret(4,self.fallback);return
   if pc==0x405d60:
    assert self.text(self.arg(1))=='<color=%g,%g,%g>%c'
    sp=u.reg_read(UC_X86_REG_ESP)
    raw=bytes(u.mem_read(sp+12,28))
    self.trace.append('format')
    self.format_args={'double_bits':list(struct.unpack('<3Q',raw[:24])),'glyph':struct.unpack('<I',raw[24:])[0]}
    # Produce the already independently executed CRT output, preserving all
    # native argument promotions; this hook changes CString storage only.
    assert self.format_args==self.expected_args
    self.w(self.arg(),self.string(self.format_text));self.ret(value=self.arg());return
  if getattr(self,'color_ready',False):
   if pc==0x5b5620:assert self.text(self.arg())=='name';self.ret(4,self.string('color'));return
   if pc==0x405b80:self.ret(4);return
   if pc==0x5b4c30:
    assert self.text(self.arg(1))=='color'
    self.u.mem_write(self.arg(),struct.pack('<3I',*self.color_bits));self.ret(8,self.arg());return
  super().hook(u,pc,size,user)

def native_case(m,pc,c,crt):
 m.fpu(pc);m.actual={};m.record_actual=True;m.glyph_ready=True;m.trace=[];m.format_args=None
 k=m.alloc(0x300);color=m.alloc(0x24);m.w(k+0x190,c.get('minor',0));m.w(k+0x1e8,color)
 bits=c.get('rgb',[0,0x3e99999a,0x3f800000]);m.u.mem_write(color+0x18,struct.pack('<3I',*bits))
 m.selected=m.alloc(0x20);m.fallback=m.alloc(0x20)
 selected=c.get('selected',132);fallback=c.get('fallback',140)
 m.u.mem_write(m.selected+0x18,struct.pack('<H',selected));m.u.mem_write(m.fallback+0x18,struct.pack('<H',fallback))
 m.selected_nonnull=c.get('selected_nonnull',True)
 m.variable=m.alloc(0x20);m.cs(m.variable+0x10,'ch_gem')
 flags=c.get('flags',0);m.w(0xb3cbf4,flags);m.w(0xb3cbf0,m.variable if flags&1 else 0)
 session=m.alloc(0x180);m.w(0xb36550,session);m.w(session+0xe4,c.get('session',2));m.w(0xb36534,k if c.get('world',True)else 0)
 game=m.alloc(0x400);m.w(0xb36530,game);m.w(game+0x3bc,m.alloc(0x60))
 vt=m.alloc(0x30);m.w(vt+0x24,0x4e5340);m.w(k,vt)
 m.cs(k+0x198,c.get('name','Kingdom'))
 m.players=[];nodes=[]
 for i,p in enumerate(c.get('players',[{'display':'Kingdom','link':True}])):
  a=m.alloc(0x80);m.cs(a+8,p['display']);m.cs(a+0x24,'differentKingdomName');m.w(a+0x28,k if p.get('link',False)else 0)
  m.players.append(a);n=m.alloc(8);m.w(n,a);nodes.append(n)
 for i,n in enumerate(nodes):m.w(n+4,nodes[i+1]if i+1<len(nodes)else 0)
 m.w(session+0xbc,nodes[0]if nodes else 0)
 use_selected=False
 if not c.get('minor',0):
  if c.get('session',2)==2:found=next((i for i,p in enumerate(c.get('players',[{'display':'Kingdom','link':True}]))if c.get('world',True)and p.get('link',False)),None)
  else:found=next((i for i,p in enumerate(c.get('players',[{'display':'Kingdom','link':True}]))if p['display'].split('\0')[0]==c.get('name','Kingdom').split('\0')[0]),None)
  use_selected=found is not None and m.selected_nonnull
 glyph=selected if use_selected else fallback
 doubles=[float(struct.unpack('<f',struct.pack('<I',b))[0])for b in bits]
 raw=struct.pack('<3dI',*doubles,glyph)
 m.expected_args={'double_bits':list(struct.unpack('<3Q',raw[:24])),'glyph':glyph}
 units=crt.fmt('<color=%g,%g,%g>%c',raw)
 m.format_text=struct.pack('<'+'H'*len(units),*units).decode('utf-16-le','surrogatepass')
 out=m.alloc(4);m.call(0x4e93c0,[out],this=k,callee_pop=4)
 actual_units=m.rs(out)
 return {'text':actual_units,'flags':m.r(0xb3cbf4),'trace':m.trace,'format_args':m.format_args},m.actual.copy()

def main():
 rows=[];fmtrows=[];colorrows=[];pins={};crtpins={}
 cases=[{}, {'minor':1},{'minor':0xffffffff},{'players':[]},{'selected_nonnull':False},{'flags':1},
 {'session':3},{'session':4,'players':[{'display':'kingdom','link':True},{'display':'Kingdom','link':False}]},
 {'session':2,'players':[{'display':'Kingdom','link':False},{'display':'other','link':True}]},
 {'session':2,'world':False},{'session':3,'name':'A\0B','players':[{'display':'A\0C','link':False}]},
 {'selected':0},{'selected':0xd800},{'fallback':0x3a9,'players':[]}]
 rnd=random.Random(0x4e93c0)
 values=[0,0x80000000,1,0x7fffff,0x800000,0x3a800000,0x3f000000,0x3f800000,0x3dcccccd,0x3e99999a,0x7f7fffff,0xff7fffff]
 values += [rnd.randrange(0x7f800000)|(rnd.randrange(2)<<31)for _ in range(256)]
 for image in ['original','v14']:
  m=GlyphMachine(image);crt=GlyphMachine(image);crt.record_actual=True
  for pc in PC:
   crt.fpu(pc)
   for bits in values:
    v=float(struct.unpack('<f',struct.pack('<I',bits))[0]);units=crt.fmt('%g',struct.pack('<d',v))
    fmtrows.append({'image':image,'precision':pc,'bits':bits,'text':''.join(map(chr,units))})
   for c in cases:
    output,visited=native_case(m,pc,c,crt);pins.update(visited);rows.append({'image':image,'precision':pc,'input':c,'output':output})
   m.glyph_ready=False;m.color_ready=True
   for bits in [[0,0x3e99999a,0x3f800000],[0x3f59999a,0,0x3dcccccd],[0x7f7fffff,1,0x80000000]]:
    m.color_bits=bits;d=m.alloc(0x24);m.cs(d+0x10,'');node=m.alloc(4);m.fpu(pc);m.actual={};m.record_actual=True
    m.call(0x47e0f0,[node],this=d,callee_pop=4)
    colorrows.append({'image':image,'precision':pc,'input':bits,'stored':list(struct.unpack('<3I',m.u.mem_read(d+0x18,12))),'initialized':m.rb(d+0x14)})
    pins.update(m.actual)
  crtpins.update(crt.actual)
 for collection in [rows,fmtrows,colorrows]:
  a=[{k:v for k,v in r.items()if k!='image'}for r in collection if r['image']=='original'];b=[{k:v for k,v in r.items()if k!='image'}for r in collection if r['image']=='v14'];assert a==b
 def proof(visited):
  out=[]
  for va,size in sorted(visited.items()):
   if PE0.base<=va<PE0.base+PE0.image_size:
    raw=PE0.range(va,size);assert raw==PE1.range(va,size);out.append({'va':hex(va),'bytes':raw.hex()})
  return out
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'rows':rows,'format_rows':fmtrows,'color_rows':colorrows,'instructions':proof(pins),'crt_instructions':proof(crtpins)}
 p=HERE/'native-kingdom-glyph.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'format_rows':len(fmtrows),'color_rows':len(colorrows),'pins':len(doc['instructions']),'crt_pins':len(doc['crt_instructions']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
