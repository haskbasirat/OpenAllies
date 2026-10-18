"""Actual StartingExperience registration507FB9..507FE7 and current fixed Var.
The ctor401710,474C50 default assignment,4B3580 reset and512200 publication
execute unchanged. Native registration4B3470 is captured transport, not a second
registry. CString/allocator/lookup/log services are explicit seams. Default200
is the actual WorldRule source input; this is not a whole TGI-loader proof.
"""
from pathlib import Path
import sys,itertools,json,hashlib
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn.x86_const import *
ORIGINAL=h.P
class Stop(Exception):pass
def run(image,pc,overlay):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);seams=set()
 def seam(a,f):m.seams[a]=f;seams.add(a)
 def string(s):
  header=m.alloc(0x80);m.w32(header+12,100);p=header+16;m.u.mem_write(p,s.encode('utf-16le')+b'\0\0');return p
 name=string('StartingExperience');allocator=m.alloc(0x20);vt=m.alloc(0x20);empty=m.alloc(0x20);m.w32(allocator,vt)
 stub=m.stub_next;m.stub_next+=16;m.w32(vt+12,stub)
 seam(stub,lambda _:m.ret(value=empty));seam(0x856370,lambda _:m.ret(value=allocator));seam(0x4012c0,lambda _:m.ret(value=m.arg()))
 def literal(_):
  assert m.arg()==0xa2fbec;m.w32(m.ecx(),name);m.ret(4,m.ecx())
 seam(0x401b00,literal)
 def assign(_):m.w32(m.ecx(),m.r32(m.arg()));m.ret(4,m.ecx())
 seam(0x405b80,assign)
 retained={}
 def register(_):
  retained['index']=m.arg();retained['var']=m.arg(1);m.ret(8,retained['var'])
 seam(0x4b3470,register);seam(0x507fec,lambda _:(_ for _ in()).throw(Stop()))
 m.u.reg_write(UC_X86_REG_ESI,m.alloc(0x40));m.u.reg_write(UC_X86_REG_EDI,0xffffffff)
 try:m.call(0x507fb9)
 except Stop:pass
 var=retained['var'];assert retained['index']==0x4d
 ctor={'index':retained['index'],'value':m.r32(var),'type':m.r32(var+0x24),'flags':m.r32(var+0x28)}
 cell=m.alloc(4);m.w32(cell,var)
 def lookup(_):assert m.r32(m.arg())==name;m.ret(4,cell)
 seam(0x405370,lookup);seam(0x404f60,lambda _:m.ret())
 data=m.alloc(0x600);defs=m.alloc(4);d=m.alloc(0x60);m.w32(0xb36530,data);m.w32(data+0x118,defs,1);m.w32(defs,d)
 m.w32(d+8,name);m.w32(d+0x20,2);m.w32(d+0x40,h.f2b(200));m.call(0x474c50)
 default=m.r32(var);saved=m.alloc(0x30);m.u.mem_write(saved,bytes(m.u.mem_read(var,0x2c)));ptr=m.alloc(4);m.w32(ptr,saved);snapshot=m.alloc(0x20);m.w32(snapshot+8,ptr,1);m.w32(var,0xdeadbeef)
 m.call(0x4b3580,0xb39aa8,[snapshot]);reset=m.r32(var)
 owner=m.alloc(0x20);entry=m.alloc(12);m.w32(owner+12,entry,1);m.w32(entry,d,0,overlay);m.call(0x512200,owner)
 return {'constructor':ctor,'default':default,'reset':reset,'published':m.r32(var)},seams
def main():
 rows=[];seams=set()
 for pc,word in itertools.product(h.PRECISION,[0,h.f2b(200),h.f2b(6000),0x7f800001,0x80000000]):
  a,s=run('original',pc,word);b,t=run('v14',pc,word);assert a==b;seams|=s|t;rows.append({'precision':pc,'overlay':word,'output':a})
 doc={'scope':__doc__,'rows':rows,'executions':2*len(rows),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,
  'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),
  'seams_not_executed':[hex(a)for a in sorted(seams)if a<0x1000000],
  'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams]}
 p=HERE/'native-starting-experience.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(rows),'pins':len(doc['instructions']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
