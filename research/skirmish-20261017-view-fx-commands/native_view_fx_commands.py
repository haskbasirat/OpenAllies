"""Native43B870/429020 intrusive VWorld command append, plus bounded constructor stores.
Prior lists, FXL virtualBC and retained-counter inputs are explicit fixture state.
Positive FXL retention executes native45A940; runtime adapter currently binds only
constructor-proven null FXL. No update/render or whole VWorld initialization claim.
"""
from pathlib import Path
import sys,json,hashlib,itertools
HERE=Path(__file__).resolve().parent;ROOT=HERE.parents[1]
sys.path.insert(0,str(ROOT/'research/phase8-ai'));import native_machine as nm
from unicorn import UC_HOOK_MEM_WRITE
from unicorn.x86_const import UC_X86_REG_ESI,UC_X86_REG_EBX,UC_X86_REG_ECX,UC_X86_REG_FPCW
class Required(Exception):pass

def machine(image,pc):
 m=nm.Machine(pc)
 m.u.reg_write(UC_X86_REG_FPCW,nm.PRECISIONS[pc])
 if image=='v14':
  old=(nm.original.image_size+0xfff)&~0xfff;new=(nm.modified.image_size+0xfff)&~0xfff
  if new>old:m.u.mem_map(nm.original.base+old,new-old)
  for vs,rva,raw,off in nm.modified.sections:m.u.mem_write(nm.modified.base+rva,nm.modified.b[off:off+raw])
 return m

def run(c,image,pc):
 m=machine(image,pc);world=m.alloc(0x110);nodes=[m.alloc(0x6c) for _ in range(c['prior'])];new=m.alloc(0x6c)
 names={p:i for i,p in enumerate(nodes)};names[new]='new';names[0]=None
 for i,p in enumerate(nodes):m.w(p+0x68,nodes[i+1] if i+1<len(nodes) else 0)
 m.w(world+0xe0,nodes[0] if nodes else 0);m.w(world+0xe4,nodes[-1] if nodes else 0);m.w(world+0xe8,c['count'])
 m.w(new+0x68,0xdeadbeef);definition=m.alloc(0x1a0);vtable=m.alloc(0xc0);m.w(definition,vtable)
 m.w(vtable+0xbc,0x6a71d0 if c['definition'] in ['true','required'] else 0x6a71f0)
 m.w(definition+0x19c,9);m.w(new+0x60,0 if c['definition']=='null' else definition)
 events=[];writes={world+0xe0:'head',world+0xe4:'tail',world+0xe8:'count',new+0x68:'new_next',definition+0x19c:'retain_count'}
 for i,p in enumerate(nodes):writes[p+0x68]='next_'+str(i)
 def write(u,access,a,size,value,_):
  if a in writes:events.append([writes[a],names.get(value,value) if 'next' in writes[a] or writes[a] in ['head','tail'] else value&0xffffffff])
 m.u.hook_add(UC_HOOK_MEM_WRITE,write)
 stopped=None
 if c['null_command']:
  m.seam(0x429023,lambda mm:(_ for _ in ()).throw(Required('nonnull command at429023')))
 elif c['definition']=='required':
  m.seam(0x45a940,lambda mm:(_ for _ in ()).throw(Required('retention45A940')))
 try:m.call(0x43b870,world,[0 if c['null_command'] else new])
 except Required as e:stopped=str(e)
 output=dict(head=names[m.r(world+0xe0)],tail=names[m.r(world+0xe4)],count=m.r(world+0xe8),
  prior_next=[names[m.r(p+0x68)] for p in nodes],new_next=names.get(m.r(new+0x68),m.r(new+0x68)),retain_count=m.r(definition+0x19c),events=events,stopped=stopped)
 return output,m.visited

def constructor(image,pc):
 m=machine(image,pc);world=m.alloc(0x110);m.u.mem_write(world,b'\xcd'*0x110)
 m.u.reg_write(UC_X86_REG_ESI,world);m.u.reg_write(UC_X86_REG_EBX,0)
 #Exact store suffix only. Earlier442C37 xor ebx,ebx is pinned separately.
 m.u.emu_start(0x442cf0,0x442d0e,count=100)
 return {hex(o):m.r(world+o) for o in [0xe0,0xe4,0xe8,0xec,0xf0]},m.visited

def main():
 rows=[];ctors=[];pins={};stops={}
 for pc,prior,overflow,definition,null_command in itertools.product(nm.PRECISIONS,[0,1,3],[False,True],['null','false','true','required'],[False,True]):
  if null_command and definition!='null':continue
  c=dict(prior=prior,count=0xffffffff if overflow else prior,definition=definition,null_command=null_command)
  a,pa=run(c,'original',pc);b,pb=run(c,'v14',pc);assert a==b;pins.update(pa);pins.update(pb);rows.append(dict(precision=pc,input=c,output=a))
 for pc in nm.PRECISIONS:
  a,pa=constructor('original',pc);b,pb=constructor('v14',pc);assert a==b;pins.update(pa);pins.update(pb);ctors.append(dict(precision=pc,output=a))
 for address in [0x429023,0x45a940]:stops[hex(address)]=nm.original.range(address,3).hex()
 for va,raw in pins.items():assert nm.original.range(va,len(raw))==nm.modified.range(va,len(raw))==raw
 #The xor is source proof for the isolated suffix precondition, not an executed pin.
 xor=nm.original.range(0x442c37,2);assert xor==nm.modified.range(0x442c37,2)==bytes.fromhex('33db')
 doc=dict(schema=1,scope=__doc__,original_sha256=nm.sha(nm.original.b),v14_sha256=nm.sha(nm.modified.b),rows=rows,
  constructor_rows=ctors,native_executions=2*(len(rows)+len(ctors)),instructions=nm.instructions(pins),required_boundaries=stops,
  static_constructor_precondition=dict(va='0x442c37',bytes=xor.hex(),meaning='xor ebx,ebx before isolated suffix'))
 path=HERE/'native-view-fx-commands.json'
 if '--write' in sys.argv:
  with path.open('x') as f:json.dump(doc,f,indent=2);f.write('\n')
 else:assert json.loads(path.read_text())==doc
 print(json.dumps(dict(rows=len(rows),constructors=len(ctors),runs=doc['native_executions'],executed_pins=len(pins),sha256=nm.sha(path.read_bytes()))))
if __name__=='__main__':main()
