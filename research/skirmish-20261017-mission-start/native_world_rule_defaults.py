"""Bounded native WorldRule scalar/enum default readers and Difficulty enum_specified copy.
Actual473140/473170/4733C0/473440 leaves run. CRT integer/double conversion is
an explicit transport restricted to exact signed integral source literals;
this is not a general CRT parser proof. ASCII compare/CString assignment are
seams. The4909DE..490AC1 interval executes actual Difficulty registry copying
and enum lookup with preallocated vector storage; not a whole LoadData claim.
"""
from pathlib import Path
import sys,json,hashlib,struct
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
ORIGINAL=h.P
class Done(Exception):pass

def run(c,image,pc):
 h.P=ORIGINAL if image=='original'else h.V;m=h.Machine(pc);seams=set()
 def seam(a,f):m.seams[a]=f;seams.add(a)
 def string(s):
  p=m.alloc(len(s)*2+32);m.u.mem_write(p,s.encode('utf-16le')+b'\0\0');return p
 def text(p):
  b=bytearray()
  while True:
   ch=bytes(m.u.mem_read(p,2));p+=2
   if ch==b'\0\0':return b.decode('utf-16le')
   b+=ch
 seam(0x96cad6,lambda _:m.ret(value=int(text(m.arg()).lower()!=text(m.arg(1)).lower())))
 seam(0x96cb83,lambda _:m.ret(value=int(text(m.arg())!=text(m.arg(1)))))
 seam(0x96c995,lambda _:m.ret(value=int(text(m.arg()))))
 seam(0x96d3e8,lambda _:m.ret_float(h.d2b(float(int(text(m.arg())))),'f64'))
 d=m.alloc(0x80);out=m.alloc(4);s=m.alloc(4);m.w32(out,0xaabbccff);m.w32(s,string(c['literal']))
 if c['kind']in(0,1,2):
  addr={0:0x4733c0,1:0x473140,2:0x473170}[c['kind']]
  ok=m.call(addr,d,[out,s])&255;return {'ok':ok,'value':m.r32(out)&(255 if c['kind']==0 else 0xffffffff)},seams
 if not c.get('specified'):
  table=m.alloc(max(8,8*len(c['enum_ids'])));m.w32(d+0x34,table,len(c['enum_ids']),len(c['enum_ids']))
  for i,name in enumerate(c['enum_ids']):m.w32(table+8*i,string(name),0)
  ok=m.call(0x473440,d,[out,s])&255;return {'ok':ok,'value':m.r32(out)},seams
 manager=m.alloc(0x600);array=m.alloc(4*len(c['enum_ids']));m.w32(manager+0x164,array,len(c['enum_ids']));m.w32(0xb36530,manager)
 for i,name in enumerate(c['enum_ids']):
  v=m.alloc(0x30);m.w32(v+8,string(name),0,string('display_'+name));m.w32(array+4*i,v)
 table=m.alloc(max(8,8*len(c['enum_ids'])));m.w32(d+8,string('DifficultyLevel'));m.w32(d+0x34,table,0,len(c['enum_ids']))
 def assign(_):m.w32(m.ecx(),m.r32(m.arg()));m.ret(4,m.ecx())
 seam(0x405b80,assign)
 def stop(_):raise Done()
 seam(0x490ac1,stop)
 m.u.reg_write(h.UC_X86_REG_ESP,h.STACK);m.u.reg_write(h.UC_X86_REG_EDI,d);m.u.reg_write(h.UC_X86_REG_EBX,0)
 m.w32(h.STACK+0x10,m.r32(s));m.w32(h.STACK+0x18,string('enum_specified'))
 try:m.u.emu_start(0x4909de,h.STOP,count=100000);raise AssertionError('interval did not stop')
 except Done:pass
 copied=[text(m.r32(table+i*8))for i in range(m.r32(d+0x38))]
 return {'kind':m.r32(d+0x20),'enum_ids':copied,'value':m.r32(d+0x40)},seams

def main():
 cases=[];seams=set()
 inputs=[{'kind':0,'literal':s}for s in ['true','false','TRUE']]+[{'kind':1,'literal':s}for s in ['0','100','-1']]+[{'kind':2,'literal':s}for s in ['500','200','1000']]
 inputs +=[{'kind':3,'literal':'EXPLORED','enum_ids':['NONE','EXPLORED','FULL']},
  {'kind':3,'literal':'sun_default','enum_ids':['sun_default','sun_morning','sun_dusk','sun_night']},
  {'kind':3,'literal':'a','enum_ids':['A','a','B']},
  {'kind':3,'specified':True,'literal':'MEDIUM','enum_ids':['EASY','MEDIUM','HARD']},
  {'kind':3,'specified':True,'literal':'MEDIUM','enum_ids':['HARD','MEDIUM','MEDIUM','EASY']}]
 for pc in h.PRECISION:
  for c in inputs:
   a,s=run(c,'original',pc);b,t=run(c,'v14',pc);assert a==b;seams|=s|t;cases.append({'precision':pc,'input':c,'output':a})
 doc={'schema':1,'scope':__doc__,'producer_sha256':hashlib.sha256(Path(__file__).read_bytes()).hexdigest(),'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':cases,
  'seams_not_executed':[hex(a)for a in sorted(seams)if a<0x1000000],
  'executed_instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())if a not in seams]}
 p=HERE/'native-world-rule-defaults.json'
 if '--write'in sys.argv:p.write_text(json.dumps(doc,indent=2)+'\n')
 else:assert json.loads(p.read_text())==doc
 print(json.dumps({'rows':len(cases),'machines':len(cases)*2,'pins':len(doc['executed_instructions']),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
