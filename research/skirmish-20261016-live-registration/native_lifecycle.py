"""Actual4FAE80 prologue + inherited4FB4B7 tail and native epilogue; no writes to PE."""
from pathlib import Path
import hashlib,json,struct,sys
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-movement'));import harness as h
from unicorn import UC_HOOK_CODE
from unicorn.x86_const import *
class Stop(Exception):pass
class RequiredNull(Exception):pass
class Machine(h.Machine):
 def __init__(self,case,pc):
  super().__init__(pc);self.case=case;self.trace=[];self.ptrs={};self.rows={};self.ready=False
  self.world=self.alloc(0x400);self.resource=self.alloc(0x100);self.out=self.alloc(4)
  self.source=self.string('ActualDefinition');self.w32(0xb36534,self.world)
  self.copycell=self.alloc(4);self.w32(self.copycell,self.source)
  for row in case['records']:
   ident=row['id'];p=self.alloc(0x400);vt=self.alloc(0x100);data=self.alloc(0x200)
   self.ptrs[ident]=p;self.rows[ident]=row;self.w32(p,vt,data,row['flags']);self.w32(p+0x14,ident)
   self.w32(p+0x20,*row.get('xy',[h.f2b(3.75),h.f2b(-4.5)]))
   for off,handler in [(0xe8,self.init),(0xec,self.post),(0xb0,self.obj)]:
    stub=self.stub_next;self.stub_next+=16;self.w32(vt+off,stub);self.seams[stub]=handler
  self.seams.update({0x542a00:self.editor,0x4f9510:self.register,0x4296b0:self.publish,
   0x4f5490:self.destroy,0x401b00:self.string_new,0x4b9f40:self.format,
   0x401560:self.string_copy,0x5a1040:self.warning})
  self.u.hook_add(UC_HOOK_CODE,self.observe)
 def string(self,text):
  p=self.alloc(18+len(text)*2)+16;self.w32(p-12,len(text),len(text),100000);self.u.mem_write(p,text.encode('utf-16le')+b'\0\0');return p
 def ident(self,p):return self.r32(p+0x14) if p else None
 def current(self):return self.ident(self.r32(self.out))
 def event(self,*items):
  self.trace.append(list(items));step=len(self.trace)
  if self.case.get('stop')==step:raise Stop()
  for change in self.case.get('mutations',[]):
   if change['event']==step:
    if 'out'in change:self.w32(self.out,self.ptrs.get(change['out'],0))
    if 'flags'in change:self.w32(self.ptrs[change['flags']['id']]+8,change['flags']['bits'])
 def init(self,m):
  ident=self.ident(self.ecx());assert self.arg()==0;self.event('init',ident)
  self.ret(4,value=self.rows[ident].get('init_return',1))
 def post(self,m):
  ident=self.ident(self.ecx());assert[self.arg(),self.arg(1)]==[0,1];self.event('post',ident);self.ret(8)
 def obj(self,m):
  ident=self.ident(self.ecx());self.event('object',ident);self.ret(value=self.rows[ident].get('object_return',1))
 def editor(self,m):
  assert self.ecx()==self.world+0x110 and self.arg()==self.out
  self.event('editor');self.ret(4)
 def register(self,m):
  ident=self.ident(self.arg())
  if ident is None:raise RequiredNull()
  assert self.ecx()==self.world;self.event('register',ident);self.ret(4)
 def publish(self,m):
  ident=self.ident(self.arg());assert self.arg(1)==1
  assert self.ecx()==self.r32(self.ptrs[ident]+4)+0xbc
  self.event('publish',ident);self.ret(8)
 def destroy(self,m):
  ident=self.ident(self.arg())
  if ident is None:raise RequiredNull()
  assert self.ecx()==self.world;self.event('destroy',ident);self.ret(4)
 def string_new(self,m):self.w32(self.ecx(),self.source);self.ret(4,value=self.ecx())
 def string_copy(self,m):self.w32(self.ecx(),self.r32(self.arg()));self.ret(4,value=self.ecx())
 def format(self,m):self.ret(8,value=self.copycell)
 def warning(self,m):
  assert self.arg()==self.resource and self.arg(2)==self.source and self.arg(3)==1
  x=struct.unpack('<Q',self.u.mem_read(self.esp()+4+4*4,8))[0];y=struct.unpack('<Q',self.u.mem_read(self.esp()+4+6*4,8))[0]
  self.event('warning',self.warning_ident,x,y);self.ret()
 def observe(self,u,pc,n,_):
  if pc==0x4faeb3:
   # Actual prologue already allocated locals/SEH and saved nonvolatile regs.
   # Supply precisely the inherited values from the omitted allocation/Load.
   self.frame=u.reg_read(UC_X86_REG_EBP);self.base_sp=self.esp()+4
   assert self.base_sp==self.frame-0x124
   u.reg_write(UC_X86_REG_ESP,self.base_sp);u.reg_write(UC_X86_REG_EBX,self.resource)
   self.w32(self.frame-0x18,self.source);self.w32(self.frame-0x14,1) #same saved ID as initial record+14
   self.w32(self.out,self.ptrs.get(self.case['current'],0));self.ready=True
   u.reg_write(UC_X86_REG_EIP,0x4fb4b7)
  elif not self.ready:return
  elif pc in [0x4fb4b7,0x4fb4e9,0x4fb506,0x4fb514,0x4fb526,0x4fb58f,0x4fb5f7]:
   self.event('current')
   if pc==0x4fb58f:self.warning_ident=self.current()
  elif pc in [0x4fb4c4,0x4fb4ef]:self.event('flags',self.ident(self.ecx()))
  elif pc==0x4fb4d3:self.event('clear_loading',self.ident(self.ecx()))
  elif pc==0x4fb602:self.event('clear_out')
 def run(self):
  outcome='ok';complete=False
  try:
   self.call(0x4fae80,self.world,[self.resource,self.out,0,0]);complete=True
   assert self.final_esp==h.STACK+20 and self.fpu_top()==0
   assert self.u.reg_read(UC_X86_REG_FPCW)==h.PRECISION[self.precision]
  except Stop:outcome='stopped'
  except(RequiredNull,h.NullDereference):outcome='required_null'
  return {'outcome':outcome,'trace':self.trace,'current':self.current(),
   'flags':[[r['id'],self.r32(self.ptrs[r['id']]+8)]for r in self.case['records']],
   'complete_native_epilogue':complete}
