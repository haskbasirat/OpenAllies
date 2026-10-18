"""Bounded actual Faction+30 source-list construction.
Real parser, string getter, comma tokenizer and prepend stores execute. Actual
48BAE0 prologue runs; public/art/Ability islands are explicitly skipped at48BB33
to the real48BC47 push then zero-count Ability loop. Property463FA0 is an explicit
canonical identity lookup service. Shared string-pool856370 and parser heap,
line-source/mutex/EH services remain the inherited harness seams. This is not a
full Faction/source loader; intercepted entry bytes are excluded from pins.
The473DB8 ctor interval proves vtable/zero list stores with explicit registers.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/skirmish-20261016-world-start'))
import native_technology_source as old
import trace_registry as mapped
from unicorn.x86_const import *
ORIGINAL=old.P;V14=old.V
class Cutoff(Exception):pass
class M(old.TechMachine):
 def __init__(self):self.active=False;super().__init__();self.properties={n:self.alloc(0x40)for n in ['first','second','third']}
 def hook(self,u,pc,size,user):
  if self.active:
   esp=u.reg_read(UC_X86_REG_ESP)
   if pc==0x48bb33:
    u.reg_write(UC_X86_REG_EBP,self.node);u.reg_write(UC_X86_REG_EDI,self.dest);u.reg_write(UC_X86_REG_ESI,0)
    self.w(esp+0x28,0);u.reg_write(UC_X86_REG_EIP,0x48bc47);return
   if pc==0x48be0a:raise Cutoff()
   if pc==0x463fa0:
    token=self.raw_wstr(self.arg());target=self.properties.get(token.lower(),0)
    self.recorded.append(dict(event='property_find',token=token,found=bool(target)));self.ret(target,pop=4);return
  super().hook(u,pc,size,user)
def run(tokens,image,pc):
 mapped.P=ORIGINAL if image=='original'else V14
 try:m=M()
 finally:mapped.P=ORIGINAL
 m.u.reg_write(UC_X86_REG_FPCW,{'Single24':0x7f,'Double53':0x27f,'Extended64':0x37f}[pc]);m.visited.clear()
 m.parse_file('template.tgi',b'[Template Faction]\n{string IDS\n localized name = ""\n bool public = true\n string properties = NULL\n}\n')
 text='[Faction template=Faction]\n{IDS=subject\n'+('properties='+tokens+'\n'if tokens is not None else'')+'}\n'
 parsed=m.parse_file('subject.tgi',text.encode());m.node=m.children(parsed['root'])[0]
 m.dest=m.alloc(0x80);m.u.mem_write(m.dest,b'\0'*0x80);m.w(m.dest+0x10,m.cstring(''));m.w(m.dest+8,m.cstring('subject'));m.active=True
 try:
  result=m.thiscall(0x48bae0,m.dest,[m.node]);raise AssertionError(('missing cutoff',result,hex(m.u.reg_read(UC_X86_REG_EIP)),m.last_throw,m.recorded[-8:]))
 except Cutoff:pass
 except AssertionError:
  if not(m.u.reg_read(UC_X86_REG_EIP)==0x96bef5 and m.last_throw):raise
 reverse={p:n for n,p in m.properties.items()};values=[];node=m.r(m.dest+0x30)
 while node:values.append(reverse.get(m.r(node),'NULL'));node=m.r(node+4)
 out=dict(throw=m.last_throw,values=values,head_nonnull=bool(m.r(m.dest+0x30)),tail_nonnull=bool(m.r(m.dest+0x34)),events=[e for e in m.recorded if e.get('event')=='property_find'])
 seams={0x463fa0,0x48bb33,0x48be0a,0x856370}
 pins={a:bytes(m.u.mem_read(a,n))for a,n in m.visited.items()if ORIGINAL.base<=a<ORIGINAL.base+ORIGINAL.image_size and a not in seams}
 return out,pins
def ctor(image,pc):
 sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
 h.P=h.Image(ROOT.parent/'AA.exe',h.ORIGINAL_SHA)if image=='original'else h.V
 m=h.Machine(pc);dest=m.alloc(0x80);m.u.mem_write(dest,b'\xaa'*0x80)
 #The actual native interval follows xor EBX,EBX in the owning constructor.
 shim=m.alloc(16);m.u.mem_write(shim,b'\x89\xce\x31\xdb\xe9'+(0x473db8-(shim+9)).to_bytes(4,'little',signed=True))
 m.seams[0x6f7810]=lambda _:m.ret(value=m.alloc(m.arg()))
 m.seams[0x473df0]=lambda _:(_ for _ in()).throw(Cutoff())
 try:m.call(shim,dest)
 except Cutoff:pass
 return dict(vtable=m.r32(dest),head=m.r32(dest+0x30),tail=m.r32(dest+0x34)),{a:bytes.fromhex(b)for a,b in h.VISITED.items()if a not in(0x6f7810,0x473df0)}
def main():
 rows=[];constructors=[];pins={}
 for pc,tokens in itertools.product(['Single24','Double53','Extended64'],[None,'NULL','first','first,second,third','second,FIRST,second',' first, second','first,,second','first,missing,third']):
  a,pa=run(tokens,'original',pc);b,pb=run(tokens,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,tokens=tokens,output=a))
 for pc in ['Single24','Double53','Extended64']:
  a,pa=ctor('original',pc);b,pb=ctor('v14',pc);assert a==b;pins.update(pa);pins.update(pb);constructors.append(dict(precision=pc,output=a))
 for a,b in pins.items():assert ORIGINAL.range(a,len(b))==V14.range(a,len(b))==b,hex(a)
 doc=dict(scope=__doc__,rows=rows,constructors=constructors,executions=2*(len(rows)+len(constructors)),
  intercepted_or_required_entries=['0x463fa0','0x48bb33','0x48be0a','0x856370','0x6f7810','0x473df0'],
  instructions=[dict(va=hex(a),bytes=b.hex())for a,b in sorted(pins.items())])
 p=HERE/'native-faction-properties.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps(dict(rows=len(rows),constructors=len(constructors),pins=len(pins),sha256=hashlib.sha256(p.read_bytes()).hexdigest())))
if __name__=='__main__':main()
