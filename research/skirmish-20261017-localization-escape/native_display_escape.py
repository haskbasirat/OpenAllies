"""Actual494000 DisplayString escape decoding and actual48D500 Glyph Find.
CString/heap transport and ASCII CRT comparison are named seams. Malformed or
unknown escapes STOP at actual410C40 diagnostic entry; no diagnostic body is
replaced by success. Explicit glyph/AVar inputs do not claim source construction.
"""
from pathlib import Path
import sys,json,struct,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-saved-components'));import native_machine as h
from unicorn.x86_const import UC_X86_REG_EIP,UC_X86_REG_ESP
H=lambda b:hashlib.sha256(b).hexdigest()
class Machine(h.Machine):
 def __init__(self,image,pc,c):
  super().__init__(b'',pc)
  if image is not h.P:
   old=(h.P.image_size+0xfff)&~0xfff;new=(image.image_size+0xfff)&~0xfff
   if new>old:self.u.mem_map(image.base+old,new-old)
   for vs,r,raw,off in image.sections:
    if raw:self.u.mem_write(image.base+r,image.b[off:off+raw])
  self.c=c;self.stop=None;self.lookups=[]
  #Native CString COW reaches CRT HeapAlloc import; explicit Win32 allocation transport.
  self.w(0x9f0188,self.stub(lambda m:m.ret(12,m.alloc(m.arg(2),0xa5))))
  self.w(0x9f0184,self.stub(lambda m:m.ret(12,1)))
  self.manager=self.alloc(0x70);self.data=self.alloc(0x400);self.vars=self.alloc(0xa0)
  self.w(self.data+0x3bc,self.manager);self.w(0xb36530,self.data);self.w(0xb39990,self.vars)
  for off,value in[(0x80,c['start']),(0x84,c['end']),(0x90,c['single'])]:
   value_ptr=self.alloc(4);self.w(value_ptr,value);self.w(self.vars+off,value_ptr)
  bucket=self.alloc(4);self.w(self.manager+0x30,bucket);self.w(self.manager+0x34,1);self.w(self.manager+0x28,len(c['glyphs']))
  for name,value in c['glyphs'].items():
   glyph=self.alloc(0x20);self.w(glyph+8,self.cstring(name));self.u.mem_write(glyph+0x18,struct.pack('<H',value&0xffff))
   node=self.alloc(8);self.w(node,glyph);self.w(node+4,self.r(bucket));self.w(bucket,node)
  def compare(m):
   a=m.read_wide(m.arg(0));b=m.read_wide(m.arg(1));assert a.isascii()and b.isascii();m.ret(0,0 if a.lower()==b.lower()else 1)
  self.seam(0x96cad6,compare)
  def missing(m):self.stop=m.read_wide(m.arg(0));m.u.emu_stop()
  self.seam(0x410c40,missing)
  self.probes[0x48d500]=lambda m:self.lookups.append(m.read_wide(m.arg(0)))
  self.string=self.alloc(4);self.w(self.string,self.cstring(c['text']))
 def run(self):
  self.w(h.STACK,h.STOP);self.w(h.STACK+4,self.string);self.u.reg_write(UC_X86_REG_ESP,h.STACK)
  self.u.emu_start(0x494000,h.STOP,count=2000000)
  assert self.u.reg_read(UC_X86_REG_EIP)==h.STOP or self.stop
  if not self.stop:assert self.u.reg_read(UC_X86_REG_ESP)==h.STACK+4 #cdecl caller retains its argument
  text=self.cstring_text(self.r(self.string));raw=text.encode('utf-16-le')
  return {'outcome':'required_diagnostic'if self.stop else'complete','text':[int.from_bytes(raw[i:i+2],'little')for i in range(0,len(raw),2)],'diagnostic':self.stop,'lookups':self.lookups}
def cases():
 glyphs={'ch_faction_experience':133,'n':10,'ch_amp':38,'ch_high':0x12345}
 for text in['plain','&ch_faction_experience.Hedges','x&CH_FACTION_EXPERIENCE.y','&ch_amp.&ch_high.','a\\nb','&&','\\\\','\\&','&missing.','&ch_amp.&missing.','&','\\','&unterminated','@&ch_faction_experience.Hedges','&ch_amp.&&x','a&n.b']:
  yield {'text':text,'glyphs':glyphs,'start':38,'end':46,'single':92}
 yield {'text':'&unterminated','glyphs':{},'start':38,'end':46,'single':92}
 yield {'text':'%n:x/n','glyphs':{'n':10},'start':37,'end':58,'single':47}
def main():
 rows=[]
 for c in cases():
  for pc in h.CW:
   v=[Machine(image,pc,c).run()for image in(h.P,h.V)];assert v[0]==v[1];rows.append({'input':c,'pc':pc,'native':v[0]})
 seams={0x410c40:'required_diagnostic_entry',0x96cad6:'ASCII_CRT_comparison_transport',0x856370:'CString_allocator_transport',0x6f7810:'heap_transport',0x6f7430:'heap_transport',0x401b00:'CString_literal_transport',0x856560:'CString_literal_transport'}
 doc={'schema':1,'scope':__doc__,'producer_sha256':H(Path(__file__).read_bytes()),'original_sha256':H(h.P.b),'v14_sha256':H(h.V.b),'rows':rows,'machines':len(rows)*2,'pins':[{'va':hex(a),'bytes':b,'kind':seams.get(a,'executed')}for a,b in sorted(h.VISITED.items())],'compiled_escape_defaults':[{'va':hex(a),'bytes':h.P.range(a,n).hex(),'meaning':label}for a,n,label in[(0x402bd6,14,'401BD0 TextEscapeCharacterStart default0x26'),(0x402c17,16,'401BD0 TextEscapeCharacterEnd default0x2E'),(0x402ce6,21,'401BD0 TextOneCharEscapeCharacter default0x5C')]],'external_import_transports':[{'iat':'0x9f0188','name':'HeapAlloc','result':'fresh A5-filled owned block'},{'iat':'0x9f0184','name':'HeapFree','result':'success; no pointer reuse'}]}
 for a,n in[(0x402bd6,14),(0x402c17,16),(0x402ce6,21)]:assert h.P.range(a,n)==h.V.range(a,n)
 p=HERE/'native-display-escape.json'
 if'--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'machines':len(rows)*2,'pins':len(h.VISITED),'sha256':H(p.read_bytes())}))
if __name__=='__main__':main()