def cases():
 out=[]
 def add(name,flags=0x80000000,init=1,obj=1,current=1,mutations=None):
  c={'name':name,'current':current,'records':[{'id':1,'flags':flags,'init_return':init,'object_return':obj},{'id':2,'flags':0x81000000,'init_return':1,'object_return':1}], 'mutations':mutations or []};out.append(c);return c
 add('null',current=None)
 for flags in [0,0x80000000,0x80000080,0xffffffff]:
  for init in [0,1,128,255]:
   for obj in [0,1,128]:add(f'flags{flags:x}-init{init}-object{obj}',flags,init,obj)
 # Callback results mutate the same out slot/actual flags at exact boundaries.
 base=add('normal');normal=Machine(base,'Double53').run()['trace']
 editor=add('editor',flags=0x80000080);et=Machine(editor,'Double53').run()['trace']
 failure=add('failed',init=0);ft=Machine(failure,'Double53').run()['trace']
 for label,source,trace in [('normal',base,normal),('editor',editor,et),('failed',failure,ft)]:
  for name in ['init','post','register','object','publish','editor','warning','destroy']:
   for j,event in enumerate(trace,1):
    if event[0]==name:
     for replacement in [None,2]:out.append({**source,'name':f'{label}-{name}-out{replacement}','mutations':[{'event':j,'out':replacement}]})
  for stop in range(1,len(trace)+1):out.append({**source,'name':f'{label}-stop{stop}','stop':stop})
 # Re-read bit7 after Init/editor instead of caching the first flag word.
 add('init-enables-editor-flag',mutations=[{'event':3,'flags':{'id':1,'bits':0x80}}])
 add('editor-clears-editor-flag',flags=0x80000080,mutations=[{'event':4,'flags':{'id':1,'bits':0}}])
 return out
def main():
 inputs=cases();rows=[]
 for label,image in [('original',h.P),('v14',h.V)]:
  old=h.P;h.P=image
  try:
   for pc in h.PRECISION:
    for case in inputs:rows.append({'image':label,'precision':pc,'input':case,'output':Machine(case,pc).run()})
  finally:h.P=old
 n=len(inputs)*3
 for i in range(n):assert rows[i]['output']==rows[n+i]['output']
 d={'schema':1,'original_sha256':h.ORIGINAL_SHA,'v14_sha256':h.V14_SHA,'cases':rows,
  'instructions':[{'va':hex(a),'bytes':b}for a,b in sorted(h.VISITED.items())],
  'seams':['Earlier allocation/Load omitted via explicit redirect after real4FAE80 prologue; EBP/EDI/EBX/local source fields supplied','Init/PostCreate virtual services','editor542A00 and registration4F9510 services','Object predicate virtual+B0','definition publication4296B0','string constructor/copy and display-string lookup; native XY widening/warning stack setup and ref decrements execute','Map warning5A1040 and destruction4F5490'],
  'scope':'Only post-Load lifecycle tail plus actual inherited native frame/SEH and common native epilogue; no actualInit/PostCreate/register/warning/destructor/definition service semantics or full4FAE80 claim'}
 p=HERE/'native-lifecycle.json';p.write_text(json.dumps(d,indent=1)+'\n');print(json.dumps({'cases':len(rows),'inputs':len(inputs),'instructions':len(h.VISITED),'sha256':hashlib.sha256(p.read_bytes()).hexdigest()}))
if __name__=='__main__':main()
